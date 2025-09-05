// Enhanced Tauri commands for comprehensive stock data
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};
use crate::models::{
    StockInfoEnhanced, EnhancedPriceData, FundamentalData, RealTimeQuote, 
    IntradayPrice, OptionData, ComprehensiveStockData, ApiResponse, FetchRequest, Config
};
use crate::api::{SchwabClient, StockDataProvider};
use chrono::{DateTime, Utc, NaiveDate};

async fn get_database_connection() -> Result<SqlitePool, String> {
    let database_url = "sqlite:../stocks.db";
    SqlitePool::connect(database_url).await
        .map_err(|e| format!("Database connection failed: {}", e))
}

#[tauri::command]
pub async fn get_enhanced_stock_info(symbol: String) -> Result<ApiResponse<StockInfoEnhanced>, String> {
    let pool = get_database_connection().await?;
    
    let query = "
        SELECT id, symbol, company_name, exchange, sector, industry, 
               market_cap, description, employees, founded_year, 
               headquarters, website, created_at, updated_at 
        FROM stocks_enhanced 
        WHERE symbol = ?1
    ";
    
    match sqlx::query(query)
        .bind(&symbol)
        .fetch_optional(&pool).await
    {
        Ok(Some(row)) => {
            let stock = StockInfoEnhanced {
                id: row.get::<i64, _>("id"),
                symbol: row.get::<String, _>("symbol"),
                company_name: row.get::<String, _>("company_name"),
                exchange: row.try_get::<Option<String>, _>("exchange").unwrap_or(None),
                sector: row.try_get::<Option<String>, _>("sector").unwrap_or(None),
                industry: row.try_get::<Option<String>, _>("industry").unwrap_or(None),
                market_cap: row.try_get::<Option<f64>, _>("market_cap").unwrap_or(None),
                description: row.try_get::<Option<String>, _>("description").unwrap_or(None),
                employees: row.try_get::<Option<i32>, _>("employees").unwrap_or(None),
                founded_year: row.try_get::<Option<i32>, _>("founded_year").unwrap_or(None),
                headquarters: row.try_get::<Option<String>, _>("headquarters").unwrap_or(None),
                website: row.try_get::<Option<String>, _>("website").unwrap_or(None),
                created_at: chrono::Utc::now(), // TODO: Parse from database
                updated_at: chrono::Utc::now(), // TODO: Parse from database
            };
            
            Ok(ApiResponse {
                success: true,
                data: Some(stock),
                error: None,
                timestamp: Utc::now(),
            })
        }
        Ok(None) => {
            Ok(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Stock '{}' not found", symbol)),
                timestamp: Utc::now(),
            })
        }
        Err(e) => {
            Ok(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Database query error: {}", e)),
                timestamp: Utc::now(),
            })
        }
    }
}

