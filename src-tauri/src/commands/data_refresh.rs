// Tauri Commands for Data Refresh Operations
// Exposes data freshness checking and refresh control to the frontend

use crate::database::helpers::get_database_connection;
use crate::tools::data_freshness_checker::{DataFreshnessChecker, SystemFreshnessReport};
use crate::tools::data_refresh_orchestrator::{DataRefreshOrchestrator, RefreshRequest, RefreshMode, RefreshResult};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::Emitter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshProgress {
    pub session_id: String,
    pub operation_type: String,
    pub start_time: String,
    pub total_steps: i32,
    pub completed_steps: i32,
    pub current_step_name: Option<String>,
    pub current_step_progress: f64,
    pub overall_progress_percent: f64,
    pub estimated_completion: Option<String>,
    pub status: String,
    pub initiated_by: String,
    pub elapsed_minutes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshRequestDto {
    pub mode: String, // "quick", "standard", "full"
    pub force_sources: Option<Vec<String>>,
    pub initiated_by: Option<String>,
}

/// Get current data freshness status
#[tauri::command]
pub async fn get_data_freshness_status() -> Result<SystemFreshnessReport, String> {
    let pool = get_database_connection().await
        .map_err(|e| format!("Database connection failed: {}", e))?;

    let freshness_checker = DataFreshnessChecker::new(pool);
    freshness_checker.check_system_freshness().await
        .map_err(|e| format!("Failed to check data freshness: {}", e))
}

/// Check if specific screening features are ready
#[tauri::command]
pub async fn check_screening_readiness(feature: String) -> Result<bool, String> {
    let pool = get_database_connection().await
        .map_err(|e| format!("Database connection failed: {}", e))?;

    let freshness_checker = DataFreshnessChecker::new(pool);
    let report = freshness_checker.check_system_freshness().await
        .map_err(|e| format!("Failed to check data freshness: {}", e))?;

    match feature.as_str() {
        "garp_screening" => Ok(report.screening_readiness.garp_screening),
        "graham_screening" => Ok(report.screening_readiness.graham_screening),
        "valuation_analysis" => Ok(report.screening_readiness.valuation_analysis),
        _ => Err(format!("Unknown screening feature: {}", feature)),
    }
}

/// Start a data refresh operation
#[tauri::command]
pub async fn start_data_refresh(app_handle: tauri::AppHandle, request: RefreshRequestDto) -> Result<String, String> {
    let pool = get_database_connection().await
        .map_err(|e| format!("Database connection failed: {}", e))?;

    let orchestrator = DataRefreshOrchestrator::new(pool).await
        .map_err(|e| format!("Failed to initialize refresh orchestrator: {}", e))?;

    let refresh_mode = match request.mode.as_str() {
        "market" => RefreshMode::Market,
        "financials" => RefreshMode::Financials,
        "ratios" => RefreshMode::Ratios,
        _ => return Err(format!("Invalid refresh mode: {}", request.mode)),
    };

    let refresh_request = RefreshRequest {
        mode: refresh_mode,
        force_sources: request.force_sources.unwrap_or_default(),
        initiated_by: request.initiated_by.unwrap_or_else(|| "ui".to_string()),
        session_id: None, // Auto-generated
    };

    // Start refresh in background and return session ID
    let result = orchestrator.execute_refresh(refresh_request).await
        .map_err(|e| format!("Failed to start data refresh: {}", e))?;

    let session_id = result.session_id.clone();
    let mode = request.mode.clone();

    // Spawn background task to monitor completion and emit event
    tokio::spawn(async move {
        // Wait for completion by polling the database
        let pool = match get_database_connection().await {
            Ok(p) => p,
            Err(_) => return,
        };

        // Poll every 2 seconds for completion
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            let query = "SELECT status FROM refresh_progress WHERE session_id = ? ORDER BY start_time DESC LIMIT 1";
            if let Ok(row) = sqlx::query(query).bind(&session_id).fetch_optional(&pool).await {
                if let Some(row) = row {
                    let status: String = row.get("status");
                    if status == "completed" || status == "failed" {
                        // Emit event to frontend
                        let _ = app_handle.emit("refresh-completed", serde_json::json!({
                            "mode": mode,
                            "session_id": session_id,
                            "status": status
                        }));
                        break;
                    }
                }
            }
        }
    });

    Ok(result.session_id)
}

