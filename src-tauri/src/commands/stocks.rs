use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockInfo {
    pub id: i64,
    pub symbol: String,
    pub company_name: String,
    pub sector: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockWithData {
    pub id: i64,
    pub symbol: String,
    pub company_name: String,
    pub has_data: bool,
    pub data_count: i64,
}

async fn get_database_connection() -> Result<SqlitePool, String> {
    let database_url = "sqlite:db/stocks.db";
    SqlitePool::connect(database_url).await
        .map_err(|e| format!("Database connection failed: {}", e))
}

#[tauri::command]
pub async fn get_all_stocks() -> Result<Vec<StockInfo>, String> {
    let pool = get_database_connection().await?;
    
    let query = "SELECT id, symbol, company_name, sector FROM stocks";
    
    match sqlx::query(query).fetch_all(&pool).await {
        Ok(rows) => {
            let stocks: Vec<StockInfo> = rows.into_iter().map(|row| {
                StockInfo {
                    id: row.get::<i64, _>("id"),
                    symbol: row.get::<String, _>("symbol"),
                    company_name: row.get::<String, _>("company_name"),
                    sector: row.try_get::<Option<String>, _>("sector").unwrap_or(None),
                }
            }).collect();
            Ok(stocks)
        }
        Err(e) => {
            eprintln!("Database query error: {}", e);
            Err(format!("Failed to fetch all stocks: {}", e))
        }
    }
}

#[tauri::command]
pub async fn search_stocks(query: String) -> Result<Vec<StockWithData>, String> {
    let pool = get_database_connection().await?;
    
    let search_query = format!("%{}%", query);
    let sql_query = "
        SELECT 
            s.id,
            s.symbol, 
            s.company_name,
            CASE WHEN EXISTS(SELECT 1 FROM daily_prices dp WHERE dp.stock_id = s.id) THEN 1 ELSE 0 END as has_data
        FROM stocks s
        WHERE s.symbol LIKE ? OR s.company_name LIKE ?
        ORDER BY s.symbol
        LIMIT 100
    ";
    
    match sqlx::query(sql_query)
        .bind(&search_query)
        .bind(&search_query)
        .fetch_all(&pool)
        .await 
    {
        Ok(rows) => {
            let stocks: Vec<StockWithData> = rows.into_iter().map(|row| {
                let has_data = row.get::<i64, _>("has_data") > 0;
                StockWithData {
                    id: row.get::<i64, _>("id"),
                    symbol: row.get::<String, _>("symbol"),
                    company_name: row.get::<String, _>("company_name"),
                    has_data,
                    data_count: if has_data { 1 } else { 0 }, // Simplified for performance
                }
            }).collect();
            Ok(stocks)
        }
        Err(e) => {
            eprintln!("Database query error: {}", e);
            Err(format!("Failed to search stocks: {}", e))
        }
    }
}

#[tauri::command]
pub async fn get_stocks_with_data_status() -> Result<Vec<StockWithData>, String> {
    let pool = get_database_connection().await?;
    
    let query = "
        SELECT 
            s.id,
            s.symbol, 
            s.company_name,
            CASE WHEN EXISTS(SELECT 1 FROM daily_prices dp WHERE dp.stock_id = s.id) THEN 1 ELSE 0 END as has_data
        FROM stocks s
        ORDER BY has_data DESC, s.symbol
    ";
    
    match sqlx::query(query).fetch_all(&pool).await {
        Ok(rows) => {
            let stocks: Vec<StockWithData> = rows.into_iter().map(|row| {
                let has_data = row.get::<i64, _>("has_data") > 0;
                StockWithData {
                    id: row.get::<i64, _>("id"),
                    symbol: row.get::<String, _>("symbol"),
                    company_name: row.get::<String, _>("company_name"),
                    has_data,
                    data_count: if has_data { 1 } else { 0 }, // Simplified for performance
                }
            }).collect();
            Ok(stocks)
        }
        Err(e) => {
            eprintln!("Database query error: {}", e);
            Err(format!("Failed to fetch stocks with data status: {}", e))
        }
    }
}

#[tauri::command]
pub async fn get_stocks_paginated(limit: i64, offset: i64) -> Result<Vec<StockWithData>, String> {
    let pool = get_database_connection().await?;
    
    let query = "
        SELECT 
            s.id,
            s.symbol, 
            s.company_name,
            CASE WHEN EXISTS(SELECT 1 FROM daily_prices dp WHERE dp.stock_id = s.id) THEN 1 ELSE 0 END as has_data
        FROM stocks s
        ORDER BY has_data DESC, s.symbol
        LIMIT ? OFFSET ?
    ";
    
    match sqlx::query(query)
        .bind(limit)
        .bind(offset)
        .fetch_all(&pool)
        .await 
    {
        Ok(rows) => {
            let stocks: Vec<StockWithData> = rows.into_iter().map(|row| {
                let has_data = row.get::<i64, _>("has_data") > 0;
                StockWithData {
                    id: row.get::<i64, _>("id"),
                    symbol: row.get::<String, _>("symbol"),
                    company_name: row.get::<String, _>("company_name"),
                    has_data,
                    data_count: if has_data { 1 } else { 0 }, // Simplified for performance
                }
            }).collect();
            Ok(stocks)
        }
        Err(e) => {
            eprintln!("Database query error: {}", e);
            Err(format!("Failed to fetch paginated stocks: {}", e))
        }
    }
}

#[tauri::command]
pub async fn get_sp500_symbols() -> Result<Vec<String>, String> {
    let pool = get_database_connection().await?;
    
    // Try to fetch fresh data from GitHub with timeout
    let symbols = match fetch_sp500_from_github_with_timeout().await {
        Ok(fresh_symbols) => {
            // Update database with fresh data
            update_sp500_in_database(&pool, &fresh_symbols).await?;
            fresh_symbols
        }
        Err(e) => {
            println!("⚠️ Failed to fetch fresh S&P 500 data: {}", e);
            println!("📱 Using cached data from database...");
            
            // Fall back to database
            get_sp500_from_database(&pool).await?
        }
    };
    
    Ok(symbols)
}

async fn fetch_sp500_from_github_with_timeout() -> Result<Vec<String>, String> {
    let url = "https://raw.githubusercontent.com/datasets/s-and-p-500-companies/main/data/constituents.csv";
    
    // Create client with timeout
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    
    let response = client.get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch S&P 500 data: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }
    
    let csv_content = response.text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;
    
    let mut symbols = Vec::new();
    let mut lines = csv_content.lines();
    
    // Skip header
    if let Some(_header) = lines.next() {
        for line in lines {
            if line.trim().is_empty() {
                continue;
            }
            
            let fields: Vec<&str> = line.split(',').collect();
            if fields.len() >= 1 {
                let symbol = fields[0].trim().to_string();
                symbols.push(symbol);
            }
        }
    }
    
    println!("✅ Fetched {} S&P 500 symbols from GitHub", symbols.len());
    Ok(symbols)
}