#[tauri::command]
pub async fn get_enhanced_price_history(
    symbol: String, 
    start_date: String, 
    end_date: String
) -> Result<ApiResponse<Vec<EnhancedPriceData>>, String> {
    let pool = get_database_connection().await?;
    
    let query = "
        SELECT dpe.id, dpe.stock_id, dpe.date, dpe.open_price, dpe.high_price, 
               dpe.low_price, dpe.close_price, dpe.adjusted_close, dpe.volume, 
               dpe.average_volume, dpe.pe_ratio, dpe.pe_ratio_forward, dpe.pb_ratio, 
               dpe.ps_ratio, dpe.dividend_yield, dpe.dividend_per_share, dpe.eps, 
               dpe.eps_forward, dpe.beta, dpe.week_52_high, dpe.week_52_low, 
               dpe.week_52_change_percent, dpe.shares_outstanding, dpe.float_shares, 
               dpe.revenue_ttm, dpe.profit_margin, dpe.operating_margin, 
               dpe.return_on_equity, dpe.return_on_assets, dpe.debt_to_equity, 
               dpe.created_at
        FROM daily_prices_enhanced dpe
        JOIN stocks_enhanced se ON dpe.stock_id = se.id
        WHERE se.symbol = ?1 AND dpe.date BETWEEN ?2 AND ?3
        ORDER BY dpe.date ASC
        LIMIT 1000
    ";
    
    match sqlx::query(query)
        .bind(&symbol)
        .bind(&start_date)
        .bind(&end_date)
        .fetch_all(&pool).await
    {
        Ok(rows) => {
            let price_data: Vec<EnhancedPriceData> = rows.into_iter().map(|row| {
                EnhancedPriceData {
                    id: row.get::<i64, _>("id"),
                    stock_id: row.get::<i64, _>("stock_id"),
                    date: row.get::<String, _>("date"),
                    open_price: row.get::<f64, _>("open_price"),
                    high_price: row.get::<f64, _>("high_price"),
                    low_price: row.get::<f64, _>("low_price"),
                    close_price: row.get::<f64, _>("close_price"),
                    adjusted_close: row.try_get::<Option<f64>, _>("adjusted_close").unwrap_or(None),
                    volume: row.try_get::<Option<i64>, _>("volume").unwrap_or(None),
                    average_volume: row.try_get::<Option<i64>, _>("average_volume").unwrap_or(None),
                    
                    // Fundamental ratios
                    pe_ratio: row.try_get::<Option<f64>, _>("pe_ratio").unwrap_or(None),
                    pe_ratio_forward: row.try_get::<Option<f64>, _>("pe_ratio_forward").unwrap_or(None),
                    pb_ratio: row.try_get::<Option<f64>, _>("pb_ratio").unwrap_or(None),
                    ps_ratio: row.try_get::<Option<f64>, _>("ps_ratio").unwrap_or(None),
                    dividend_yield: row.try_get::<Option<f64>, _>("dividend_yield").unwrap_or(None),
                    dividend_per_share: row.try_get::<Option<f64>, _>("dividend_per_share").unwrap_or(None),
                    eps: row.try_get::<Option<f64>, _>("eps").unwrap_or(None),
                    eps_forward: row.try_get::<Option<f64>, _>("eps_forward").unwrap_or(None),
                    beta: row.try_get::<Option<f64>, _>("beta").unwrap_or(None),
                    
                    // 52-week data
                    week_52_high: row.try_get::<Option<f64>, _>("week_52_high").unwrap_or(None),
                    week_52_low: row.try_get::<Option<f64>, _>("week_52_low").unwrap_or(None),
                    week_52_change_percent: row.try_get::<Option<f64>, _>("week_52_change_percent").unwrap_or(None),
                    
                    // Market metrics
                    shares_outstanding: row.try_get::<Option<f64>, _>("shares_outstanding").unwrap_or(None),
                    float_shares: row.try_get::<Option<f64>, _>("float_shares").unwrap_or(None),
                    revenue_ttm: row.try_get::<Option<f64>, _>("revenue_ttm").unwrap_or(None),
                    profit_margin: row.try_get::<Option<f64>, _>("profit_margin").unwrap_or(None),
                    operating_margin: row.try_get::<Option<f64>, _>("operating_margin").unwrap_or(None),
                    return_on_equity: row.try_get::<Option<f64>, _>("return_on_equity").unwrap_or(None),
                    return_on_assets: row.try_get::<Option<f64>, _>("return_on_assets").unwrap_or(None),
                    debt_to_equity: row.try_get::<Option<f64>, _>("debt_to_equity").unwrap_or(None),
                    
                    created_at: chrono::Utc::now(), // TODO: Parse from database
                }
            }).collect();
            
            Ok(ApiResponse {
                success: true,
                data: Some(price_data),
                error: None,
                timestamp: Utc::now(),
            })
        }
        Err(e) => {
            Ok(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Price history query error: {}", e)),
                timestamp: Utc::now(),
            })
        }
    }
}

