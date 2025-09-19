use anyhow::{Result, anyhow};
use chrono::{DateTime, Duration, Utc, Local};
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};
use std::collections::HashMap;
use tokio::process::Command;
use uuid::Uuid;

use crate::tools::data_freshness_checker::{DataFreshnessChecker, FreshnessStatus, SystemFreshnessReport};
use crate::tools::date_range_calculator::DateRangeCalculator;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RefreshMode {
    Quick,    // Prices + ratios only (5-15 min)
    Standard, // + Recent financials (15-30 min)
    Full,     // + Historical data (1-2 hours)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshRequest {
    pub mode: RefreshMode,
    pub force_sources: Vec<String>,
    pub initiated_by: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshResult {
    pub session_id: String,
    pub success: bool,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_seconds: Option<i64>,
    pub sources_refreshed: Vec<String>,
    pub sources_failed: Vec<String>,
    pub total_records_processed: i64,
    pub error_message: Option<String>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshStep {
    pub name: String,
    pub data_source: String,
    pub estimated_duration_minutes: i32,
    pub command: String,
    pub dependencies: Vec<String>,
    pub priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshProgress {
    pub session_id: String,
    pub operation_type: String,
    pub start_time: DateTime<Utc>,
    pub total_steps: i32,
    pub completed_steps: i32,
    pub current_step_name: String,
    pub current_step_progress: f64,
    pub overall_progress_percent: f64,
    pub estimated_completion: Option<DateTime<Utc>>,
    pub status: String,
    pub error_details: Option<String>,
}

pub struct DataRefreshOrchestrator {
    pool: SqlitePool,
    freshness_checker: DataFreshnessChecker,
    date_calculator: DateRangeCalculator,
    refresh_steps: HashMap<RefreshMode, Vec<RefreshStep>>,
}

impl DataRefreshOrchestrator {
    pub async fn new(pool: SqlitePool) -> Result<Self> {
        let freshness_checker = DataFreshnessChecker::new(pool.clone());
        let date_calculator = DateRangeCalculator::new();
        let refresh_steps = Self::define_refresh_steps();

        Ok(Self {
            pool,
            freshness_checker,
            date_calculator,
            refresh_steps,
        })
    }

    /// Execute a data refresh operation based on the request
    pub async fn execute_refresh(&self, request: RefreshRequest) -> Result<RefreshResult> {
        let session_id = request.session_id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
        let start_time = Utc::now();

        println!("🚀 Starting data refresh session: {}", session_id);
        println!("🎯 Mode: {:?} | Initiated by: {}", request.mode, request.initiated_by);

        // Create progress tracking record
        self.create_progress_record(&session_id, &request).await?;

        let result = match self.execute_refresh_internal(session_id.clone(), request.clone()).await {
            Ok(result) => {
                self.mark_progress_complete(&session_id, true, None).await?;
                result
            }
            Err(e) => {
                self.mark_progress_complete(&session_id, false, Some(e.to_string())).await?;
                return Err(e);
            }
        };

        Ok(result)
    }

    async fn execute_refresh_internal(&self, session_id: String, request: RefreshRequest) -> Result<RefreshResult> {
        let start_time = Utc::now();
        let mut sources_refreshed = Vec::new();
        let mut sources_failed = Vec::new();
        let mut total_records_processed = 0i64;

        // 1. Check current freshness status
        println!("🔍 Checking current data freshness...");
        let freshness_report = self.freshness_checker.check_system_freshness().await?;
        self.update_progress(&session_id, 1, "Checking data freshness", 100.0).await?;

        // 2. Determine what needs to be refreshed
        let refresh_plan = self.create_refresh_plan(&request, &freshness_report).await?;
        println!("📋 Refresh plan: {} steps identified", refresh_plan.len());

        if refresh_plan.is_empty() {
            println!("✅ All data is current, no refresh needed");
            return Ok(RefreshResult {
                session_id,
                success: true,
                start_time,
                end_time: Some(Utc::now()),
                duration_seconds: Some(0),
                sources_refreshed: vec!["none (all current)".to_string()],
                sources_failed: Vec::new(),
                total_records_processed: 0,
                error_message: None,
                recommendations: vec!["All data sources are current".to_string()],
            });
        }

        // 3. Execute refresh steps in dependency order
        let total_steps = refresh_plan.len() as i32 + 2; // +2 for start/finish steps
        self.update_progress_total_steps(&session_id, total_steps).await?;

        for (step_index, step) in refresh_plan.iter().enumerate() {
            let step_number = step_index as i32 + 2; // +1 for zero-index, +1 for initial check
            println!("🔄 Step {}/{}: {}", step_number, total_steps, step.name);

            self.update_progress(&session_id, step_number, &step.name, 0.0).await?;

            match self.execute_refresh_step(step, &session_id).await {
                Ok(records) => {
                    sources_refreshed.push(step.data_source.clone());
                    total_records_processed += records;
                    self.update_refresh_status(&step.data_source, true, Some(records), None).await?;
                    self.update_progress(&session_id, step_number, &step.name, 100.0).await?;
                    println!("✅ {} completed successfully ({} records)", step.name, records);
                }
                Err(e) => {
                    sources_failed.push(step.data_source.clone());
                    self.update_refresh_status(&step.data_source, false, None, Some(e.to_string())).await?;
                    println!("❌ {} failed: {}", step.name, e);

                    // For critical steps, abort the entire refresh
                    if step.priority <= 2 {
                        return Err(anyhow!("Critical refresh step failed: {}", e));
                    }
                    // For non-critical steps, continue but log the failure
                }
            }
        }

        // 4. Final verification and cleanup
        self.update_progress(&session_id, total_steps, "Finalizing refresh", 100.0).await?;
        let final_report = self.freshness_checker.check_system_freshness().await?;

        let end_time = Utc::now();
        let duration_seconds = end_time.signed_duration_since(start_time).num_seconds();

        println!("🎉 Refresh session completed in {} seconds", duration_seconds);
        println!("✅ Refreshed: {}", sources_refreshed.join(", "));
        if !sources_failed.is_empty() {
            println!("❌ Failed: {}", sources_failed.join(", "));
        }

        Ok(RefreshResult {
            session_id,
            success: sources_failed.is_empty(),
            start_time,
            end_time: Some(end_time),
            duration_seconds: Some(duration_seconds),
            sources_refreshed,
            sources_failed,
            total_records_processed,
            error_message: None,
            recommendations: self.generate_post_refresh_recommendations(&final_report),
        })
    }

    /// Create a refresh plan based on mode and current freshness
    async fn create_refresh_plan(&self, request: &RefreshRequest, freshness_report: &SystemFreshnessReport) -> Result<Vec<RefreshStep>> {
        let available_steps = self.refresh_steps.get(&request.mode)
            .ok_or_else(|| anyhow!("Unknown refresh mode: {:?}", request.mode))?;

        let mut plan = Vec::new();

        // If force_sources is specified, only refresh those
        if !request.force_sources.is_empty() {
            for force_source in &request.force_sources {
                if let Some(step) = available_steps.iter().find(|s| s.data_source == *force_source) {
                    plan.push(step.clone());
                }
            }
        } else {
            // Otherwise, determine what needs refresh based on staleness
            for step in available_steps {
                if let Some(source_status) = freshness_report.data_sources.get(&step.data_source) {
                    if source_status.status.needs_refresh() {
                        plan.push(step.clone());
                    }
                } else {
                    // Unknown sources get refreshed by default
                    plan.push(step.clone());
                }
            }
        }

        // Sort by priority and dependencies
        plan.sort_by_key(|step| step.priority);

        Ok(plan)
    }

    /// Execute a single refresh step
    async fn execute_refresh_step(&self, step: &RefreshStep, session_id: &str) -> Result<i64> {
        let start_time = Utc::now();

        // Record the start of this refresh
        self.record_refresh_start(&step.data_source).await?;

        let records_processed = match step.data_source.as_str() {
            "daily_prices" => self.refresh_daily_prices(session_id).await?,
            "pe_ratios" => self.refresh_pe_ratios(session_id).await?,
            "ps_evs_ratios" => self.refresh_ps_evs_ratios(session_id).await?,
            "financial_statements" => self.refresh_financial_statements(session_id).await?,
            "company_metadata" => self.refresh_company_metadata(session_id).await?,
            _ => return Err(anyhow!("Unknown data source: {}", step.data_source)),
        };

        let end_time = Utc::now();
        let duration_seconds = end_time.signed_duration_since(start_time).num_seconds();

        // Record the completion
        self.record_refresh_complete(&step.data_source, records_processed, duration_seconds as i32).await?;

        Ok(records_processed)
    }

    /// Refresh daily price data using incremental updates
    async fn refresh_daily_prices(&self, _session_id: &str) -> Result<i64> {
        println!("📈 Refreshing daily price data...");

        // Execute the import-schwab-prices command with incremental mode
        let output = Command::new("cargo")
            .args(&["run", "--bin", "import-schwab-prices"])
            .output()
            .await?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Price refresh failed: {}", error));
        }

        // Parse the output to get the number of records processed
        let output_str = String::from_utf8_lossy(&output.stdout);
        let records = self.parse_records_from_output(&output_str, "price bars");

        Ok(records)
    }

    /// Refresh P/E ratios
    async fn refresh_pe_ratios(&self, _session_id: &str) -> Result<i64> {
        println!("📊 Refreshing P/E ratios...");

        let output = Command::new("cargo")
            .args(&["run", "--bin", "run_pe_calculation"])
            .output()
            .await?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("P/E ratio refresh failed: {}", error));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        let records = self.parse_records_from_output(&output_str, "P/E ratios");

        Ok(records)
    }

    /// Refresh P/S and EV/S ratios
    async fn refresh_ps_evs_ratios(&self, _session_id: &str) -> Result<i64> {
        println!("📊 Refreshing P/S and EV/S ratios...");

        let output = Command::new("cargo")
            .args(&["run", "--bin", "calculate-ratios"])
            .output()
            .await?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("P/S and EV/S ratio refresh failed: {}", error));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        let records = self.parse_records_from_output(&output_str, "ratios");

        Ok(records)
    }

    /// Refresh financial statements (future implementation)
    async fn refresh_financial_statements(&self, _session_id: &str) -> Result<i64> {
        println!("📋 Financial statement refresh not yet implemented");
        // TODO: Implement direct EDGAR API refresh
        Ok(0)
    }

    /// Refresh company metadata
    async fn refresh_company_metadata(&self, _session_id: &str) -> Result<i64> {
        println!("🏢 Refreshing company metadata...");

        // Update coverage information based on current price data
        let query = r#"
            UPDATE company_metadata
            SET
                earliest_data_date = (
                    SELECT MIN(date) FROM daily_prices WHERE stock_id = company_metadata.stock_id
                ),
                latest_data_date = (
                    SELECT MAX(date) FROM daily_prices WHERE stock_id = company_metadata.stock_id
                ),
                total_trading_days = (
                    SELECT COUNT(*) FROM daily_prices WHERE stock_id = company_metadata.stock_id
                ),
                updated_at = CURRENT_TIMESTAMP
            WHERE stock_id IN (SELECT id FROM stocks WHERE is_sp500 = 1)
        "#;

        let result = sqlx::query(query).execute(&self.pool).await?;
        Ok(result.rows_affected() as i64)
    }

    /// Parse record counts from command output
    fn parse_records_from_output(&self, output: &str, record_type: &str) -> i64 {
        // Simple parsing - look for patterns like "123 price bars" or "456 records"
        for line in output.lines() {
            if line.contains(record_type) || line.contains("records") {
                // Extract number from the line
                let words: Vec<&str> = line.split_whitespace().collect();
                for word in words {
                    if let Ok(num) = word.replace(",", "").parse::<i64>() {
                        if num > 0 {
                            return num;
                        }
                    }
                }
            }
        }
        0 // Default if parsing fails
    }

    /// Define refresh steps for each mode
    fn define_refresh_steps() -> HashMap<RefreshMode, Vec<RefreshStep>> {
        let mut steps = HashMap::new();

        // Quick mode: Just prices and critical ratios
        steps.insert(RefreshMode::Quick, vec![
            RefreshStep {
                name: "Update daily prices".to_string(),
                data_source: "daily_prices".to_string(),
                estimated_duration_minutes: 15,
                command: "import-schwab-prices".to_string(),
                dependencies: vec![],
                priority: 1,
            },
            RefreshStep {
                name: "Calculate P/E ratios".to_string(),
                data_source: "pe_ratios".to_string(),
                estimated_duration_minutes: 25,
                command: "run_pe_calculation".to_string(),
                dependencies: vec!["daily_prices".to_string()],
                priority: 2,
            },
            RefreshStep {
                name: "Update company metadata".to_string(),
                data_source: "company_metadata".to_string(),
                estimated_duration_minutes: 2,
                command: "update-company-metadata".to_string(),
                dependencies: vec!["daily_prices".to_string()],
                priority: 3,
            },
        ]);

        // Standard mode: Add P/S and EV/S ratios
        let mut standard_steps = steps[&RefreshMode::Quick].clone();
        standard_steps.push(RefreshStep {
            name: "Calculate P/S and EV/S ratios".to_string(),
            data_source: "ps_evs_ratios".to_string(),
            estimated_duration_minutes: 8,
            command: "calculate-ratios".to_string(),
            dependencies: vec!["daily_prices".to_string()],
            priority: 4,
        });
        steps.insert(RefreshMode::Standard, standard_steps);

        // Full mode: Add financial statements
        let mut full_steps = steps[&RefreshMode::Standard].clone();
        full_steps.push(RefreshStep {
            name: "Update financial statements".to_string(),
            data_source: "financial_statements".to_string(),
            estimated_duration_minutes: 45,
            command: "concurrent-edgar-extraction".to_string(),
            dependencies: vec![],
            priority: 5,
        });
        steps.insert(RefreshMode::Full, full_steps);

        steps
    }

    /// Create progress tracking record
    async fn create_progress_record(&self, session_id: &str, request: &RefreshRequest) -> Result<()> {
        let steps = self.refresh_steps.get(&request.mode)
            .ok_or_else(|| anyhow!("Unknown refresh mode"))?;

        let query = r#"
            INSERT INTO refresh_progress (
                session_id, operation_type, total_steps, current_step_name,
                initiated_by, data_sources_refreshed
            ) VALUES (?, ?, ?, ?, ?, ?)
        "#;

        let data_sources_json = serde_json::to_string(&steps.iter().map(|s| &s.data_source).collect::<Vec<_>>())?;

        sqlx::query(query)
            .bind(session_id)
            .bind(format!("{:?}", request.mode))
            .bind(steps.len() as i32 + 2) // +2 for start/finish
            .bind("Initializing")
            .bind(&request.initiated_by)
            .bind(data_sources_json)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Update progress for a specific step
    async fn update_progress(&self, session_id: &str, step_number: i32, step_name: &str, step_progress: f64) -> Result<()> {
        let query = r#"
            UPDATE refresh_progress
            SET completed_steps = ?, current_step_name = ?, current_step_progress = ?
            WHERE session_id = ?
        "#;

        sqlx::query(query)
            .bind(step_number)
            .bind(step_name)
            .bind(step_progress)
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Update total steps (used when plan is finalized)
    async fn update_progress_total_steps(&self, session_id: &str, total_steps: i32) -> Result<()> {
        let query = "UPDATE refresh_progress SET total_steps = ? WHERE session_id = ?";

        sqlx::query(query)
            .bind(total_steps)
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Mark progress as complete
    async fn mark_progress_complete(&self, session_id: &str, success: bool, error_details: Option<String>) -> Result<()> {
        let status = if success { "completed" } else { "error" };

        let query = r#"
            UPDATE refresh_progress
            SET end_time = CURRENT_TIMESTAMP, status = ?, error_details = ?
            WHERE session_id = ?
        "#;

        sqlx::query(query)
            .bind(status)
            .bind(error_details)
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Record the start of a data source refresh
    async fn record_refresh_start(&self, data_source: &str) -> Result<()> {
        let query = r#"
            UPDATE data_refresh_status
            SET last_refresh_start = CURRENT_TIMESTAMP, refresh_status = 'refreshing'
            WHERE data_source = ?
        "#;

        sqlx::query(query)
            .bind(data_source)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Record the completion of a data source refresh
    async fn record_refresh_complete(&self, data_source: &str, records_updated: i64, duration_seconds: i32) -> Result<()> {
        Ok(())
    }

    /// Update refresh status for a data source
    async fn update_refresh_status(&self, data_source: &str, success: bool, records: Option<i64>, error: Option<String>) -> Result<()> {
        let status = if success { "current" } else { "error" };

        let query = r#"
            UPDATE data_refresh_status
            SET
                last_refresh_complete = CURRENT_TIMESTAMP,
                last_successful_refresh = CASE WHEN ? THEN CURRENT_TIMESTAMP ELSE last_successful_refresh END,
                refresh_status = ?,
                records_updated = COALESCE(?, records_updated),
                error_message = ?,
                updated_at = CURRENT_TIMESTAMP
            WHERE data_source = ?
        "#;

        sqlx::query(query)
            .bind(success)
            .bind(status)
            .bind(records)
            .bind(error)
            .bind(data_source)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Generate recommendations after refresh
    fn generate_post_refresh_recommendations(&self, freshness_report: &SystemFreshnessReport) -> Vec<String> {
        let mut recommendations = Vec::new();

        if freshness_report.screening_readiness.garp_screening {
            recommendations.push("GARP screening is now ready with current data".to_string());
        }

        if freshness_report.screening_readiness.graham_screening {
            recommendations.push("Graham value screening is now ready with current data".to_string());
        }

        if freshness_report.screening_readiness.valuation_analysis {
            recommendations.push("Valuation analysis is now ready with current ratios".to_string());
        }

        if !freshness_report.screening_readiness.blocking_issues.is_empty() {
            recommendations.push("Some screening features may still have data issues".to_string());
        }

        if recommendations.is_empty() {
            recommendations.push("All major data sources have been updated".to_string());
        }

        recommendations
    }

    /// Get current progress for a session
    pub async fn get_progress(&self, session_id: &str) -> Result<Option<RefreshProgress>> {
        let query = r#"
            SELECT
                session_id, operation_type, start_time, total_steps, completed_steps,
                current_step_name, current_step_progress, status, error_details
            FROM refresh_progress
            WHERE session_id = ?
        "#;

        let row = sqlx::query(query)
            .bind(session_id)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            let completed_steps: i32 = row.get("completed_steps");
            let total_steps: i32 = row.get("total_steps");
            let overall_progress = if total_steps > 0 {
                (completed_steps as f64 / total_steps as f64) * 100.0
            } else {
                0.0
            };

            Ok(Some(RefreshProgress {
                session_id: row.get("session_id"),
                operation_type: row.get("operation_type"),
                start_time: row.get::<String, _>("start_time").parse()?,
                total_steps,
                completed_steps,
                current_step_name: row.get("current_step_name"),
                current_step_progress: row.get("current_step_progress"),
                overall_progress_percent: overall_progress,
                estimated_completion: None, // TODO: Calculate based on progress and timing
                status: row.get("status"),
                error_details: row.get("error_details"),
            }))
        } else {
            Ok(None)
        }
    }

    /// Get system freshness status
    pub async fn get_system_status(&self) -> Result<SystemFreshnessReport> {
        self.freshness_checker.check_system_freshness().await
    }
}