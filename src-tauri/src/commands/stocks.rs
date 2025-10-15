use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};
use crate::database::helpers::get_database_connection;

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

    // Read-only: Just fetch from database (sp500_symbols is a view on stocks where is_sp500 = 1)
    // S&P 500 membership is managed by init_sp500 binary, not by frontend commands
    get_sp500_from_database(&pool).await
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

#[cfg(test)]
mod tests {
    use sqlx::{SqlitePool, pool::PoolOptions};
    use std::time::Duration;
    use anyhow::Result;

    /// Simple test database setup for stocks module tests
    struct TestDatabase {
        _pool: SqlitePool,
    }

    impl TestDatabase {
        async fn new() -> Result<Self> {
            let current_dir = std::env::current_dir()?;
            let test_db_path = current_dir.join("db/test.db");

            let database_url = format!("sqlite:{}", test_db_path.to_string_lossy());

            let pool = PoolOptions::new()
                .max_connections(10)
                .min_connections(2)
                .acquire_timeout(Duration::from_secs(10))
                .idle_timeout(Some(Duration::from_secs(600)))
                .connect(&database_url).await?;

            Ok(TestDatabase { _pool: pool })
        }
    }

    #[tokio::test]
    async fn test_get_stocks_paginated() {
        let _test_db = TestDatabase::new().await.unwrap();

        let result = super::get_stocks_paginated(10, 0).await;
        assert!(result.is_ok(), "get_stocks_paginated should succeed");

        let stocks = result.unwrap();
        assert!(stocks.len() <= 10, "Should return at most 10 stocks");

        // Test pagination with offset
        let result2 = super::get_stocks_paginated(5, 5).await;
        assert!(result2.is_ok(), "get_stocks_paginated with offset should succeed");

        println!("✅ get_stocks_paginated test passed");
    }

    #[tokio::test]
    async fn test_search_stocks() {
        let _test_db = TestDatabase::new().await.unwrap();

        let result = super::search_stocks("AAPL".to_string()).await;
        assert!(result.is_ok(), "search_stocks should succeed");

        let stocks = result.unwrap();
        if !stocks.is_empty() {
            assert!(stocks[0].symbol.contains("AAPL") || stocks[0].company_name.to_lowercase().contains("apple"),
                    "Search should return relevant results");
        }

        // Test empty search
        let empty_result = super::search_stocks("NONEXISTENTSYMBOL123".to_string()).await;
        assert!(empty_result.is_ok(), "Empty search should succeed");

        println!("✅ search_stocks test passed");
    }

    #[tokio::test]
    async fn test_get_sp500_symbols() {
        let _test_db = TestDatabase::new().await.unwrap();

        let result = super::get_sp500_symbols().await;
        assert!(result.is_ok(), "get_sp500_symbols should succeed");

        let symbols = result.unwrap();
        assert!(!symbols.is_empty(), "Should return S&P 500 symbols");
        assert!(symbols.len() >= 400, "Should have at least 400 symbols (allowing for some variance)");

        // Check that symbols are properly formatted
        for symbol in symbols.iter().take(10) {
            assert!(!symbol.is_empty(), "Symbol should not be empty");
            assert!(symbol.chars().all(|c| c.is_alphanumeric() || c == '.'),
                    "Symbol should contain only alphanumeric characters and dots");
        }

        println!("✅ get_sp500_symbols test passed with {} symbols", symbols.len());
    }
}