#[tauri::command]
pub async fn fetch_comprehensive_data(
    symbol: String,
    start_date: String,
    end_date: String,
    include_fundamentals: bool,
    include_real_time: bool,
    include_options: bool
) -> Result<ApiResponse<ComprehensiveStockData>, String> {
    use crate::models::Config;
    use crate::api::SchwabClient;
    use chrono::NaiveDate;
    
    let pool = get_database_connection().await?;
    
    // Get stock info from enhanced table
    let stock_info_query = "
        SELECT id, symbol, company_name, exchange, sector, industry, 
               market_cap, description, employees, founded_year, 
               headquarters, website, created_at, updated_at 
        FROM stocks_enhanced 
        WHERE symbol = ?1
    ";
    
    let stock_info = match sqlx::query(stock_info_query)
        .bind(&symbol)
        .fetch_optional(&pool).await
    {
        Ok(Some(row)) => {
            StockInfoEnhanced {
                id: row.get::<i64, _>("id"),
                symbol: row.get::<String, _>("symbol"),
                company_name: row.get::<String, _>("company_name"),
                exchange: row.try_get::<Option<String>, _>("exchange").unwrap_or(None),
                sector: row.try_get::<Option<String>, _>("sector").unwrap_or(None),
                industry: row.try_get::<Option<String>, _>("industry").unwrap_or(None),
                market_cap: row.try_get::<Option<f64>, _>("market_cap").unwrap_or(None),
                description: row.try_get::<Option<String>, _>("description").unwrap_or(None),
                employees: row.try_get::<Option<i32>, _>("employees").unwrap_or(None),
                founded_year: row.try_get::<Option<i32>, _>("founded_year").unwrap_or(None),
                headquarters: row.try_get::<Option<String>, _>("headquarters").unwrap_or(None),
                website: row.try_get::<Option<String>, _>("website").unwrap_or(None),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            }
        }
        Ok(None) => {
            return Ok(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Stock '{}' not found in enhanced database", symbol)),
                timestamp: Utc::now(),
            });
        }
        Err(e) => {
            return Ok(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Database query error: {}", e)),
                timestamp: Utc::now(),
            });
        }
    };
    
    // Get price history from enhanced table
    let price_query = "
        SELECT id, stock_id, date, open_price, high_price, low_price, close_price,
               adjusted_close, volume, average_volume, pe_ratio, pe_ratio_forward,
               pb_ratio, ps_ratio, dividend_yield, dividend_per_share, eps, eps_forward,
               beta, week_52_high, week_52_low, week_52_change_percent,
               shares_outstanding, float_shares, revenue_ttm, profit_margin,
               operating_margin, return_on_equity, return_on_assets, debt_to_equity,
               created_at
        FROM daily_prices_enhanced
        WHERE stock_id = ?1 AND date BETWEEN ?2 AND ?3
        ORDER BY date ASC
        LIMIT 1000
    ";
    
    let price_data: Vec<EnhancedPriceData> = match sqlx::query(price_query)
        .bind(stock_info.id)
        .bind(&start_date)
        .bind(&end_date)
        .fetch_all(&pool).await
    {
        Ok(rows) => {
            rows.into_iter().map(|row| {
                EnhancedPriceData {
                    id: row.get::<i64, _>("id"),
                    stock_id: row.get::<i64, _>("stock_id"),
                    date: row.get::<String, _>("date"),
                    open_price: row.get::<f64, _>("open_price"),
                    high_price: row.get::<f64, _>("high_price"),
                    low_price: row.get::<f64, _>("low_price"),
                    close_price: row.get::<f64, _>("close_price"),
                    adjusted_close: row.try_get::<Option<f64>, _>("adjusted_close").unwrap_or(None),
                    volume: row.try_get::<Option<i64>, _>("volume").unwrap_or(None),
                    average_volume: row.try_get::<Option<i64>, _>("average_volume").unwrap_or(None),
                    pe_ratio: row.try_get::<Option<f64>, _>("pe_ratio").unwrap_or(None),
                    pe_ratio_forward: row.try_get::<Option<f64>, _>("pe_ratio_forward").unwrap_or(None),
                    pb_ratio: row.try_get::<Option<f64>, _>("pb_ratio").unwrap_or(None),
                    ps_ratio: row.try_get::<Option<f64>, _>("ps_ratio").unwrap_or(None),
                    dividend_yield: row.try_get::<Option<f64>, _>("dividend_yield").unwrap_or(None),
                    dividend_per_share: row.try_get::<Option<f64>, _>("dividend_per_share").unwrap_or(None),
                    eps: row.try_get::<Option<f64>, _>("eps").unwrap_or(None),
                    eps_forward: row.try_get::<Option<f64>, _>("eps_forward").unwrap_or(None),
                    beta: row.try_get::<Option<f64>, _>("beta").unwrap_or(None),
                    week_52_high: row.try_get::<Option<f64>, _>("week_52_high").unwrap_or(None),
                    week_52_low: row.try_get::<Option<f64>, _>("week_52_low").unwrap_or(None),
                    week_52_change_percent: row.try_get::<Option<f64>, _>("week_52_change_percent").unwrap_or(None),
                    shares_outstanding: row.try_get::<Option<f64>, _>("shares_outstanding").unwrap_or(None),
                    float_shares: row.try_get::<Option<f64>, _>("float_shares").unwrap_or(None),
                    revenue_ttm: row.try_get::<Option<f64>, _>("revenue_ttm").unwrap_or(None),
                    profit_margin: row.try_get::<Option<f64>, _>("profit_margin").unwrap_or(None),
                    operating_margin: row.try_get::<Option<f64>, _>("operating_margin").unwrap_or(None),
                    return_on_equity: row.try_get::<Option<f64>, _>("return_on_equity").unwrap_or(None),
                    return_on_assets: row.try_get::<Option<f64>, _>("return_on_assets").unwrap_or(None),
                    debt_to_equity: row.try_get::<Option<f64>, _>("debt_to_equity").unwrap_or(None),
                    created_at: chrono::Utc::now(),
                }
            }).collect()
        }
        Err(_) => Vec::new(),
    };
    
    // Optionally fetch live fundamental data from Schwab API
    let fundamentals = if include_fundamentals {
        let config = Config::from_env().map_err(|e| format!("Config error: {}", e))?;
        let client = SchwabClient::new(&config).map_err(|e| format!("Client error: {}", e))?;
        
        match client.get_fundamentals(&symbol).await {
            Ok(fund_data) => Some(fund_data),
            Err(_) => None,
        }
    } else {
        None
    };
    
    // Optionally fetch real-time quote
    let real_time_quote = if include_real_time {
        match sqlx::query("
            SELECT id, stock_id, timestamp, bid_price, bid_size, ask_price, ask_size,
                   last_price, last_size, volume, change_amount, change_percent,
                   day_high, day_low
            FROM real_time_quotes 
            WHERE stock_id = ?1 
            ORDER BY timestamp DESC 
            LIMIT 1
        ")
        .bind(stock_info.id)
        .fetch_optional(&pool).await
        {
            Ok(Some(row)) => Some(RealTimeQuote {
                id: Some(row.get::<i64, _>("id")),
                stock_id: row.get::<i64, _>("stock_id"),
                symbol: symbol.clone(),
                timestamp: chrono::Utc::now(),
                bid_price: row.try_get::<Option<f64>, _>("bid_price").unwrap_or(None),
                bid_size: row.try_get::<Option<i32>, _>("bid_size").unwrap_or(None),
                ask_price: row.try_get::<Option<f64>, _>("ask_price").unwrap_or(None),
                ask_size: row.try_get::<Option<i32>, _>("ask_size").unwrap_or(None),
                last_price: row.get::<f64, _>("last_price"),
                last_size: row.try_get::<Option<i32>, _>("last_size").unwrap_or(None),
                volume: row.try_get::<Option<i64>, _>("volume").unwrap_or(None),
                change_amount: row.try_get::<Option<f64>, _>("change_amount").unwrap_or(None),
                change_percent: row.try_get::<Option<f64>, _>("change_percent").unwrap_or(None),
                day_high: row.try_get::<Option<f64>, _>("day_high").unwrap_or(None),
                day_low: row.try_get::<Option<f64>, _>("day_low").unwrap_or(None),
            }),
            _ => None,
        }
    } else {
        None
    };
    
    let comprehensive_data = ComprehensiveStockData {
        stock_info,
        price_data,
        fundamentals,
        real_time_quote,
        intraday_data: Vec::new(), // Not implemented yet
        options_data: Vec::new(),  // Not implemented yet
    };
    
    Ok(ApiResponse {
        success: true,
        data: Some(comprehensive_data),
        error: None,
        timestamp: Utc::now(),
    })
}

