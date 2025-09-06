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
            // Return dummy data as fallback
            Ok(vec![
                StockInfo {
                    id: 1,
                    symbol: "AAPL".to_string(),
                    company_name: "Apple Inc.".to_string(),
                    sector: Some("Technology".to_string()),
                },
                StockInfo {
                    id: 2,
                    symbol: "MSFT".to_string(),
                    company_name: "Microsoft Corporation".to_string(),
                    sector: Some("Technology".to_string()),
                },
                StockInfo {
                    id: 3,
                    symbol: "GOOGL".to_string(),
                    company_name: "Alphabet Inc.".to_string(),
                    sector: Some("Technology".to_string()),
                },
            ])
        }
    }
}

#[tauri::command]
pub async fn search_stocks(query: String) -> Result<Vec<StockInfo>, String> {
    let pool = get_database_connection().await?;
    
    let sql_query = "SELECT id, symbol, company_name, sector FROM stocks 
                     WHERE symbol LIKE ?1 OR company_name LIKE ?1 
                     LIMIT 50";
    
    let search_pattern = format!("%{}%", query);
    
    match sqlx::query(sql_query)
        .bind(&search_pattern)
        .fetch_all(&pool).await 
    {
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
            eprintln!("Search query error: {}", e);
            // Fallback to dummy filtered data
            let all_stocks = get_all_stocks().await?;
            let filtered = all_stocks.into_iter()
                .filter(|stock| 
                    stock.symbol.to_lowercase().contains(&query.to_lowercase()) ||
                    stock.company_name.to_lowercase().contains(&query.to_lowercase())
                )
                .collect();
            Ok(filtered)
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
            COUNT(dp.id) as data_count
        FROM stocks s
        LEFT JOIN daily_prices dp ON s.id = dp.stock_id
        GROUP BY s.id, s.symbol, s.company_name
        ORDER BY data_count DESC, s.symbol
    ";
    
    match sqlx::query(query).fetch_all(&pool).await {
        Ok(rows) => {
            let stocks: Vec<StockWithData> = rows.into_iter().map(|row| {
                let data_count = row.get::<i64, _>("data_count");
                StockWithData {
                    id: row.get::<i64, _>("id"),
                    symbol: row.get::<String, _>("symbol"),
                    company_name: row.get::<String, _>("company_name"),
                    has_data: data_count > 0,
                    data_count,
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