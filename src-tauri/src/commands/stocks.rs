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
    let database_url = "sqlite:../stocks.db";
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