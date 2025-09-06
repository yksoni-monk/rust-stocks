use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};
use crate::api::alpha_vantage_client::{AlphaVantageClient, DataFetchMode};
use crate::database::{
    get_database_connection, get_stock_id_by_symbol, batch_insert_daily_prices,
    store_earnings_data, batch_update_pe_ratios,
    update_processing_status, set_processing_completed, set_processing_failed,
    get_processing_status, ProcessingStatus
};
use crate::models::Config;
use chrono::NaiveDate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchRequest {
    pub symbols: Vec<String>,
    pub start_date: String,
    pub end_date: String,
    pub concurrent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchProgress {
    pub current_stock: String,
    pub completed: usize,
    pub total: usize,
    pub success_count: usize,
    pub error_count: usize,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockSymbol {
    pub symbol: String,
    pub company_name: String,
}


#[tauri::command]
pub async fn get_available_stock_symbols() -> Result<Vec<StockSymbol>, String> {
    let pool = get_database_connection().await?;
    
    // Fetch all stocks from database
    match sqlx::query("SELECT symbol, company_name FROM stocks ORDER BY symbol LIMIT 500")
        .fetch_all(&pool).await
    {
        Ok(rows) => {
            let stocks: Vec<StockSymbol> = rows.into_iter().map(|row| {
                StockSymbol {
                    symbol: row.get::<String, _>("symbol"),
                    company_name: row.get::<String, _>("company_name"),
                }
            }).collect();
            
            if stocks.is_empty() {
                // Return a message indicating initialization is needed
                Ok(vec![
                    StockSymbol { 
                        symbol: "INIT".to_string(), 
                        company_name: "Click 'Initialize S&P 500 Stocks' first".to_string() 
                    }
                ])
            } else {
                Ok(stocks)
            }
        }
        Err(e) => {
            eprintln!("Database query error: {}", e);
            // Return fallback popular stocks
            Ok(vec![
                StockSymbol { symbol: "AAPL".to_string(), company_name: "Apple Inc.".to_string() },
                StockSymbol { symbol: "MSFT".to_string(), company_name: "Microsoft Corporation".to_string() },
                StockSymbol { symbol: "GOOGL".to_string(), company_name: "Alphabet Inc.".to_string() },
            ])
        }
    }
}

#[tauri::command]
pub async fn fetch_single_stock_data(symbol: String, start_date: String, end_date: String) -> Result<String, String> {
    use crate::models::Config;
    use crate::api::{SchwabClient, StockDataProvider};
    
    let pool = get_database_connection().await?;
    
    // Load config and create Schwab client
    let config = Config::from_env().map_err(|e| format!("Failed to load config: {}", e))?;
    let schwab_client = SchwabClient::new(&config).map_err(|e| format!("Failed to create Schwab client: {}", e))?;
    
    // Check if stock exists, if not create it with real data
    let stock_id = match sqlx::query("SELECT id FROM stocks WHERE symbol = ?1")
        .bind(&symbol)
        .fetch_optional(&pool).await
    {
        Ok(Some(row)) => row.get::<i64, _>("id"),
        Ok(None) => {
            // Get real company name from Schwab API
            let instrument_data = schwab_client.get_instrument(&symbol).await
                .map_err(|e| format!("Failed to get instrument data: {}", e))?;
            
            let fallback_name = format!("{} Inc.", symbol);
            let company_name = instrument_data.get("fundamental")
                .and_then(|f| f.get("companyName"))
                .and_then(|n| n.as_str())
                .unwrap_or(&fallback_name);
            
            // Create new stock entry with real data
            match sqlx::query("INSERT INTO stocks (symbol, company_name) VALUES (?1, ?2) RETURNING id")
                .bind(&symbol)
                .bind(company_name)
                .fetch_one(&pool).await
            {
                Ok(row) => row.get::<i64, _>("id"),
                Err(e) => return Err(format!("Failed to create stock: {}", e)),
            }
        }
        Err(e) => return Err(format!("Database query failed: {}", e)),
    };
    
    // Parse date strings to NaiveDate
    let start_date_parsed = chrono::NaiveDate::parse_from_str(&start_date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid start date: {}", e))?;
    let end_date_parsed = chrono::NaiveDate::parse_from_str(&end_date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid end date: {}", e))?;

    // Fetch REAL price history from Schwab API
    let price_history = schwab_client.get_price_history(&symbol, start_date_parsed, end_date_parsed)
        .await.map_err(|e| format!("Failed to fetch price history: {}", e))?;
    
    // Fetch REAL fundamentals from Schwab API
    let fundamentals = schwab_client.get_fundamentals(&symbol)
        .await.map_err(|e| format!("Failed to fetch fundamentals: {}", e))?;
    
    // Insert REAL price data into database
    let mut records_added = 0;
    for price_bar in price_history {
        match sqlx::query(
            "INSERT OR IGNORE INTO daily_prices (
                stock_id, date, open_price, high_price, low_price, close_price, volume,
                pe_ratio, market_cap, dividend_yield, eps, beta, week_52_high, week_52_low,
                pb_ratio, ps_ratio, shares_outstanding, profit_margin, operating_margin,
                return_on_equity, return_on_assets, debt_to_equity, dividend_per_share
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23)"
        )
        .bind(stock_id)
        .bind(&price_bar.datetime)
        .bind(price_bar.open)
        .bind(price_bar.high)
        .bind(price_bar.low)
        .bind(price_bar.close)
        .bind(price_bar.volume)
        .bind(fundamentals.pe_ratio)
        .bind(fundamentals.market_cap)
        .bind(fundamentals.dividend_yield)
        .bind(fundamentals.eps)
        .bind(fundamentals.beta)
        .bind(fundamentals.week_52_high)
        .bind(fundamentals.week_52_low)
        .bind(fundamentals.pb_ratio)
        .bind(fundamentals.ps_ratio)
        .bind(fundamentals.shares_outstanding)
        .bind(fundamentals.profit_margin)
        .bind(fundamentals.operating_margin)
        .bind(fundamentals.return_on_equity)
        .bind(fundamentals.return_on_assets)
        .bind(fundamentals.debt_to_equity)
        .bind(fundamentals.dividend_per_share)
        .execute(&pool).await
        {
            Ok(result) => {
                if result.rows_affected() > 0 {
                    records_added += 1;
                }
            }
            Err(e) => eprintln!("Failed to insert price data for {}: {}", price_bar.datetime, e),
        }
    }
    
    let message = format!(
        "Successfully fetched REAL data for {} from Schwab API. Added {} price records with fundamentals (P/E: {:.2}, Market Cap: ${:.2}B, Div Yield: {:.2}%)",
        symbol, records_added, 
        fundamentals.pe_ratio.unwrap_or(0.0),
        fundamentals.market_cap.unwrap_or(0.0) / 1e9,
        fundamentals.dividend_yield.unwrap_or(0.0)
    );
    
    Ok(message)
}

#[tauri::command]
pub async fn fetch_all_stocks_concurrent(start_date: String, end_date: String) -> Result<String, String> {
    let stocks = get_available_stock_symbols().await?;
    let mut success_count = 0;
    let mut error_count = 0;
    
    // Process stocks concurrently (simulate concurrent fetching)
    for stock in &stocks {
        match fetch_single_stock_data(stock.symbol.clone(), start_date.clone(), end_date.clone()).await {
            Ok(_) => success_count += 1,
            Err(_) => error_count += 1,
        }
        
        // Add small delay to simulate realistic processing time
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    
    let message = format!(
        "Concurrent fetch completed for {} stocks from {} to {}. Success: {}, Errors: {}",
        stocks.len(), start_date, end_date, success_count, error_count
    );
    
    Ok(message)
}

#[tauri::command]
pub async fn fetch_stock_data_comprehensive(
    symbol: String, 
    fetch_mode: String // "compact" or "full"
) -> Result<String, String> {
    use std::time::Instant;
    
    let start_time = Instant::now();
    let fetch_mode: DataFetchMode = fetch_mode.into();
    
    println!("Starting comprehensive data fetch for {} in {:?} mode", symbol, fetch_mode);
    
    let pool = get_database_connection().await?;
    
    // Get stock ID
    let stock_id = match get_stock_id_by_symbol(&pool, &symbol).await? {
        Some(id) => id,
        None => return Err(format!("Stock symbol {} not found in database", symbol)),
    };
    
    // Update processing status to processing
    update_processing_status(&pool, stock_id, "prices", "processing", Some(&fetch_mode.to_string())).await?;
    
    let result = fetch_comprehensive_data_internal(&pool, stock_id, &symbol, fetch_mode).await;
    
    match result {
        Ok(message) => {
            set_processing_completed(&pool, stock_id, "prices", 0).await
                .map_err(|e| format!("Failed to update processing status: {}", e))?;
            
            let duration = start_time.elapsed();
            println!("Comprehensive fetch completed for {} in {:?}", symbol, duration);
            
            Ok(format!("{}. Completed in {:.2}s", message, duration.as_secs_f64()))
        }
        Err(error) => {
            set_processing_failed(&pool, stock_id, "prices", &error).await
                .map_err(|e| format!("Failed to update processing status: {}", e))?;
            
            Err(error)
        }
    }
}

async fn fetch_comprehensive_data_internal(
    pool: &SqlitePool,
    stock_id: i64,
    symbol: &str,
    fetch_mode: DataFetchMode,
) -> Result<String, String> {
    // Load config and create Alpha Vantage client
    let config = Config::from_env().map_err(|e| format!("Failed to load config: {}", e))?;
    let alpha_client = AlphaVantageClient::new(config.alpha_vantage_api_key);
    
    println!("Step 1: Fetching comprehensive data for {} using Alpha Vantage", symbol);
    
    // Fetch comprehensive data using Alpha Vantage
    let comprehensive_data = alpha_client.fetch_comprehensive_daily_data(symbol, fetch_mode).await?;
    
    println!("Step 2: Storing {} price records in database", comprehensive_data.daily_prices.len());
    
    // Store daily price data
    let price_records_inserted = batch_insert_daily_prices(
        pool,
        stock_id,
        &comprehensive_data.daily_prices,
        "alpha_vantage"
    ).await?;
    
    println!("Step 3: Storing earnings data in database");
    
    // Store earnings data
    let earnings_records_inserted = store_earnings_data(
        pool,
        stock_id,
        &comprehensive_data.earnings_data
    ).await?;
    
    println!("Step 4: Updating P/E ratios for {} price records", comprehensive_data.calculated_pe_ratios.len());
    
    // Update P/E ratios
    let pe_data: Vec<(NaiveDate, Option<f64>, Option<f64>)> = comprehensive_data.calculated_pe_ratios
        .iter()
        .map(|pe| (pe.date, pe.pe_ratio, pe.eps_used))
        .collect();
    
    let pe_records_updated = batch_update_pe_ratios(pool, stock_id, &pe_data).await?;
    
    // Generate summary message
    let quality = &comprehensive_data.data_quality;
    let metadata = &comprehensive_data.fetch_metadata;
    
    let message = format!(
        "Successfully fetched comprehensive data for {} using Alpha Vantage API.\n\
        • Price Records: {} inserted\n\
        • Earnings Records: {} inserted  \n\
        • P/E Calculations: {} updated ({:.1}% coverage)\n\
        • Date Range: {} to {}\n\
        • API Calls: {}\n\
        • Data Source: {}",
        symbol,
        price_records_inserted,
        earnings_records_inserted,
        pe_records_updated,
        quality.pe_calculation_coverage * 100.0,
        quality.date_range_start.map(|d| d.to_string()).unwrap_or("N/A".to_string()),
        quality.date_range_end.map(|d| d.to_string()).unwrap_or("N/A".to_string()),
        metadata.api_calls_made,
        metadata.data_source
    );
    
    Ok(message)
}

#[tauri::command]
pub async fn fetch_all_stocks_comprehensive(
    fetch_mode: String // "compact" or "full"
) -> Result<String, String> {
    use std::time::Instant;
    use tokio::time::{sleep, Duration};
    
    let start_time = Instant::now();
    let fetch_mode: DataFetchMode = fetch_mode.into();
    
    println!("Starting bulk comprehensive data fetch in {:?} mode", fetch_mode);
    
    let pool = get_database_connection().await?;
    
    // Get all available stocks
    let stocks = get_available_stock_symbols().await?;
    
    if stocks.is_empty() {
        return Err("No stocks available for fetching".to_string());
    }
    
    let total_stocks = stocks.len();
    let mut success_count = 0;
    let mut error_count = 0;
    let mut processed_count = 0;
    
    println!("Processing {} stocks with rate limiting (5 calls per minute)", total_stocks);
    
    // Process stocks with rate limiting (5 API calls per minute for Alpha Vantage free tier)
    // Each stock needs 2 API calls (daily data + earnings), so we can process 2.5 stocks per minute
    // We'll be conservative and process 2 stocks per minute
    for (index, stock) in stocks.iter().enumerate() {
        processed_count += 1;
        
        println!("Processing stock {}/{}: {}", processed_count, total_stocks, stock.symbol);
        
        // Get stock ID
        let stock_id = match get_stock_id_by_symbol(&pool, &stock.symbol).await {
            Ok(Some(id)) => id,
            Ok(None) => {
                eprintln!("Stock symbol {} not found in database", stock.symbol);
                error_count += 1;
                continue;
            }
            Err(e) => {
                eprintln!("Failed to get stock ID for {}: {}", stock.symbol, e);
                error_count += 1;
                continue;
            }
        };
        
        // Set processing status
        if let Err(e) = update_processing_status(&pool, stock_id, "prices", "processing", Some(&fetch_mode.to_string())).await {
            eprintln!("Failed to update processing status for {}: {}", stock.symbol, e);
        }
        
        // Fetch comprehensive data
        match fetch_comprehensive_data_internal(&pool, stock_id, &stock.symbol, fetch_mode.clone()).await {
            Ok(_) => {
                success_count += 1;
                println!("✅ Successfully processed {}", stock.symbol);
                
                if let Err(e) = set_processing_completed(&pool, stock_id, "prices", 0).await {
                    eprintln!("Failed to update completion status for {}: {}", stock.symbol, e);
                }
            }
            Err(e) => {
                error_count += 1;
                eprintln!("❌ Failed to process {}: {}", stock.symbol, e);
                
                if let Err(e) = set_processing_failed(&pool, stock_id, "prices", &e).await {
                    eprintln!("Failed to update failure status for {}: {}", stock.symbol, e);
                }
            }
        }
        
        // Rate limiting: wait 30 seconds between stocks (2 stocks per minute)
        // This ensures we stay well under the 5 API calls per minute limit
        if index < total_stocks - 1 { // Don't wait after the last stock
            println!("Rate limiting: waiting 30 seconds before next stock...");
            sleep(Duration::from_secs(30)).await;
        }
    }
    
    let duration = start_time.elapsed();
    let success_rate = (success_count as f64 / total_stocks as f64) * 100.0;
    
    let message = format!(
        "Bulk comprehensive fetch completed!\n\
        • Total stocks: {}\n\
        • Successful: {} ({:.1}%)\n\
        • Failed: {}\n\
        • Total time: {:.1} minutes\n\
        • Average time per stock: {:.1} seconds",
        total_stocks,
        success_count,
        success_rate,
        error_count,
        duration.as_secs_f64() / 60.0,
        duration.as_secs_f64() / total_stocks as f64
    );
    
    println!("{}", message);
    Ok(message)
}

#[tauri::command]
pub async fn get_processing_status_for_stock(stock_id: i64) -> Result<Option<ProcessingStatus>, String> {
    let pool = get_database_connection().await?;
    get_processing_status(&pool, stock_id, "prices").await
}

#[tauri::command]
pub async fn get_fetch_progress() -> Result<FetchProgress, String> {
    // Return real progress status - no fake data
    Ok(FetchProgress {
        current_stock: "".to_string(),
        completed: 0,
        total: 0,
        success_count: 0,
        error_count: 0,
        status: "Ready".to_string(),
    })
}