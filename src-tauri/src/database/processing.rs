use sqlx::{SqlitePool, Row};
use chrono::NaiveDateTime;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingStatus {
    pub id: Option<i64>,
    pub stock_id: i64,
    pub data_type: String, // 'prices', 'earnings', 'fundamentals'
    pub status: String, // 'pending', 'processing', 'completed', 'failed'
    pub fetch_mode: Option<String>, // 'compact', 'full'
    pub records_processed: i64,
    pub total_records: i64,
    pub error_message: Option<String>,
    pub started_at: Option<NaiveDateTime>,
    pub completed_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// Create or update processing status
pub async fn update_processing_status(
    pool: &SqlitePool,
    stock_id: i64,
    data_type: &str,
    status: &str,
    fetch_mode: Option<&str>,
) -> Result<(), String> {
    let current_time = chrono::Utc::now().naive_utc();
    
    // Check if record exists
    let existing = sqlx::query(
        "SELECT id FROM processing_status WHERE stock_id = ?1 AND data_type = ?2"
    )
    .bind(stock_id)
    .bind(data_type)
    .fetch_optional(pool).await
    .map_err(|e| format!("Failed to check existing processing status: {}", e))?;
    
    if let Some(_row) = existing {
        // Update existing record
        let started_at = if status == "processing" {
            Some(current_time)
        } else {
            None
        };
        
        let completed_at = if status == "completed" || status == "failed" {
            Some(current_time)
        } else {
            None
        };
        
        sqlx::query(
            "UPDATE processing_status 
             SET status = ?1, fetch_mode = ?2, started_at = COALESCE(?3, started_at), 
                 completed_at = ?4, updated_at = ?5
             WHERE stock_id = ?6 AND data_type = ?7"
        )
        .bind(status)
        .bind(fetch_mode)
        .bind(started_at)
        .bind(completed_at)
        .bind(current_time)
        .bind(stock_id)
        .bind(data_type)
        .execute(pool).await
        .map_err(|e| format!("Failed to update processing status: {}", e))?;
    } else {
        // Insert new record
        let started_at = if status == "processing" {
            Some(current_time)
        } else {
            None
        };
        
        sqlx::query(
            "INSERT INTO processing_status (
                stock_id, data_type, status, fetch_mode, records_processed, total_records,
                started_at, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
        )
        .bind(stock_id)
        .bind(data_type)
        .bind(status)
        .bind(fetch_mode)
        .bind(0)
        .bind(0)
        .bind(started_at)
        .bind(current_time)
        .bind(current_time)
        .execute(pool).await
        .map_err(|e| format!("Failed to insert processing status: {}", e))?;
    }
    
    println!("DEBUG: Updated processing status for stock_id {} data_type {} to {}", stock_id, data_type, status);
    Ok(())
}

/// Update processing progress
pub async fn update_processing_progress(
    pool: &SqlitePool,
    stock_id: i64,
    data_type: &str,
    records_processed: i64,
    total_records: i64,
) -> Result<(), String> {
    let current_time = chrono::Utc::now().naive_utc();
    
    sqlx::query(
        "UPDATE processing_status 
         SET records_processed = ?1, total_records = ?2, updated_at = ?3
         WHERE stock_id = ?4 AND data_type = ?5"
    )
    .bind(records_processed)
    .bind(total_records)
    .bind(current_time)
    .bind(stock_id)
    .bind(data_type)
    .execute(pool).await
    .map_err(|e| format!("Failed to update processing progress: {}", e))?;
    
    Ok(())
}

/// Set processing status to failed with error message
pub async fn set_processing_failed(
    pool: &SqlitePool,
    stock_id: i64,
    data_type: &str,
    error_message: &str,
) -> Result<(), String> {
    let current_time = chrono::Utc::now().naive_utc();
    
    sqlx::query(
        "UPDATE processing_status 
         SET status = 'failed', error_message = ?1, completed_at = ?2, updated_at = ?3
         WHERE stock_id = ?4 AND data_type = ?5"
    )
    .bind(error_message)
    .bind(current_time)
    .bind(current_time)
    .bind(stock_id)
    .bind(data_type)
    .execute(pool).await
    .map_err(|e| format!("Failed to set processing failed: {}", e))?;
    
    Ok(())
}

/// Set processing status to completed
pub async fn set_processing_completed(
    pool: &SqlitePool,
    stock_id: i64,
    data_type: &str,
    records_processed: i64,
) -> Result<(), String> {
    let current_time = chrono::Utc::now().naive_utc();
    
    sqlx::query(
        "UPDATE processing_status 
         SET status = 'completed', records_processed = ?1, completed_at = ?2, updated_at = ?3
         WHERE stock_id = ?4 AND data_type = ?5"
    )
    .bind(records_processed)
    .bind(current_time)
    .bind(current_time)
    .bind(stock_id)
    .bind(data_type)
    .execute(pool).await
    .map_err(|e| format!("Failed to set processing completed: {}", e))?;
    
    Ok(())
}

/// Get processing status for a specific stock and data type
pub async fn get_processing_status(
    pool: &SqlitePool,
    stock_id: i64,
    data_type: &str,
) -> Result<Option<ProcessingStatus>, String> {
    let result = sqlx::query(
        "SELECT * FROM processing_status WHERE stock_id = ?1 AND data_type = ?2"
    )
    .bind(stock_id)
    .bind(data_type)
    .fetch_optional(pool).await
    .map_err(|e| format!("Failed to get processing status: {}", e))?;
    
    if let Some(row) = result {
        Ok(Some(ProcessingStatus {
            id: Some(row.get("id")),
            stock_id: row.get("stock_id"),
            data_type: row.get("data_type"),
            status: row.get("status"),
            fetch_mode: row.get("fetch_mode"),
            records_processed: row.get("records_processed"),
            total_records: row.get("total_records"),
            error_message: row.get("error_message"),
            started_at: row.get("started_at"),
            completed_at: row.get("completed_at"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }))
    } else {
        Ok(None)
    }
}

/// Get all processing statuses for a stock
pub async fn get_all_processing_statuses(
    pool: &SqlitePool,
    stock_id: i64,
) -> Result<Vec<ProcessingStatus>, String> {
    let results = sqlx::query(
        "SELECT * FROM processing_status WHERE stock_id = ?1 ORDER BY data_type"
    )
    .bind(stock_id)
    .fetch_all(pool).await
    .map_err(|e| format!("Failed to get processing statuses: {}", e))?;
    
    let mut statuses = Vec::new();
    for row in results {
        statuses.push(ProcessingStatus {
            id: Some(row.get("id")),
            stock_id: row.get("stock_id"),
            data_type: row.get("data_type"),
            status: row.get("status"),
            fetch_mode: row.get("fetch_mode"),
            records_processed: row.get("records_processed"),
            total_records: row.get("total_records"),
            error_message: row.get("error_message"),
            started_at: row.get("started_at"),
            completed_at: row.get("completed_at"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        });
    }
    
    Ok(statuses)
}

/// Get bulk processing overview
#[derive(Debug, Serialize)]
pub struct BulkProcessingStatus {
    pub total_stocks: i64,
    pub completed_stocks: i64,
    pub processing_stocks: i64,
    pub failed_stocks: i64,
    pub pending_stocks: i64,
    pub overall_progress: f64,
}

pub async fn get_bulk_processing_status(
    pool: &SqlitePool,
    data_type: &str,
) -> Result<BulkProcessingStatus, String> {
    let results = sqlx::query(
        "SELECT status, COUNT(*) as count FROM processing_status 
         WHERE data_type = ?1 GROUP BY status"
    )
    .bind(data_type)
    .fetch_all(pool).await
    .map_err(|e| format!("Failed to get bulk processing status: {}", e))?;
    
    let mut completed = 0i64;
    let mut processing = 0i64;
    let mut failed = 0i64;
    let mut pending = 0i64;
    
    for row in results {
        let status: String = row.get("status");
        let count: i64 = row.get("count");
        
        match status.as_str() {
            "completed" => completed = count,
            "processing" => processing = count,
            "failed" => failed = count,
            "pending" => pending = count,
            _ => {}
        }
    }
    
    let total = completed + processing + failed + pending;
    let overall_progress = if total > 0 {
        completed as f64 / total as f64
    } else {
        0.0
    };
    
    Ok(BulkProcessingStatus {
        total_stocks: total,
        completed_stocks: completed,
        processing_stocks: processing,
        failed_stocks: failed,
        pending_stocks: pending,
        overall_progress,
    })
}

/// Clear processing status for a stock
pub async fn clear_processing_status(
    pool: &SqlitePool,
    stock_id: i64,
    data_type: Option<&str>,
) -> Result<u64, String> {
    let result = if let Some(dt) = data_type {
        sqlx::query("DELETE FROM processing_status WHERE stock_id = ?1 AND data_type = ?2")
            .bind(stock_id)
            .bind(dt)
            .execute(pool).await
    } else {
        sqlx::query("DELETE FROM processing_status WHERE stock_id = ?1")
            .bind(stock_id)
            .execute(pool).await
    };
    
    match result {
        Ok(query_result) => Ok(query_result.rows_affected()),
        Err(e) => Err(format!("Failed to clear processing status: {}", e)),
    }
}