async fn update_sp500_in_database(pool: &SqlitePool, symbols: &[String]) -> Result<(), String> {
    // Insert/update symbols using INSERT OR REPLACE to handle existing records
    for symbol in symbols {
        sqlx::query("INSERT OR REPLACE INTO sp500_symbols (symbol) VALUES (?)")
            .bind(symbol)
            .execute(pool)
            .await
            .map_err(|e| format!("Failed to insert symbol {}: {}", symbol, e))?;
    }
    
    // Update metadata
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("INSERT OR REPLACE INTO metadata (key, value) VALUES ('sp500_symbols_updated', ?)")
        .bind(&now)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to update metadata: {}", e))?;
    
    println!("💾 Updated database with {} S&P 500 symbols", symbols.len());
    Ok(())
}

async fn get_sp500_from_database(pool: &SqlitePool) -> Result<Vec<String>, String> {
    let rows = sqlx::query("SELECT symbol FROM sp500_symbols ORDER BY symbol")
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to fetch S&P 500 symbols from database: {}", e))?;
    
    let symbols: Vec<String> = rows.into_iter()
        .map(|row| row.get::<String, _>("symbol"))
        .collect();
    
    println!("📱 Retrieved {} S&P 500 symbols from database", symbols.len());
    Ok(symbols)
}