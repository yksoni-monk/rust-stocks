use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub total_stocks: usize,
    pub total_price_records: usize,
    pub data_coverage_percentage: f64,
    pub last_update: String,
}

async fn get_database_connection() -> Result<SqlitePool, String> {
    let database_url = "sqlite:../stocks.db";
    SqlitePool::connect(database_url).await
        .map_err(|e| format!("Database connection failed: {}", e))
}

#[tauri::command]
pub async fn get_database_stats() -> Result<DatabaseStats, String> {
    let pool = get_database_connection().await?;
    
    // Get total stocks count
    let stocks_count = match sqlx::query("SELECT COUNT(*) as count FROM stocks")
        .fetch_one(&pool).await 
    {
        Ok(row) => row.get::<i64, _>("count") as usize,
        Err(_) => 0,
    };
    
    // Get total price records count
    let price_records_count = match sqlx::query("SELECT COUNT(*) as count FROM daily_prices")
        .fetch_one(&pool).await 
    {
        Ok(row) => row.get::<i64, _>("count") as usize,
        Err(_) => 0,
    };
    
    // Get latest update date
    let last_update = match sqlx::query("SELECT MAX(date) as latest_date FROM daily_prices")
        .fetch_one(&pool).await 
    {
        Ok(row) => {
            match row.try_get::<Option<String>, _>("latest_date") {
                Ok(Some(date)) => date,
                _ => "No data".to_string(),
            }
        }
        Err(_) => "No data".to_string(),
    };
    
    // Calculate rough data coverage percentage
    let data_coverage_percentage = if stocks_count > 0 {
        // Get stocks with data
        let stocks_with_data = match sqlx::query(
            "SELECT COUNT(DISTINCT stock_id) as count FROM daily_prices"
        ).fetch_one(&pool).await {
            Ok(row) => row.get::<i64, _>("count") as f64,
            Err(_) => 0.0,
        };
        
        (stocks_with_data / stocks_count as f64) * 100.0
    } else {
        0.0
    };
    
    Ok(DatabaseStats {
        total_stocks: stocks_count,
        total_price_records: price_records_count,
        data_coverage_percentage,
        last_update,
    })
}

#[tauri::command]
pub async fn fetch_stock_data(stock_symbols: Vec<String>, start_date: String, end_date: String) -> Result<String, String> {
    // For now, just return a simulation message
    // In the full implementation, this would integrate with your DataCollector
    let message = format!(
        "Data fetching simulation: Would fetch {} stocks from {} to {}. \
        This feature will be integrated with the actual DataCollector in the next phase.",
        stock_symbols.len(), 
        start_date, 
        end_date
    );
    Ok(message)
}