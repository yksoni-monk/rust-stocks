use sqlx::{SqlitePool, Row};
use chrono::NaiveDate;
use crate::api::alpha_vantage_client::AlphaVantageEarningsResponse;

/// Store earnings data in the database
pub async fn store_earnings_data(
    pool: &SqlitePool,
    stock_id: i64,
    earnings_data: &AlphaVantageEarningsResponse,
) -> Result<usize, String> {
    let mut tx = pool.begin().await
        .map_err(|e| format!("Failed to start transaction: {}", e))?;
    
    let mut inserted_count = 0;
    
    // Store quarterly earnings
    for earning in &earnings_data.quarterly_earnings {
        let fiscal_date = NaiveDate::parse_from_str(&earning.fiscal_date_ending, "%Y-%m-%d")
            .map_err(|e| format!("Failed to parse fiscal date {}: {}", earning.fiscal_date_ending, e))?;
        
        let reported_date = if !earning.reported_date.is_empty() {
            Some(NaiveDate::parse_from_str(&earning.reported_date, "%Y-%m-%d")
                .map_err(|e| format!("Failed to parse reported date {}: {}", earning.reported_date, e))?)
        } else {
            None
        };
        
        let reported_eps = earning.reported_eps.parse::<f64>()
            .map_err(|e| format!("Failed to parse reported EPS {}: {}", earning.reported_eps, e))?;
        
        let estimated_eps = if let Some(ref eps_str) = earning.estimated_eps {
            if !eps_str.is_empty() {
                Some(eps_str.parse::<f64>()
                    .map_err(|e| format!("Failed to parse estimated EPS {}: {}", eps_str, e))?)
            } else {
                None
            }
        } else {
            None
        };
        
        let surprise = if let Some(ref surprise_str) = earning.surprise {
            if !surprise_str.is_empty() {
                Some(surprise_str.parse::<f64>()
                    .map_err(|e| format!("Failed to parse surprise {}: {}", surprise_str, e))?)
            } else {
                None
            }
        } else {
            None
        };
        
        let surprise_percentage = if let Some(ref pct_str) = earning.surprise_percentage {
            if !pct_str.is_empty() {
                Some(pct_str.parse::<f64>()
                    .map_err(|e| format!("Failed to parse surprise percentage {}: {}", pct_str, e))?)
            } else {
                None
            }
        } else {
            None
        };
        
        let result = sqlx::query(
            "INSERT OR REPLACE INTO earnings_data (
                stock_id, fiscal_date_ending, reported_date, reported_eps, estimated_eps,
                surprise, surprise_percentage, report_time, earnings_type
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
        )
        .bind(stock_id)
        .bind(fiscal_date)
        .bind(reported_date)
        .bind(reported_eps)
        .bind(estimated_eps)
        .bind(surprise)
        .bind(surprise_percentage)
        .bind(&earning.report_time)
        .bind("quarterly")
        .execute(&mut *tx).await;
        
        match result {
            Ok(_) => inserted_count += 1,
            Err(e) => {
                eprintln!("Failed to insert quarterly earnings for {}: {}", fiscal_date, e);
            }
        }
    }
    
    // Store annual earnings
    for earning in &earnings_data.annual_earnings {
        let fiscal_date = NaiveDate::parse_from_str(&earning.fiscal_date_ending, "%Y-%m-%d")
            .map_err(|e| format!("Failed to parse fiscal date {}: {}", earning.fiscal_date_ending, e))?;
        
        let reported_eps = earning.reported_eps.parse::<f64>()
            .map_err(|e| format!("Failed to parse reported EPS {}: {}", earning.reported_eps, e))?;
        
        let result = sqlx::query(
            "INSERT OR REPLACE INTO earnings_data (
                stock_id, fiscal_date_ending, reported_eps, earnings_type
            ) VALUES (?1, ?2, ?3, ?4)"
        )
        .bind(stock_id)
        .bind(fiscal_date)
        .bind(reported_eps)
        .bind("annual")
        .execute(&mut *tx).await;
        
        match result {
            Ok(_) => inserted_count += 1,
            Err(e) => {
                eprintln!("Failed to insert annual earnings for {}: {}", fiscal_date, e);
            }
        }
    }
    
    tx.commit().await
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;
    
    println!("DEBUG: Stored {} earnings records for stock_id {}", inserted_count, stock_id);
    Ok(inserted_count)
}