/// Get progress of active refresh operations
#[tauri::command]
pub async fn get_refresh_progress(session_id: String) -> Result<Option<RefreshProgress>, String> {
    let pool = get_database_connection().await
        .map_err(|e| format!("Database connection failed: {}", e))?;

    let query = r#"
        SELECT
            session_id,
            operation_type,
            start_time,
            total_steps,
            completed_steps,
            current_step_name,
            current_step_progress,
            ROUND((CAST(completed_steps AS REAL) / CAST(total_steps AS REAL)) * 100, 1) as overall_progress_percent,
            estimated_completion,
            status,
            initiated_by,
            CAST((JULIANDAY('now') - JULIANDAY(start_time)) * 24 * 60 AS INTEGER) as elapsed_minutes
        FROM refresh_progress
        WHERE session_id = ? AND status IN ('running', 'completed', 'failed', 'error')
        ORDER BY start_time DESC
        LIMIT 1
    "#;

    let rows = sqlx::query(query)
        .bind(&session_id)
        .fetch_all(&pool)
        .await
        .map_err(|e| format!("Failed to get refresh progress: {}", e))?;

    let progress = rows.into_iter().map(|row| {
        RefreshProgress {
            session_id: row.get("session_id"),
            operation_type: row.get("operation_type"),
            start_time: row.get("start_time"),
            total_steps: row.get("total_steps"),
            completed_steps: row.get("completed_steps"),
            current_step_name: row.try_get("current_step_name").ok(),
            current_step_progress: row.get("current_step_progress"),
            overall_progress_percent: row.get("overall_progress_percent"),
            estimated_completion: row.try_get("estimated_completion").ok(),
            status: row.get("status"),
            initiated_by: row.get("initiated_by"),
            elapsed_minutes: row.get("elapsed_minutes"),
        }
    }).next();

    Ok(progress)
}

/// Get last refresh results
#[tauri::command]
pub async fn get_last_refresh_result() -> Result<Option<RefreshResult>, String> {
    let pool = get_database_connection().await
        .map_err(|e| format!("Database connection failed: {}", e))?;

    let query = r#"
        SELECT
            session_id,
            operation_type,
            start_time,
            end_time,
            total_steps,
            completed_steps,
            status,
            error_details,
            initiated_by,
            data_sources_refreshed,
            total_records_processed,
            performance_metrics
        FROM refresh_progress
        WHERE status IN ('completed', 'error')
        ORDER BY start_time DESC
        LIMIT 1
    "#;

    let row = sqlx::query(query)
        .fetch_optional(&pool)
        .await
        .map_err(|e| format!("Failed to get last refresh result: {}", e))?;

    if let Some(row) = row {
        let sources_refreshed: Vec<String> = row.try_get::<String, _>("data_sources_refreshed")
            .map(|s| serde_json::from_str(&s).unwrap_or_default())
            .unwrap_or_default();

        let start_time = row.get::<String, _>("start_time");
        let end_time = row.try_get::<String, _>("end_time").ok();

        let duration_seconds = if let Some(_end_time) = &end_time {
            // Calculate duration in seconds (simplified)
            Some(60) // Placeholder
        } else {
            None
        };

        let result = RefreshResult {
            session_id: row.get("session_id"),
            success: row.get::<String, _>("status") == "completed",
            start_time: chrono::DateTime::parse_from_rfc3339(&start_time)
                .unwrap_or_else(|_| chrono::Utc::now().into())
                .with_timezone(&chrono::Utc),
            end_time: end_time.and_then(|t| chrono::DateTime::parse_from_rfc3339(&t).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc)),
            duration_seconds,
            total_records_processed: row.get("total_records_processed"),
            sources_refreshed,
            sources_failed: vec![], // Would need to parse from error_details
            recommendations: vec![], // Would need to generate based on current state
            error_message: row.try_get("error_details").ok(),
        };

        Ok(Some(result))
    } else {
        Ok(None)
    }
}

/// Cancel an active refresh operation
#[tauri::command]
pub async fn cancel_refresh_operation(session_id: String) -> Result<bool, String> {
    let pool = get_database_connection().await
        .map_err(|e| format!("Database connection failed: {}", e))?;

    let query = "UPDATE refresh_progress SET status = 'cancelled' WHERE session_id = ? AND status = 'running'";

    let result = sqlx::query(query)
        .bind(&session_id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to cancel refresh operation: {}", e))?;

    Ok(result.rows_affected() > 0)
}

/// Get estimated duration for refresh modes
#[tauri::command]
pub async fn get_refresh_duration_estimates() -> Result<std::collections::HashMap<String, i32>, String> {
    let pool = get_database_connection().await
        .map_err(|e| format!("Database connection failed: {}", e))?;

    let query = r#"
        SELECT data_source, estimated_duration_minutes
        FROM refresh_configuration
        ORDER BY refresh_priority
    "#;

    let rows = sqlx::query(query)
        .fetch_all(&pool)
        .await
        .map_err(|e| format!("Failed to get duration estimates: {}", e))?;

    let mut estimates = std::collections::HashMap::new();

    // Calculate estimates for each mode
    let mut quick_total = 0;
    let mut standard_total = 0;
    let mut full_total = 0;

    for row in rows {
        let source: String = row.get("data_source");
        let duration: i32 = row.get("estimated_duration_minutes");

        match source.as_str() {
            "daily_prices" | "pe_ratios" | "company_metadata" => {
                quick_total += duration;
                standard_total += duration;
                full_total += duration;
            }
            "ps_evs_ratios" => {
                standard_total += duration;
                full_total += duration;
            }
            "financial_statements" => {
                full_total += duration;
            }
            _ => {}
        }
    }

    estimates.insert("quick".to_string(), quick_total);
    estimates.insert("standard".to_string(), standard_total);
    estimates.insert("full".to_string(), full_total);

    Ok(estimates)
}