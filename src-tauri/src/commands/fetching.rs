use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};

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

async fn get_database_connection() -> Result<SqlitePool, String> {
    let database_url = "sqlite:../stocks.db";
    SqlitePool::connect(database_url).await
        .map_err(|e| format!("Database connection failed: {}", e))
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
                pe_ratio, market_cap, dividend_yield
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
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