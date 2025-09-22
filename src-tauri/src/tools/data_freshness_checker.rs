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
    pub garp_screening: bool,
    pub graham_screening: bool,
    pub valuation_analysis: bool,
    pub blocking_issues: Vec<String>,
}

pub struct DataStatusReader {
    pool: SqlitePool,
    today: NaiveDate,
}

impl DataStatusReader {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            today: Local::now().naive_local().date(),
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

    /// Fast check using existing summary view (0.004s vs 10s)
    async fn check_data_sources_fast(&self) -> Result<HashMap<String, DataFreshnessStatus>> {
        let query = r#"
            SELECT 
                data_source,
                refresh_status,
                latest_data_date,
                last_successful_refresh,
                records_updated,
                error_message,
                max_staleness_days,
                refresh_frequency_hours,
                auto_refresh_enabled,
                refresh_priority,
                staleness_days,
                refresh_recommendation,
                next_recommended_refresh
            FROM v_data_freshness_summary
        "#;
        let rows = sqlx::query(query).fetch_all(&self.pool).await?;

        let mut data_sources = HashMap::new();

        for row in rows {
            let source_name: String = row.get(freshness_columns::DATA_SOURCE);
            let _status_str: String = row.get(freshness_columns::REFRESH_STATUS);
            let latest_date_str: Option<String> = row.try_get(freshness_columns::LATEST_DATA_DATE).ok();
            let last_refresh_str: Option<String> = row.try_get(freshness_columns::LAST_SUCCESSFUL_REFRESH).ok();
            let records_updated: i64 = row.get(freshness_columns::RECORDS_UPDATED);
            let staleness_days: i64 = row.get(freshness_columns::STALENESS_DAYS);

            // Determine status based on staleness_days, not just database status
            let status = if staleness_days <= 2 {
                FreshnessStatus::Current
            } else if staleness_days <= 7 {
                FreshnessStatus::Stale
            } else if staleness_days <= 30 {
                FreshnessStatus::Stale
            } else {
                FreshnessStatus::Stale // Keep as Stale for very old data
            };

            let priority = match status {
                FreshnessStatus::Current => RefreshPriority::Low,
                FreshnessStatus::Stale => RefreshPriority::Medium,
                _ => RefreshPriority::Critical,
            };

            data_sources.insert(source_name.clone(), DataFreshnessStatus {
                data_source: source_name,
                status: status.clone(),
                latest_data_date: latest_date_str,
                last_refresh: last_refresh_str,
                staleness_days: Some(staleness_days),
                records_count: records_updated,
                message: format!("Status: {:?}, {} records, {} days old", status, records_updated, staleness_days),
                refresh_priority: priority,
            });
        }

        Ok(data_sources)
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
            "ps_evs_ratios" => {
                let row = sqlx::query("SELECT COUNT(*) as count, MAX(date) as latest FROM daily_valuation_ratios")
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
            let staleness = self.today.signed_duration_since(latest_date).num_days();

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
            latest_data_date: latest_date_str,
            last_refresh: None, // TODO: Get from refresh tracking table
            staleness_days,
            records_count: total_records,
            message,
            refresh_priority: priority,
        })
    }

    /// Check P/E ratio freshness (critical for GARP analysis)
    async fn check_pe_ratios(&self) -> Result<DataFreshnessStatus> {
        let query = r#"
            SELECT
                MAX(date) as latest_pe_date,
                COUNT(*) as pe_records,
                COUNT(DISTINCT stock_id) as stocks_with_pe
            FROM daily_valuation_ratios
            WHERE pe_ratio IS NOT NULL
        "#;

        let row = sqlx::query(query).fetch_one(&self.pool).await?;

        let latest_date_str: Option<String> = row.get("latest_pe_date");
        let pe_records: i64 = row.get("pe_records");

        // Also check the latest date in daily_prices for comparison
        let price_query = "SELECT MAX(date) as latest_price_date FROM daily_prices";
        let price_row = sqlx::query(price_query).fetch_one(&self.pool).await?;
        let latest_price_date_str: Option<String> = price_row.get("latest_price_date");

        let (status, staleness_days, message) = if let (Some(pe_date_str), Some(price_date_str)) = (latest_date_str.as_ref(), latest_price_date_str.as_ref()) {
            let pe_date = NaiveDate::parse_from_str(pe_date_str, "%Y-%m-%d")?;
            let price_date = NaiveDate::parse_from_str(price_date_str, "%Y-%m-%d")?;

            let pe_staleness = self.today.signed_duration_since(pe_date).num_days();
            let price_gap = price_date.signed_duration_since(pe_date).num_days();

            let status = if price_gap <= 2 && pe_staleness <= 7 {
                FreshnessStatus::Current
            } else if price_gap <= 7 && pe_staleness <= 30 {
                FreshnessStatus::Stale
            } else {
                FreshnessStatus::Error
            };

            let message = if price_gap > 2 {
                format!("P/E ratios ({}) lag behind prices ({}) by {} days", pe_date, price_date, price_gap)
            } else {
                format!("P/E ratios current to {}, {} days old", pe_date, pe_staleness)
            };

            (status, Some(pe_staleness), message)
        } else {
            (FreshnessStatus::Missing, None, "No P/E ratio data found".to_string())
        };

        let priority = match status {
            FreshnessStatus::Current => RefreshPriority::Low,
            FreshnessStatus::Stale => RefreshPriority::High, // P/E is critical for GARP
            FreshnessStatus::Missing | FreshnessStatus::Error => RefreshPriority::Critical,
        };

        Ok(DataFreshnessStatus {
            data_source: "pe_ratios".to_string(),
            status,
            latest_data_date: latest_date_str,
            last_refresh: None,
            staleness_days,
            records_count: pe_records,
            message,
            refresh_priority: priority,
        })
    }

    /// Determine overall system status
    fn determine_overall_status(&self, data_sources: &HashMap<String, DataFreshnessStatus>) -> FreshnessStatus {
        // Only consider core data sources for overall status
        let core_sources = ["daily_prices", "financial_statements", "ps_evs_ratios"];
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

        // GARP screening requires current P/E ratios and financial data
        let garp_ready = self.check_garp_readiness(data_sources, &mut blocking_issues);

        // Graham screening requires current prices and P/E ratios
        let graham_ready = self.check_graham_readiness(data_sources, &mut blocking_issues);

        // Valuation analysis requires P/S and EV/S ratios
        let valuation_ready = self.check_valuation_readiness(data_sources, &mut blocking_issues);

        Ok(ScreeningReadiness {
            garp_screening: garp_ready,
            graham_screening: graham_ready,
            valuation_analysis: valuation_ready,
            blocking_issues,
        })
    }

    fn check_garp_readiness(&self, data_sources: &HashMap<String, DataFreshnessStatus>, blocking_issues: &mut Vec<String>) -> bool {
        let pe_status = data_sources.get("pe_ratios").map(|ds| &ds.status);
        let financial_status = data_sources.get("financial_statements").map(|ds| &ds.status);

        match (pe_status, financial_status) {
            (Some(FreshnessStatus::Current), Some(FreshnessStatus::Current | FreshnessStatus::Stale)) => true,
            _ => {
                if let Some(FreshnessStatus::Stale | FreshnessStatus::Missing | FreshnessStatus::Error) = pe_status {
                    blocking_issues.push("GARP screening blocked: P/E ratios are stale or missing".to_string());
                }
                if let Some(FreshnessStatus::Missing | FreshnessStatus::Error) = financial_status {
                    blocking_issues.push("GARP screening blocked: Financial statements missing".to_string());
                }
                false
            }
        }
    }

    fn check_graham_readiness(&self, data_sources: &HashMap<String, DataFreshnessStatus>, blocking_issues: &mut Vec<String>) -> bool {
        let price_status = data_sources.get("daily_prices").map(|ds| &ds.status);
        let pe_status = data_sources.get("pe_ratios").map(|ds| &ds.status);

        match (price_status, pe_status) {
            (Some(FreshnessStatus::Current | FreshnessStatus::Stale), Some(FreshnessStatus::Current)) => true,
            _ => {
                if let Some(FreshnessStatus::Missing | FreshnessStatus::Error) = price_status {
                    blocking_issues.push("Graham screening blocked: Price data missing".to_string());
                }
                if let Some(FreshnessStatus::Stale | FreshnessStatus::Missing | FreshnessStatus::Error) = pe_status {
                    blocking_issues.push("Graham screening blocked: P/E ratios are stale or missing".to_string());
                }
                false
            }
        }
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

    /// Quick check specifically for GARP analysis
    pub async fn check_garp_data_freshness(&self) -> Result<DataFreshnessStatus> {
        self.check_pe_ratios().await
    }

    /// Quick check specifically for Graham analysis
    pub async fn check_graham_data_freshness(&self) -> Result<DataFreshnessStatus> {
        let pe_status = self.check_pe_ratios().await?;
        let price_status = self.check_daily_prices().await?;

        // Return the worst status between the two
        let combined_status = match (&pe_status.status, &price_status.status) {
            (FreshnessStatus::Current, FreshnessStatus::Current) => FreshnessStatus::Current,
            (FreshnessStatus::Stale, FreshnessStatus::Current) |
            (FreshnessStatus::Current, FreshnessStatus::Stale) |
            (FreshnessStatus::Stale, FreshnessStatus::Stale) => FreshnessStatus::Stale,
            _ => FreshnessStatus::Error,
        };

        Ok(DataFreshnessStatus {
            data_source: "graham_screening".to_string(),
            status: combined_status.clone(),
            latest_data_date: pe_status.latest_data_date,
            last_refresh: None,
            staleness_days: pe_status.staleness_days,
            records_count: pe_status.records_count + price_status.records_count,
            message: format!("Graham readiness: {} | {}", pe_status.message, price_status.message),
            refresh_priority: if combined_status == FreshnessStatus::Current {
                RefreshPriority::Low
            } else {
                RefreshPriority::High
            },
        })
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
            "garp" => self.screening_readiness.garp_screening,
            "graham" => self.screening_readiness.graham_screening,
            "valuation" => self.screening_readiness.valuation_analysis,
            _ => false,
        }
    }
}