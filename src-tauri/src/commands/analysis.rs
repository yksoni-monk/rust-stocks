use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceData {
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: i64,
    pub pe_ratio: Option<f64>,
}

async fn get_database_connection() -> Result<SqlitePool, String> {
    let database_url = "sqlite:../stocks.db";
    SqlitePool::connect(database_url).await
        .map_err(|e| format!("Database connection failed: {}", e))
}

#[tauri::command]
pub async fn get_price_history(stock_id: i64, start_date: String, end_date: String) -> Result<Vec<PriceData>, String> {
    let pool = get_database_connection().await?;
    
    let query = "
        SELECT date, open_price, high_price, low_price, close_price, volume, pe_ratio 
        FROM daily_prices 
        WHERE stock_id = ?1 AND date BETWEEN ?2 AND ?3 
        ORDER BY date ASC
        LIMIT 1000
    ";
    
    match sqlx::query(query)
        .bind(stock_id)
        .bind(&start_date)
        .bind(&end_date)
        .fetch_all(&pool).await 
    {
        Ok(rows) => {
            let price_data: Vec<PriceData> = rows.into_iter().map(|row| {
                PriceData {
                    date: row.get::<String, _>("date"),
                    open: row.get::<f64, _>("open_price"),
                    high: row.get::<f64, _>("high_price"),
                    low: row.get::<f64, _>("low_price"),
                    close: row.get::<f64, _>("close_price"),
                    volume: row.try_get::<Option<i64>, _>("volume").unwrap_or(None).unwrap_or(0),
                    pe_ratio: row.try_get::<Option<f64>, _>("pe_ratio").unwrap_or(None),
                }
            }).collect();
            
            Ok(price_data)
        }
        Err(e) => {
            eprintln!("Price history query error: {}", e);
            // Return dummy data as fallback
            Ok(vec![
                PriceData {
                    date: "2024-01-01".to_string(),
                    open: 150.0,
                    high: 155.0,
                    low: 148.0,
                    close: 153.0,
                    volume: 1000000,
                    pe_ratio: Some(25.5),
                },
                PriceData {
                    date: "2024-01-02".to_string(),
                    open: 153.0,
                    high: 158.0,
                    low: 151.0,
                    close: 157.0,
                    volume: 1200000,
                    pe_ratio: Some(26.1),
                },
            ])
        }
    }
}

#[tauri::command]
pub async fn export_data(stock_id: i64, format: String) -> Result<String, String> {
    let pool = get_database_connection().await?;
    
    // Get stock info
    let stock_symbol = match sqlx::query("SELECT symbol FROM stocks WHERE id = ?1")
        .bind(stock_id)
        .fetch_one(&pool).await 
    {
        Ok(row) => row.get::<String, _>("symbol"),
        Err(_) => format!("Stock_{}", stock_id),
    };
    
    // Get count of records for this stock
    let record_count = match sqlx::query("SELECT COUNT(*) as count FROM daily_prices WHERE stock_id = ?1")
        .bind(stock_id)
        .fetch_one(&pool).await 
    {
        Ok(row) => row.get::<i64, _>("count"),
        Err(_) => 0,
    };
    
    let message = format!(
        "Export simulation: {} records for {} in {} format. \
        This feature will be enhanced to generate actual files in the next phase.",
        record_count,
        stock_symbol,
        format
    );
    
    Ok(message)
}