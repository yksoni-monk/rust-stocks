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
    // Use the centralized database helper instead of direct connection
    crate::database::helpers::get_database_connection().await
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

#[cfg(test)]
mod tests {
    use sqlx::{SqlitePool, pool::PoolOptions};
    use std::time::Duration;
    use anyhow::Result;

    /// Simple test database setup for data module tests
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
    async fn test_get_database_stats() {
        let _test_db = TestDatabase::new().await.unwrap();

        let result = super::get_database_stats().await;
        assert!(result.is_ok(), "get_database_stats should succeed");

        let stats = result.unwrap();
        assert!(stats.total_stocks > 0, "Should have stocks in database");
        assert!(stats.data_coverage_percentage >= 0.0 && stats.data_coverage_percentage <= 100.0,
                "Data coverage percentage should be between 0 and 100");
        assert!(!stats.last_update.is_empty(), "Last update should not be empty");

        println!("✅ Database stats test passed: {} stocks, {} price records, {:.1}% coverage",
                 stats.total_stocks, stats.total_price_records, stats.data_coverage_percentage);
    }
}