#[tauri::command]
pub async fn get_real_time_quote(symbol: String) -> Result<ApiResponse<RealTimeQuote>, String> {
    let pool = get_database_connection().await?;
    
    let query = "
        SELECT id, stock_id, timestamp, bid_price, bid_size, ask_price, ask_size,
               last_price, last_size, volume, change_amount, change_percent,
               day_high, day_low
        FROM real_time_quotes rtq
        JOIN stocks_enhanced se ON rtq.stock_id = se.id
        WHERE se.symbol = ?1
        ORDER BY timestamp DESC
        LIMIT 1
    ";
    
    match sqlx::query(query)
        .bind(&symbol)
        .fetch_optional(&pool).await
    {
        Ok(Some(row)) => {
            let quote = RealTimeQuote {
                id: Some(row.get::<i64, _>("id")),
                stock_id: row.get::<i64, _>("stock_id"),
                symbol: symbol.clone(),
                timestamp: chrono::Utc::now(), // TODO: Parse from database
                bid_price: row.try_get::<Option<f64>, _>("bid_price").unwrap_or(None),
                bid_size: row.try_get::<Option<i32>, _>("bid_size").unwrap_or(None),
                ask_price: row.try_get::<Option<f64>, _>("ask_price").unwrap_or(None),
                ask_size: row.try_get::<Option<i32>, _>("ask_size").unwrap_or(None),
                last_price: row.get::<f64, _>("last_price"),
                last_size: row.try_get::<Option<i32>, _>("last_size").unwrap_or(None),
                volume: row.try_get::<Option<i64>, _>("volume").unwrap_or(None),
                change_amount: row.try_get::<Option<f64>, _>("change_amount").unwrap_or(None),
                change_percent: row.try_get::<Option<f64>, _>("change_percent").unwrap_or(None),
                day_high: row.try_get::<Option<f64>, _>("day_high").unwrap_or(None),
                day_low: row.try_get::<Option<f64>, _>("day_low").unwrap_or(None),
            };
            
            Ok(ApiResponse {
                success: true,
                data: Some(quote),
                error: None,
                timestamp: Utc::now(),
            })
        }
        Ok(None) => {
            Ok(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("No real-time quote available for '{}'", symbol)),
                timestamp: Utc::now(),
            })
        }
        Err(e) => {
            Ok(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Real-time quote query error: {}", e)),
                timestamp: Utc::now(),
            })
        }
    }
}

