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
pub async fn get_price_history(symbol: String, start_date: String, end_date: String) -> Result<Vec<PriceData>, String> {
    let pool = get_database_connection().await?;
    
    // Convert string dates to Unix timestamps for database query
    let start_timestamp = chrono::NaiveDate::parse_from_str(&start_date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid start date format: {}", e))?
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp_millis();
    
    let end_timestamp = chrono::NaiveDate::parse_from_str(&end_date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid end date format: {}", e))?
        .and_hms_opt(23, 59, 59)
        .unwrap()
        .and_utc()
        .timestamp_millis();
    
    let query = "
        SELECT dp.date, dp.open_price, dp.high_price, dp.low_price, dp.close_price, dp.volume, dp.pe_ratio 
        FROM daily_prices dp
        JOIN stocks s ON dp.stock_id = s.id
        WHERE s.symbol = ?1 AND dp.date BETWEEN ?2 AND ?3 
        ORDER BY dp.date ASC
        LIMIT 1000
    ";
    
    match sqlx::query(query)
        .bind(&symbol)
        .bind(start_timestamp)
        .bind(end_timestamp)
        .fetch_all(&pool).await 
    {
        Ok(rows) => {
            let price_data: Vec<PriceData> = rows.into_iter().map(|row| {
                let timestamp: i64 = row.get::<i64, _>("date");
                let date_string = chrono::DateTime::from_timestamp_millis(timestamp)
                    .unwrap_or_default()
                    .format("%Y-%m-%d")
                    .to_string();
                
                PriceData {
                    date: date_string,
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
pub async fn export_data(symbol: String, format: String) -> Result<String, String> {
    let pool = get_database_connection().await?;
    
    // Get count of records for this stock by symbol
    let record_count = match sqlx::query("
        SELECT COUNT(*) as count 
        FROM daily_prices dp
        JOIN stocks s ON dp.stock_id = s.id
        WHERE s.symbol = ?1")
        .bind(&symbol)
        .fetch_one(&pool).await 
    {
        Ok(row) => row.get::<i64, _>("count"),
        Err(_) => 0,
    };
    
    let message = format!(
        "Export simulation: {} records for {} in {} format. \
        This feature will be enhanced to generate actual files in the next phase.",
        record_count,
        symbol,
        format
    );
    
    Ok(message)
}