/// Get quarterly earnings data from database for P/E calculations
pub async fn get_quarterly_earnings(
    pool: &SqlitePool,
    stock_id: i64,
) -> Result<Vec<(NaiveDate, f64)>, String> {
    let results = sqlx::query(
        "SELECT fiscal_date_ending, reported_eps 
         FROM earnings_data 
         WHERE stock_id = ?1 AND earnings_type = 'quarterly'
         ORDER BY fiscal_date_ending DESC"
    )
    .bind(stock_id)
    .fetch_all(pool).await
    .map_err(|e| format!("Failed to get quarterly earnings: {}", e))?;
    
    let mut earnings = Vec::new();
    for row in results {
        let fiscal_date: NaiveDate = row.get("fiscal_date_ending");
        let reported_eps: f64 = row.get("reported_eps");
        earnings.push((fiscal_date, reported_eps));
    }
    
    Ok(earnings)
}

/// Get EPS for a specific date (latest available before or on that date)
pub async fn get_eps_for_date(
    pool: &SqlitePool,
    stock_id: i64,
    target_date: NaiveDate,
) -> Result<Option<f64>, String> {
    let result = sqlx::query(
        "SELECT reported_eps 
         FROM earnings_data 
         WHERE stock_id = ?1 AND earnings_type = 'quarterly' AND fiscal_date_ending <= ?2
         ORDER BY fiscal_date_ending DESC
         LIMIT 1"
    )
    .bind(stock_id)
    .bind(target_date)
    .fetch_optional(pool).await
    .map_err(|e| format!("Failed to get EPS for date: {}", e))?;
    
    Ok(result.map(|row| row.get::<f64, _>("reported_eps")))
}

/// Check if we have recent earnings data for a stock
pub async fn has_recent_earnings_data(
    pool: &SqlitePool,
    stock_id: i64,
) -> Result<bool, String> {
    let result = sqlx::query(
        "SELECT COUNT(*) as count 
         FROM earnings_data 
         WHERE stock_id = ?1 AND earnings_type = 'quarterly' 
         AND created_at > datetime('now', '-30 days')"
    )
    .bind(stock_id)
    .fetch_one(pool).await
    .map_err(|e| format!("Failed to check recent earnings data: {}", e))?;
    
    let count: i64 = result.get("count");
    Ok(count > 0)
}

/// Get earnings data count for a stock
pub async fn get_earnings_data_count(
    pool: &SqlitePool,
    stock_id: i64,
) -> Result<(i64, i64), String> { // (quarterly_count, annual_count)
    let quarterly_result = sqlx::query(
        "SELECT COUNT(*) as count FROM earnings_data WHERE stock_id = ?1 AND earnings_type = 'quarterly'"
    )
    .bind(stock_id)
    .fetch_one(pool).await
    .map_err(|e| format!("Failed to get quarterly earnings count: {}", e))?;
    
    let annual_result = sqlx::query(
        "SELECT COUNT(*) as count FROM earnings_data WHERE stock_id = ?1 AND earnings_type = 'annual'"
    )
    .bind(stock_id)
    .fetch_one(pool).await
    .map_err(|e| format!("Failed to get annual earnings count: {}", e))?;
    
    let quarterly_count: i64 = quarterly_result.get("count");
    let annual_count: i64 = annual_result.get("count");
    
    Ok((quarterly_count, annual_count))
}

/// Clear earnings data for a stock
pub async fn clear_earnings_data(
    pool: &SqlitePool,
    stock_id: i64,
) -> Result<u64, String> {
    let result = sqlx::query("DELETE FROM earnings_data WHERE stock_id = ?1")
        .bind(stock_id)
        .execute(pool).await
        .map_err(|e| format!("Failed to clear earnings data: {}", e))?;
    
    Ok(result.rows_affected())
}