#[tauri::command]
pub async fn get_fundamentals(symbol: String) -> Result<ApiResponse<FundamentalData>, String> {
    use crate::models::Config;
    use crate::api::SchwabClient;
    
    let config = match Config::from_env() {
        Ok(config) => config,
        Err(e) => {
            return Ok(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Configuration error: {}", e)),
                timestamp: Utc::now(),
            });
        }
    };
    
    let client = match SchwabClient::new(&config) {
        Ok(client) => client,
        Err(e) => {
            return Ok(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to create Schwab client: {}", e)),
                timestamp: Utc::now(),
            });
        }
    };
    
    match client.get_fundamentals(&symbol).await {
        Ok(fundamental_data) => {
            Ok(ApiResponse {
                success: true,
                data: Some(fundamental_data),
                error: None,
                timestamp: Utc::now(),
            })
        }
        Err(e) => {
            Ok(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to fetch fundamentals: {}", e)),
                timestamp: Utc::now(),
            })
        }
    }
}

#[tauri::command]
pub async fn get_database_migration_status() -> Result<ApiResponse<String>, String> {
    let pool = get_database_connection().await?;
    
    // Check if enhanced tables exist
    let query = "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE '%_enhanced' ORDER BY name";
    
    match sqlx::query(query).fetch_all(&pool).await {
        Ok(rows) => {
            let enhanced_tables: Vec<String> = rows.into_iter()
                .map(|row| row.get::<String, _>("name"))
                .collect();
            
            let status = if enhanced_tables.is_empty() {
                "No enhanced tables found - migration needed".to_string()
            } else {
                format!("Enhanced schema active: {} tables", enhanced_tables.len())
            };
            
            Ok(ApiResponse {
                success: true,
                data: Some(status),
                error: None,
                timestamp: Utc::now(),
            })
        }
        Err(e) => {
            Ok(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Migration status query error: {}", e)),
                timestamp: Utc::now(),
            })
        }
    }
}

