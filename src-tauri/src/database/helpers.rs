use sqlx::{SqlitePool, Row};
use chrono::NaiveDate;
use crate::api::alpha_vantage_client::ConvertedDailyPrice;

/// Get database connection
pub async fn get_database_connection() -> Result<SqlitePool, String> {
    let database_url = "sqlite:db/stocks.db";
    SqlitePool::connect(database_url).await
        .map_err(|e| format!("Database connection failed: {}", e))
}

/// Insert daily price data from Alpha Vantage
pub async fn insert_daily_price_data(
    pool: &SqlitePool,
    stock_id: i64,
    price_data: &ConvertedDailyPrice,
    data_source: &str,
) -> Result<(), String> {
    sqlx::query(
        "INSERT OR REPLACE INTO daily_prices (
            stock_id, date, open_price, high_price, low_price, close_price, volume,
            data_source, last_updated
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
    )
    .bind(stock_id)
    .bind(&price_data.date)
    .bind(price_data.open)
    .bind(price_data.high)
    .bind(price_data.low)
    .bind(price_data.close)
    .bind(price_data.volume)
    .bind(data_source)
    .bind(chrono::Utc::now().naive_utc())
    .execute(pool).await
    .map_err(|e| format!("Failed to insert daily price data: {}", e))?;
    
    Ok(())
}

/// Batch insert daily price data for better performance
pub async fn batch_insert_daily_prices(
    pool: &SqlitePool,
    stock_id: i64,
    price_data: &[ConvertedDailyPrice],
    data_source: &str,
) -> Result<usize, String> {
    let mut tx = pool.begin().await
        .map_err(|e| format!("Failed to start transaction: {}", e))?;
    
    let mut inserted_count = 0;
    let current_time = chrono::Utc::now().naive_utc();
    
    for price in price_data {
        let result = sqlx::query(
            "INSERT OR REPLACE INTO daily_prices (
                stock_id, date, open_price, high_price, low_price, close_price, volume,
                data_source, last_updated
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
        )
        .bind(stock_id)
        .bind(&price.date)
        .bind(price.open)
        .bind(price.high)
        .bind(price.low)
        .bind(price.close)
        .bind(price.volume)
        .bind(data_source)
        .bind(current_time)
        .execute(&mut *tx).await;
        
        match result {
            Ok(_) => inserted_count += 1,
            Err(e) => {
                eprintln!("Failed to insert price data for {}: {}", price.date, e);
            }
        }
    }
    
    tx.commit().await
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;
    
    Ok(inserted_count)
}

/// Update P/E ratio for a specific stock and date
pub async fn update_pe_ratio_for_date(
    pool: &SqlitePool,
    stock_id: i64,
    date: NaiveDate,
    pe_ratio: Option<f64>,
    eps: Option<f64>,
) -> Result<(), String> {
    sqlx::query(
        "UPDATE daily_prices 
         SET pe_ratio = ?1, eps = ?2, last_updated = ?3 
         WHERE stock_id = ?4 AND date = ?5"
    )
    .bind(pe_ratio)
    .bind(eps)
    .bind(chrono::Utc::now().naive_utc())
    .bind(stock_id)
    .bind(date)
    .execute(pool).await
    .map_err(|e| format!("Failed to update P/E ratio: {}", e))?;
    
    Ok(())
}

/// Batch update P/E ratios for multiple dates
pub async fn batch_update_pe_ratios(
    pool: &SqlitePool,
    stock_id: i64,
    pe_data: &[(NaiveDate, Option<f64>, Option<f64>)], // (date, pe_ratio, eps)
) -> Result<usize, String> {
    let mut tx = pool.begin().await
        .map_err(|e| format!("Failed to start transaction: {}", e))?;
    
    let mut updated_count = 0;
    let current_time = chrono::Utc::now().naive_utc();
    
    for (date, pe_ratio, eps) in pe_data {
        let result = sqlx::query(
            "UPDATE daily_prices 
             SET pe_ratio = ?1, eps = ?2, last_updated = ?3 
             WHERE stock_id = ?4 AND date = ?5"
        )
        .bind(pe_ratio)
        .bind(eps)
        .bind(current_time)
        .bind(stock_id)
        .bind(date)
        .execute(&mut *tx).await;
        
        match result {
            Ok(query_result) => {
                if query_result.rows_affected() > 0 {
                    updated_count += 1;
                }
            }
            Err(e) => {
                eprintln!("Failed to update P/E ratio for {}: {}", date, e);
            }
        }
    }
    
    tx.commit().await
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;
    
    Ok(updated_count)
}

/// Get stock ID by symbol
pub async fn get_stock_id_by_symbol(pool: &SqlitePool, symbol: &str) -> Result<Option<i64>, String> {
    let result = sqlx::query("SELECT id FROM stocks WHERE symbol = ?1")
        .bind(symbol)
        .fetch_optional(pool).await
        .map_err(|e| format!("Failed to get stock ID: {}", e))?;
    
    Ok(result.map(|row| row.get::<i64, _>("id")))
}

/// Check if stock has any price data
pub async fn has_price_data(pool: &SqlitePool, stock_id: i64) -> Result<bool, String> {
    let result = sqlx::query("SELECT COUNT(*) as count FROM daily_prices WHERE stock_id = ?1")
        .bind(stock_id)
        .fetch_one(pool).await
        .map_err(|e| format!("Failed to check price data: {}", e))?;
    
    let count: i64 = result.get("count");
    Ok(count > 0)
}

/// Get price data count for a stock
pub async fn get_price_data_count(pool: &SqlitePool, stock_id: i64) -> Result<i64, String> {
    let result = sqlx::query("SELECT COUNT(*) as count FROM daily_prices WHERE stock_id = ?1")
        .bind(stock_id)
        .fetch_one(pool).await
        .map_err(|e| format!("Failed to get price data count: {}", e))?;
    
    Ok(result.get("count"))
}

/// Get latest price data date for a stock
pub async fn get_latest_price_date(pool: &SqlitePool, stock_id: i64) -> Result<Option<NaiveDate>, String> {
    let result = sqlx::query("SELECT MAX(date) as latest_date FROM daily_prices WHERE stock_id = ?1")
        .bind(stock_id)
        .fetch_optional(pool).await
        .map_err(|e| format!("Failed to get latest price date: {}", e))?;
    
    Ok(result.and_then(|row| row.get::<Option<NaiveDate>, _>("latest_date")))
}

/// Clear all price data for a stock
pub async fn clear_price_data(pool: &SqlitePool, stock_id: i64) -> Result<u64, String> {
    let result = sqlx::query("DELETE FROM daily_prices WHERE stock_id = ?1")
        .bind(stock_id)
        .execute(pool).await
        .map_err(|e| format!("Failed to clear price data: {}", e))?;
    
    Ok(result.rows_affected())
}