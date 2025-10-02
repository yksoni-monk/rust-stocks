use anyhow::Result;
use chrono::{Local, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};
use std::collections::HashMap;
use ts_rs::TS;

/// Column names for the v_data_freshness_summary view
/// This prevents magic number bugs and makes the code more maintainable
mod freshness_columns {
    pub const DATA_SOURCE: &str = "data_source";
    pub const REFRESH_STATUS: &str = "refresh_status";
    pub const LATEST_DATA_DATE: &str = "latest_data_date";
    pub const LAST_SUCCESSFUL_REFRESH: &str = "last_successful_refresh";
    pub const RECORDS_UPDATED: &str = "records_updated";
    pub const STALENESS_DAYS: &str = "staleness_days";
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DataFreshnessStatus {
    pub data_source: String,
    pub status: FreshnessStatus,
    pub latest_data_date: Option<String>, // Changed to String for TS compatibility
    pub last_refresh: Option<String>, // Changed to String for TS compatibility
    pub staleness_days: Option<i64>,
    pub records_count: i64,
    pub message: String,
    pub refresh_priority: RefreshPriority,
    // Enhanced fields for better UX
    pub data_summary: DataSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DataSummary {
    pub date_range: Option<String>,
    pub stock_count: Option<i64>,
    pub data_types: Vec<String>,
    pub key_metrics: Vec<String>,
    pub completeness_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub enum FreshnessStatus {
    Current,
    Stale,
    Missing,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, TS)]
#[ts(export)]
pub enum RefreshPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SystemFreshnessReport {
    pub overall_status: FreshnessStatus,
    pub market_data: DataFreshnessStatus,
    pub financial_data: DataFreshnessStatus,
    pub calculated_ratios: DataFreshnessStatus,
    pub recommendations: Vec<RefreshRecommendation>,
    pub screening_readiness: ScreeningReadiness,
    pub last_check: String, // Changed to String for TS compatibility
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RefreshRecommendation {
    pub action: String,
    pub reason: String,
    pub estimated_duration: String,
    pub priority: RefreshPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ScreeningReadiness {
    pub valuation_analysis: bool,
    pub blocking_issues: Vec<String>,
}

pub struct DataStatusReader {
    pool: SqlitePool,
}

impl DataStatusReader {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
        }
    }

    /// Check freshness of all data sources and generate comprehensive report
    pub async fn check_system_freshness(&self) -> Result<SystemFreshnessReport> {
        // Use fast summary view instead of individual COUNT(*) queries
        let data_sources = self.check_data_sources_fast().await?;

        // Determine overall status
        let overall_status = self.determine_overall_status(&data_sources);

        // Generate recommendations
        let recommendations = self.generate_recommendations(&data_sources).await?;

        // Check screening readiness
        let screening_readiness = self.assess_screening_readiness(&data_sources).await?;

        // Extract semantic data sources directly
        let market_data = data_sources.get("daily_prices")
            .cloned()
            .unwrap_or_else(|| DataFreshnessStatus {
                data_source: "daily_prices".to_string(),
                status: FreshnessStatus::Missing,
                latest_data_date: None,
                last_refresh: None,
                staleness_days: None,
                records_count: 0,
                message: "Market data not available".to_string(),
                refresh_priority: RefreshPriority::Critical,
                data_summary: DataSummary {
                    date_range: None,
                    stock_count: None,
                    data_types: vec!["Daily Prices".to_string()],
                    key_metrics: vec!["No data available".to_string()],
                    completeness_score: None,
                },
            });

        let financial_data = data_sources.get("financial_statements")
            .cloned()
            .unwrap_or_else(|| DataFreshnessStatus {
                data_source: "financial_statements".to_string(),
                status: FreshnessStatus::Missing,
                latest_data_date: None,
                last_refresh: None,
                staleness_days: None,
                records_count: 0,
                message: "Financial data not available".to_string(),
                refresh_priority: RefreshPriority::Critical,
                data_summary: DataSummary {
                    date_range: None,
                    stock_count: None,
                    data_types: vec!["Financial Statements".to_string()],
                    key_metrics: vec!["No data available".to_string()],
                    completeness_score: None,
                },
            });

        let calculated_ratios = data_sources.get("ps_evs_ratios")
            .cloned()
            .unwrap_or_else(|| DataFreshnessStatus {
                data_source: "ps_evs_ratios".to_string(),
                status: FreshnessStatus::Missing,
                latest_data_date: None,
                last_refresh: None,
                staleness_days: None,
                records_count: 0,
                message: "Ratio data not available".to_string(),
                refresh_priority: RefreshPriority::Critical,
                data_summary: DataSummary {
                    date_range: None,
                    stock_count: None,
                    data_types: vec!["Financial Ratios".to_string()],
                    key_metrics: vec!["No data available".to_string()],
                    completeness_score: None,
                },
            });

        Ok(SystemFreshnessReport {
            overall_status,
            market_data,
            financial_data,
            calculated_ratios,
            recommendations,
            screening_readiness,
            last_check: Utc::now().to_rfc3339(),
        })
    }

    /// Direct database queries for accurate freshness checking
    async fn check_data_sources_fast(&self) -> Result<HashMap<String, DataFreshnessStatus>> {
        let mut data_sources = HashMap::new();

        // Check daily_prices directly
        let daily_prices_status = self.check_daily_prices_direct().await?;
        data_sources.insert("daily_prices".to_string(), daily_prices_status);

        // Check financial_statements directly
        let financial_status = self.check_financial_statements_direct().await?;
        data_sources.insert("financial_statements".to_string(), financial_status);


        // Check cash_flow_statements directly
        let cash_flow_status = self.check_cash_flow_statements_direct().await?;
        data_sources.insert("cash_flow_statements".to_string(), cash_flow_status);


        Ok(data_sources)
    }

    /// Check daily_prices table directly
    async fn check_daily_prices_direct(&self) -> Result<DataFreshnessStatus> {
        let query = r#"
            SELECT 
                COUNT(*) as total_records,
                MAX(date) as latest_date,
                COUNT(DISTINCT stock_id) as unique_stocks
            FROM daily_prices
        "#;

        let row = sqlx::query(query).fetch_one(&self.pool).await?;
        let total_records: i64 = row.get("total_records");
        let latest_date_str: Option<String> = row.get("latest_date");
        let unique_stocks: i64 = row.get("unique_stocks");

        let (status, staleness_days, message) = if let Some(ref date_str) = latest_date_str {
            let latest_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")?;
            let today = Local::now().naive_local().date();
            let staleness = today.signed_duration_since(latest_date).num_days();

            let status = if staleness <= 2 {
                FreshnessStatus::Current
            } else if staleness <= 7 {
                FreshnessStatus::Stale
            } else if staleness <= 30 {
                FreshnessStatus::Stale
            } else {
                FreshnessStatus::Stale
            };

            let message = format!("Latest price data: {}, {} days old", latest_date, staleness);
            (status, Some(staleness), message)
        } else {
            (FreshnessStatus::Missing, None, "No price data found".to_string())
        };

        let priority = match status {
            FreshnessStatus::Current => RefreshPriority::Low,
            FreshnessStatus::Stale => RefreshPriority::Medium,
            FreshnessStatus::Missing | FreshnessStatus::Error => RefreshPriority::Critical,
        };

        Ok(DataFreshnessStatus {
            data_source: "daily_prices".to_string(),
            status,
            latest_data_date: latest_date_str.clone(),
            last_refresh: None,
            staleness_days,
            records_count: total_records,
            message,
            refresh_priority: priority,
            data_summary: DataSummary {
                date_range: latest_date_str,
                stock_count: Some(unique_stocks),
                data_types: vec!["Daily Prices".to_string()],
                key_metrics: vec![format!("{} records, {} stocks", total_records, unique_stocks)],
                completeness_score: None,
            },
        })
    }

    /// Check financial_statements table directly
    async fn check_financial_statements_direct(&self) -> Result<DataFreshnessStatus> {
        let query = r#"
            SELECT 
                COUNT(*) as total_records,
                MAX(report_date) as latest_date,
                COUNT(DISTINCT stock_id) as unique_stocks
            FROM income_statements
            WHERE period_type = 'Annual' AND report_date <= date('now')
        "#;

        let row = sqlx::query(query).fetch_one(&self.pool).await?;
        let total_records: i64 = row.get("total_records");
        let latest_date_str: Option<String> = row.get("latest_date");
        let unique_stocks: i64 = row.get("unique_stocks");

        let (status, staleness_days, message) = if let Some(ref date_str) = latest_date_str {
            let latest_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")?;
            let today = Local::now().naive_local().date();
            let staleness = today.signed_duration_since(latest_date).num_days();

            let status = if staleness <= 30 {
                FreshnessStatus::Current
            } else if staleness <= 90 {
                FreshnessStatus::Stale
            } else {
                FreshnessStatus::Stale
            };

            let message = format!("Latest financial data: {}, {} days old", latest_date, staleness);
            (status, Some(staleness), message)
        } else {
            (FreshnessStatus::Missing, None, "No financial data found".to_string())
        };

        let priority = match status {
            FreshnessStatus::Current => RefreshPriority::Low,
            FreshnessStatus::Stale => RefreshPriority::Medium,
            FreshnessStatus::Missing | FreshnessStatus::Error => RefreshPriority::Critical,
        };

        Ok(DataFreshnessStatus {
            data_source: "financial_statements".to_string(),
            status,
            latest_data_date: latest_date_str.clone(),
            last_refresh: None,
            staleness_days,
            records_count: total_records,
            message,
            refresh_priority: priority,
            data_summary: DataSummary {
                date_range: latest_date_str,
                stock_count: Some(unique_stocks),
                data_types: vec!["Income Statements".to_string()],
                key_metrics: vec![format!("{} records, {} stocks", total_records, unique_stocks)],
                completeness_score: None,
            },
        })
    }


    /// Check cash_flow_statements table directly
    async fn check_cash_flow_statements_direct(&self) -> Result<DataFreshnessStatus> {
        let query = r#"
            SELECT 
                COUNT(*) as total_records,
                MAX(report_date) as latest_date,
                COUNT(DISTINCT stock_id) as unique_stocks
            FROM cash_flow_statements
            WHERE period_type = 'Annual'
        "#;

        let row = sqlx::query(query).fetch_one(&self.pool).await?;
        let total_records: i64 = row.get("total_records");
        let latest_date_str: Option<String> = row.get("latest_date");
        let unique_stocks: i64 = row.get("unique_stocks");

        let (status, staleness_days, message) = if let Some(ref date_str) = latest_date_str {
            let latest_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")?;
            let today = Local::now().naive_local().date();
            let staleness = today.signed_duration_since(latest_date).num_days();

            let status = if staleness <= 30 {
                FreshnessStatus::Current
            } else if staleness <= 90 {
                FreshnessStatus::Stale
            } else {
                FreshnessStatus::Stale
            };

            let message = format!("Latest cash flow data: {}, {} days old", latest_date, staleness);
            (status, Some(staleness), message)
        } else {
            (FreshnessStatus::Missing, None, "No cash flow data found".to_string())
        };

        let priority = match status {
            FreshnessStatus::Current => RefreshPriority::Low,
            FreshnessStatus::Stale => RefreshPriority::Medium,
            FreshnessStatus::Missing | FreshnessStatus::Error => RefreshPriority::Critical,
        };

        Ok(DataFreshnessStatus {
            data_source: "cash_flow_statements".to_string(),
            status,
            latest_data_date: latest_date_str.clone(),
            last_refresh: None,
            staleness_days,
            records_count: total_records,
            message,
            refresh_priority: priority,
            data_summary: DataSummary {
                date_range: latest_date_str,
                stock_count: Some(unique_stocks),
                data_types: vec!["Cash Flow Statements".to_string()],
                key_metrics: vec![format!("{} records, {} stocks", total_records, unique_stocks)],
                completeness_score: None,
            },
        })
    }


    async fn generate_market_data_summary(&self, records_count: i64, latest_date: &Option<String>) -> Result<DataSummary> {
        // Get detailed market data stats
        let stats_query = r#"
            SELECT
                MIN(date) as earliest_date,
                MAX(date) as latest_date,
                COUNT(DISTINCT stock_id) as unique_stocks,
                COUNT(*) as total_records,
                AVG(volume) as avg_volume
            FROM daily_prices
        "#;

        let row = sqlx::query(stats_query).fetch_one(&self.pool).await?;
        let earliest_date: Option<String> = row.get("earliest_date");
        let unique_stocks: i64 = row.get("unique_stocks");
        let avg_volume: Option<f64> = row.get("avg_volume");

        let date_range = match (earliest_date, latest_date) {
            (Some(start), Some(end)) => Some(format!("{} to {}", start, end)),
            (None, Some(end)) => Some(format!("Up to {}", end)),
            _ => None,
        };

        let mut key_metrics = vec![
            format!("{} stocks", unique_stocks),
            format!("{:.1}M records", records_count as f64 / 1_000_000.0),
        ];

        if let Some(vol) = avg_volume {
            key_metrics.push(format!("Avg volume: {:.0}K", vol / 1000.0));
        }

        // Calculate completeness score based on S&P 500 coverage
        let sp500_count_query = "SELECT COUNT(*) FROM sp500_symbols";
        let sp500_total: i64 = sqlx::query_scalar(sp500_count_query).fetch_one(&self.pool).await?;
        let completeness_score = if sp500_total > 0 {
            Some((unique_stocks as f32 / sp500_total as f32) * 100.0)
        } else {
            None
        };

        Ok(DataSummary {
            date_range,
            stock_count: Some(unique_stocks),
            data_types: vec!["Daily Prices".to_string(), "Volume Data".to_string(), "OHLC Data".to_string()],
            key_metrics,
            completeness_score,
        })
    }

    async fn generate_financial_data_summary(&self, _records_count: i64, latest_date: &Option<String>) -> Result<DataSummary> {
        // Get detailed financial data stats
        let stats_query = r#"
            SELECT
                MIN(report_date) as earliest_date,
                MAX(report_date) as latest_date,
                COUNT(DISTINCT stock_id) as unique_stocks,
                COUNT(DISTINCT CASE WHEN period_type = 'TTM' THEN stock_id END) as stocks_with_ttm,
                COUNT(*) as total_records
            FROM income_statements
        "#;

        let row = sqlx::query(stats_query).fetch_one(&self.pool).await?;
        let earliest_date: Option<String> = row.get("earliest_date");
        let unique_stocks: i64 = row.get("unique_stocks");
        let stocks_with_ttm: i64 = row.get("stocks_with_ttm");

        // Check for cash flow data
        let cash_flow_query = "SELECT COUNT(DISTINCT stock_id) FROM cash_flow_statements WHERE period_type = 'TTM'";
        let cash_flow_stocks: i64 = sqlx::query_scalar(cash_flow_query).fetch_one(&self.pool).await.unwrap_or(0);

        let date_range = match (earliest_date, latest_date) {
            (Some(start), Some(end)) => Some(format!("{} to {}", start, end)),
            (None, Some(end)) => Some(format!("Up to {}", end)),
            _ => None,
        };

        let mut data_types = vec!["Income Statements".to_string(), "Balance Sheets".to_string()];
        if cash_flow_stocks > 0 {
            data_types.push("Cash Flow Statements".to_string());
        }

        let key_metrics = vec![
            format!("{} stocks", unique_stocks),
            format!("{} with TTM data", stocks_with_ttm),
            format!("{} with cash flow", cash_flow_stocks),
        ];

        // Calculate completeness score for Piotroski analysis (needs all three statement types)
        let piotroski_ready_query = r#"
            SELECT COUNT(DISTINCT s.id) FROM stocks s
            JOIN income_statements i ON s.id = i.stock_id AND i.period_type = 'TTM'
            JOIN balance_sheets b ON s.id = b.stock_id AND b.period_type = 'TTM'
            JOIN cash_flow_statements c ON s.id = c.stock_id AND c.period_type = 'TTM'
            JOIN sp500_symbols sp ON s.symbol = sp.symbol
        "#;
        let piotroski_ready: i64 = sqlx::query_scalar(piotroski_ready_query).fetch_one(&self.pool).await.unwrap_or(0);

        let sp500_total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sp500_symbols").fetch_one(&self.pool).await?;
        let completeness_score = if sp500_total > 0 {
            Some((piotroski_ready as f32 / sp500_total as f32) * 100.0)
        } else {
            None
        };

        Ok(DataSummary {
            date_range,
            stock_count: Some(unique_stocks),
            data_types,
            key_metrics,
            completeness_score,
        })
    }


    async fn generate_cash_flow_summary(&self, _records_count: i64, latest_date: &Option<String>) -> Result<DataSummary> {
        let stats_query = r#"
            SELECT
                MIN(report_date) as earliest_date,
                MAX(report_date) as latest_date,
                COUNT(DISTINCT stock_id) as unique_stocks,
                COUNT(DISTINCT CASE WHEN period_type = 'TTM' THEN stock_id END) as stocks_with_ttm,
                COUNT(DISTINCT CASE WHEN period_type = 'Quarterly' THEN stock_id END) as stocks_with_quarterly
            FROM cash_flow_statements
        "#;

        let row = sqlx::query(stats_query).fetch_one(&self.pool).await?;
        let earliest_date: Option<String> = row.get("earliest_date");
        let unique_stocks: i64 = row.get("unique_stocks");
        let stocks_with_ttm: i64 = row.get("stocks_with_ttm");
        let stocks_with_quarterly: i64 = row.get("stocks_with_quarterly");

        let date_range = match (earliest_date, latest_date) {
            (Some(start), Some(end)) => Some(format!("{} to {}", start, end)),
            (None, Some(end)) => Some(format!("Up to {}", end)),
            _ => None,
        };

        let mut data_types = vec!["Cash Flow Statements".to_string()];
        if stocks_with_ttm > 0 { data_types.push("TTM Data".to_string()); }
        if stocks_with_quarterly > 0 { data_types.push("Quarterly Data".to_string()); }

        let key_metrics = vec![
            format!("{} stocks", unique_stocks),
            format!("{} with TTM", stocks_with_ttm),
            format!("{} quarterly", stocks_with_quarterly),
        ];

        let sp500_total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sp500_symbols").fetch_one(&self.pool).await?;
        let completeness_score = if sp500_total > 0 {
            Some((stocks_with_ttm as f32 / sp500_total as f32) * 100.0)
        } else {
            None
        };

        Ok(DataSummary {
            date_range,
            stock_count: Some(unique_stocks),
            data_types,
            key_metrics,
            completeness_score,
        })
    }

    /// Update tracking after successful data import
    pub async fn update_import_status(
        pool: &SqlitePool,
        data_source: &str,
        records_updated: i64,
        latest_data_date: Option<&str>
    ) -> Result<()> {
        let query = r#"
            UPDATE data_refresh_status
            SET
                latest_data_date = ?,
                last_successful_refresh = datetime('now'),
                last_refresh_complete = datetime('now'),
                refresh_status = 'current',
                records_updated = ?,
                updated_at = datetime('now')
            WHERE data_source = ?
        "#;

        sqlx::query(query)
            .bind(latest_data_date)
            .bind(records_updated)
            .bind(data_source)
            .execute(pool)
            .await?;

        println!("✅ Updated tracking for {} ({} records)", data_source, records_updated);
        Ok(())
    }

    /// Update tracking with total database count for a data source
    pub async fn update_tracking_with_total_count(
        pool: &SqlitePool,
        data_source: &str
    ) -> Result<()> {
        let (total_count, latest_date) = match data_source {
            "daily_prices" => {
                let row = sqlx::query("SELECT COUNT(*) as count, MAX(date) as latest FROM daily_prices")
                    .fetch_one(pool).await?;
                let count: i64 = row.get("count");
                let latest: Option<String> = row.get("latest");
                (count, latest)
            },
            "financial_statements" => {
                let row = sqlx::query("SELECT COUNT(*) as count, MAX(report_date) as latest FROM income_statements")
                    .fetch_one(pool).await?;
                let count: i64 = row.get("count");
                let latest: Option<String> = row.get("latest");
                (count, latest)
            },
            _ => return Err(anyhow::anyhow!("Unknown data source: {}", data_source))
        };

        let query = r#"
            UPDATE data_refresh_status
            SET
                latest_data_date = ?,
                last_successful_refresh = datetime('now'),
                refresh_status = 'current',
                records_updated = ?,
                updated_at = datetime('now')
            WHERE data_source = ?
        "#;

        sqlx::query(query)
            .bind(latest_date.as_deref())
            .bind(total_count)
            .bind(data_source)
            .execute(pool)
            .await?;

        println!("✅ Updated tracking for {} with total count: {} records", data_source, total_count);
        Ok(())
    }

    /// Check daily price data freshness
    #[allow(dead_code)]
    async fn check_daily_prices(&self) -> Result<DataFreshnessStatus> {
        let query = r#"
            SELECT
                MAX(date) as latest_date,
                COUNT(*) as total_records,
                COUNT(DISTINCT stock_id) as unique_stocks
            FROM daily_prices
        "#;

        let row = sqlx::query(query).fetch_one(&self.pool).await?;

        let latest_date_str: Option<String> = row.get("latest_date");
        let total_records: i64 = row.get("total_records");

        let (status, staleness_days, message) = if let Some(ref date_str) = latest_date_str {
            let latest_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")?;
            let today = Local::now().naive_local().date();
            let staleness = today.signed_duration_since(latest_date).num_days();

            let status = if staleness <= 7 {
                FreshnessStatus::Current
            } else if staleness <= 30 {
                FreshnessStatus::Stale
            } else {
                FreshnessStatus::Error
            };

            let message = format!("Latest price data: {}, {} days old", latest_date, staleness);
            (status, Some(staleness), message)
        } else {
            (FreshnessStatus::Missing, None, "No price data found".to_string())
        };

        let priority = match status {
            FreshnessStatus::Current => RefreshPriority::Low,
            FreshnessStatus::Stale => RefreshPriority::Medium,
            FreshnessStatus::Missing | FreshnessStatus::Error => RefreshPriority::Critical,
        };

        Ok(DataFreshnessStatus {
            data_source: "daily_prices".to_string(),
            status,
            latest_data_date: latest_date_str.clone(),
            last_refresh: None, // TODO: Get from refresh tracking table
            staleness_days,
            records_count: total_records,
            message,
            refresh_priority: priority,
            data_summary: self.generate_market_data_summary(total_records, &latest_date_str).await.unwrap_or(DataSummary {
                date_range: latest_date_str,
                stock_count: None,
                data_types: vec!["Daily Prices".to_string()],
                key_metrics: vec![format!("{} records", total_records)],
                completeness_score: None,
            }),
        })
    }


    /// Determine overall system status
    fn determine_overall_status(&self, data_sources: &HashMap<String, DataFreshnessStatus>) -> FreshnessStatus {
        // Only consider core data sources for overall status
        let core_sources = ["daily_prices", "financial_statements", "cash_flow_statements"];
        let core_statuses: Vec<&FreshnessStatus> = data_sources
            .iter()
            .filter(|(key, _)| core_sources.contains(&key.as_str()))
            .map(|(_, ds)| &ds.status)
            .collect();

        // Only show Error if core data sources have real errors
        if core_statuses.iter().any(|s| **s == FreshnessStatus::Error) {
            FreshnessStatus::Error
        } else if core_statuses.iter().any(|s| **s == FreshnessStatus::Stale) {
            FreshnessStatus::Stale
        } else if core_statuses.iter().any(|s| **s == FreshnessStatus::Missing) {
            FreshnessStatus::Stale  // Show missing as stale, not error
        } else {
            FreshnessStatus::Current
        }
    }

    /// Generate refresh recommendations based on data status
    async fn generate_recommendations(&self, data_sources: &HashMap<String, DataFreshnessStatus>) -> Result<Vec<RefreshRecommendation>> {
        let mut recommendations = Vec::new();

        // Sort by priority to recommend most critical first
        let mut sources: Vec<_> = data_sources.values().collect();
        sources.sort_by(|a, b| b.refresh_priority.partial_cmp(&a.refresh_priority).unwrap_or(std::cmp::Ordering::Equal));

        for source in sources {
            match (&source.status, &source.refresh_priority) {
                (FreshnessStatus::Missing | FreshnessStatus::Error, _) => {
                    recommendations.push(RefreshRecommendation {
                        action: format!("Critical: Refresh {}", source.data_source),
                        reason: source.message.clone(),
                        estimated_duration: self.estimate_refresh_duration(&source.data_source),
                        priority: source.refresh_priority.clone(),
                    });
                }
                (FreshnessStatus::Stale, RefreshPriority::High | RefreshPriority::Critical) => {
                    recommendations.push(RefreshRecommendation {
                        action: format!("High priority: Refresh {}", source.data_source),
                        reason: source.message.clone(),
                        estimated_duration: self.estimate_refresh_duration(&source.data_source),
                        priority: source.refresh_priority.clone(),
                    });
                }
                (FreshnessStatus::Stale, _) => {
                    recommendations.push(RefreshRecommendation {
                        action: format!("Recommended: Refresh {}", source.data_source),
                        reason: source.message.clone(),
                        estimated_duration: self.estimate_refresh_duration(&source.data_source),
                        priority: source.refresh_priority.clone(),
                    });
                }
                _ => {} // Current data doesn't need refresh
            }
        }

        Ok(recommendations)
    }

    /// Assess readiness for different screening analyses
    async fn assess_screening_readiness(&self, data_sources: &HashMap<String, DataFreshnessStatus>) -> Result<ScreeningReadiness> {
        let mut blocking_issues = Vec::new();


        // Valuation analysis requires P/S and EV/S ratios
        let valuation_ready = self.check_valuation_readiness(data_sources, &mut blocking_issues);

        Ok(ScreeningReadiness {
            valuation_analysis: valuation_ready,
            blocking_issues,
        })
    }


    fn check_valuation_readiness(&self, data_sources: &HashMap<String, DataFreshnessStatus>, blocking_issues: &mut Vec<String>) -> bool {
        let ratio_status = data_sources.get("ps_evs_ratios").map(|ds| &ds.status);

        match ratio_status {
            Some(FreshnessStatus::Current | FreshnessStatus::Stale) => true,
            _ => {
                blocking_issues.push("Valuation analysis blocked: P/S and EV/S ratios missing".to_string());
                false
            }
        }
    }

    fn estimate_refresh_duration(&self, data_source: &str) -> String {
        match data_source {
            "daily_prices" => "5-15 minutes".to_string(),
            "pe_ratios" => "10-30 minutes".to_string(),
            "ps_evs_ratios" => "5-10 minutes".to_string(),
            "financial_statements" => "30-60 minutes".to_string(),
            "company_metadata" => "1-2 minutes".to_string(),
            _ => "Unknown".to_string(),
        }
    }

}

impl FreshnessStatus {
    pub fn is_current(&self) -> bool {
        matches!(self, FreshnessStatus::Current)
    }

    pub fn needs_refresh(&self) -> bool {
        matches!(self, FreshnessStatus::Stale | FreshnessStatus::Missing | FreshnessStatus::Error)
    }
}

impl SystemFreshnessReport {
    pub fn get_stale_components(&self) -> Vec<String> {
        let mut stale_components = Vec::new();
        
        if self.market_data.status.needs_refresh() {
            stale_components.push("market_data".to_string());
        }
        if self.financial_data.status.needs_refresh() {
            stale_components.push("financial_data".to_string());
        }
        if self.calculated_ratios.status.needs_refresh() {
            stale_components.push("calculated_ratios".to_string());
        }
        
        stale_components
    }

    pub fn get_stale_components_message(&self) -> String {
        let stale = self.get_stale_components();
        if stale.is_empty() {
            "All data is current".to_string()
        } else {
            format!("Stale data sources: {}", stale.join(", "))
        }
    }

    pub fn is_ready_for_analysis(&self, analysis_type: &str) -> bool {
        match analysis_type {
            "valuation" => self.screening_readiness.valuation_analysis,
            _ => false,
        }
    }
}