#[tauri::command]
pub async fn populate_enhanced_stock_data(
    symbol: String, 
    start_date: String, 
    end_date: String,
    fetch_fundamentals: bool
) -> Result<ApiResponse<String>, String> {
    let pool = get_database_connection().await?;
    
    // Initialize Schwab API client
    let config = match Config::from_env() {
        Ok(config) => config,
        Err(e) => {
            return Ok(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Configuration error: {}", e)),
                timestamp: Utc::now(),
            });
        }
    };
    
    let client = match SchwabClient::new(&config) {
        Ok(client) => client,
        Err(e) => {
            return Ok(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to create Schwab client: {}", e)),
                timestamp: Utc::now(),
            });
        }
    };
    
    // First, ensure stock exists in enhanced table
    let stock_id = match sqlx::query("SELECT id FROM stocks_enhanced WHERE symbol = ?1")
        .bind(&symbol)
        .fetch_optional(&pool).await
    {
        Ok(Some(row)) => row.get::<i64, _>("id"),
        Ok(None) => {
            // Create enhanced stock entry (basic info for now)
            match sqlx::query(
                "INSERT INTO stocks_enhanced (symbol, company_name, created_at, updated_at) 
                 VALUES (?1, ?2, ?3, ?4) RETURNING id"
            )
            .bind(&symbol)
            .bind(format!("{} Inc.", symbol)) // Placeholder - will be updated from fundamentals
            .bind(Utc::now())
            .bind(Utc::now())
            .fetch_one(&pool).await
            {
                Ok(row) => row.get::<i64, _>("id"),
                Err(e) => {
                    return Ok(ApiResponse {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to create enhanced stock entry: {}", e)),
                        timestamp: Utc::now(),
                    });
                }
            }
        }
        Err(e) => {
            return Ok(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Database query error: {}", e)),
                timestamp: Utc::now(),
            });
        }
    };
    
    let start_date_parsed = match NaiveDate::parse_from_str(&start_date, "%Y-%m-%d") {
        Ok(date) => date,
        Err(e) => {
            return Ok(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Invalid start date: {}", e)),
                timestamp: Utc::now(),
            });
        }
    };
    
    let end_date_parsed = match NaiveDate::parse_from_str(&end_date, "%Y-%m-%d") {
        Ok(date) => date,
        Err(e) => {
            return Ok(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Invalid end date: {}", e)),
                timestamp: Utc::now(),
            });
        }
    };
    
    // Fetch price history from Schwab API
    let price_bars = match client.get_price_history(&symbol, start_date_parsed, end_date_parsed).await {
        Ok(bars) => bars,
        Err(e) => {
            return Ok(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to fetch price history: {}", e)),
                timestamp: Utc::now(),
            });
        }
    };
    
    // Fetch fundamental data if requested
    let fundamentals = if fetch_fundamentals {
        match client.get_fundamentals(&symbol).await {
            Ok(fund_data) => Some(fund_data),
            Err(_) => None, // Don't fail the entire operation if fundamentals fail
        }
    } else {
        None
    };
    
    // Insert enhanced price data
    let mut records_inserted = 0;
    for bar in price_bars {
        let date_str = chrono::DateTime::from_timestamp(bar.datetime / 1000, 0)
            .unwrap_or_default()
            .format("%Y-%m-%d")
            .to_string();
        
        // Start with basic price data
        let mut pe_ratio = None;
        let mut market_cap = None;
        let mut dividend_yield = None;
        let mut dividend_per_share = None;
        let mut eps = None;
        let mut beta = None;
        
        // Add fundamental data if available
        if let Some(ref fund_data) = fundamentals {
            pe_ratio = fund_data.pe_ratio;
            market_cap = fund_data.market_cap;
            dividend_yield = fund_data.dividend_yield;
            dividend_per_share = fund_data.dividend_per_share;
            eps = fund_data.eps;
            beta = fund_data.beta;
        }
        
        match sqlx::query(
            "INSERT OR IGNORE INTO daily_prices_enhanced (
                stock_id, date, open_price, high_price, low_price, close_price, 
                volume, pe_ratio, dividend_yield, dividend_per_share, eps, beta,
                market_cap, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)"
        )
        .bind(stock_id)
        .bind(&date_str)
        .bind(bar.open)
        .bind(bar.high)
        .bind(bar.low)
        .bind(bar.close)
        .bind(bar.volume)
        .bind(pe_ratio)
        .bind(dividend_yield)
        .bind(dividend_per_share)
        .bind(eps)
        .bind(beta)
        .bind(market_cap)
        .bind(Utc::now())
        .execute(&pool).await
        {
            Ok(result) => {
                if result.rows_affected() > 0 {
                    records_inserted += 1;
                }
            }
            Err(e) => {
                eprintln!("Failed to insert enhanced price data for {}: {}", date_str, e);
            }
        }
    }
    
    // Update stock info with fundamental data if available
    if let Some(fund_data) = fundamentals {
        let _ = sqlx::query(
            "UPDATE stocks_enhanced SET 
             market_cap = ?1, updated_at = ?2
             WHERE id = ?3"
        )
        .bind(fund_data.market_cap)
        .bind(Utc::now())
        .bind(stock_id)
        .execute(&pool).await;
    }
    
    let message = format!(
        "Successfully populated enhanced data for {} from {} to {}. Inserted {} price records with {}.",
        symbol, 
        start_date, 
        end_date, 
        records_inserted,
        if fetch_fundamentals { "fundamental data" } else { "price data only" }
    );
    
    Ok(ApiResponse {
        success: true,
        data: Some(message),
        error: None,
        timestamp: Utc::now(),
    })
}