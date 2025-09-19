use anyhow::{Result, anyhow};
use chrono::{DateTime, Duration, Local, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFreshnessStatus {
    pub data_source: String,
    pub status: FreshnessStatus,
    pub latest_data_date: Option<NaiveDate>,
    pub last_refresh: Option<DateTime<Utc>>,
    pub staleness_days: Option<i64>,
    pub records_count: i64,
    pub message: String,
    pub refresh_priority: RefreshPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FreshnessStatus {
    Current,
    Stale,
    Missing,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum RefreshPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemFreshnessReport {
    pub overall_status: FreshnessStatus,
    pub data_sources: HashMap<String, DataFreshnessStatus>,
    pub recommendations: Vec<RefreshRecommendation>,
    pub screening_readiness: ScreeningReadiness,
    pub last_check: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshRecommendation {
    pub action: String,
    pub reason: String,
    pub estimated_duration: String,
    pub priority: RefreshPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreeningReadiness {
    pub garp_screening: bool,
    pub graham_screening: bool,
    pub valuation_analysis: bool,
    pub blocking_issues: Vec<String>,
}

pub struct DataFreshnessChecker {
    pool: SqlitePool,
    today: NaiveDate,
}

impl DataFreshnessChecker {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            today: Local::now().naive_local().date(),
        }
    }

    /// Check freshness of all data sources and generate comprehensive report
    pub async fn check_system_freshness(&self) -> Result<SystemFreshnessReport> {
        let mut data_sources = HashMap::new();
        let mut recommendations = Vec::new();

        // Check each data source
        data_sources.insert("daily_prices".to_string(), self.check_daily_prices().await?);
        data_sources.insert("pe_ratios".to_string(), self.check_pe_ratios().await?);
        data_sources.insert("ps_evs_ratios".to_string(), self.check_ps_evs_ratios().await?);
        data_sources.insert("financial_statements".to_string(), self.check_financial_statements().await?);
        data_sources.insert("company_metadata".to_string(), self.check_company_metadata().await?);

        // Determine overall status
        let overall_status = self.determine_overall_status(&data_sources);

        // Generate recommendations
        recommendations.extend(self.generate_recommendations(&data_sources).await?);

        // Check screening readiness
        let screening_readiness = self.assess_screening_readiness(&data_sources).await?;

        Ok(SystemFreshnessReport {
            overall_status,
            data_sources,
            recommendations,
            screening_readiness,
            last_check: Utc::now(),
        })
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
            latest_data_date: latest_date_str.clone().and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
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
            latest_data_date: latest_date_str.clone().and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
            last_refresh: None,
            staleness_days,
            records_count: pe_records,
            message,
            refresh_priority: priority,
        })
    }

    /// Check P/S and EV/S ratio freshness
    async fn check_ps_evs_ratios(&self) -> Result<DataFreshnessStatus> {
        let query = r#"
            SELECT
                MAX(date) as latest_ratio_date,
                COUNT(CASE WHEN ps_ratio_ttm IS NOT NULL THEN 1 END) as ps_records,
                COUNT(CASE WHEN evs_ratio_ttm IS NOT NULL THEN 1 END) as evs_records,
                COUNT(DISTINCT stock_id) as stocks_with_ratios
            FROM daily_valuation_ratios
            WHERE ps_ratio_ttm IS NOT NULL OR evs_ratio_ttm IS NOT NULL
        "#;

        let row = sqlx::query(query).fetch_one(&self.pool).await?;

        let latest_date_str: Option<String> = row.get("latest_ratio_date");
        let ps_records: i64 = row.get("ps_records");
        let evs_records: i64 = row.get("evs_records");

        let (status, staleness_days, message) = if let Some(ref date_str) = latest_date_str {
            let latest_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")?;
            let staleness = self.today.signed_duration_since(latest_date).num_days();

            let status = if staleness <= 7 {
                FreshnessStatus::Current
            } else if staleness <= 14 {
                FreshnessStatus::Stale
            } else {
                FreshnessStatus::Error
            };

            let message = format!("P/S ({}) and EV/S ({}) ratios current to {}, {} days old",
                                ps_records, evs_records, latest_date, staleness);
            (status, Some(staleness), message)
        } else {
            (FreshnessStatus::Missing, None, "No P/S or EV/S ratio data found".to_string())
        };

        let priority = match status {
            FreshnessStatus::Current => RefreshPriority::Low,
            FreshnessStatus::Stale => RefreshPriority::Medium,
            FreshnessStatus::Missing | FreshnessStatus::Error => RefreshPriority::High,
        };

        Ok(DataFreshnessStatus {
            data_source: "ps_evs_ratios".to_string(),
            status,
            latest_data_date: latest_date_str.clone().and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
            last_refresh: None,
            staleness_days,
            records_count: ps_records + evs_records,
            message,
            refresh_priority: priority,
        })
    }

    /// Check financial statements freshness
    async fn check_financial_statements(&self) -> Result<DataFreshnessStatus> {
        let query = r#"
            SELECT
                MAX(report_date) as latest_report_date,
                COUNT(*) as total_statements,
                COUNT(DISTINCT stock_id) as stocks_with_data,
                COUNT(CASE WHEN period_type = 'TTM' THEN 1 END) as ttm_statements
            FROM income_statements
        "#;

        let row = sqlx::query(query).fetch_one(&self.pool).await?;

        let latest_date_str: Option<String> = row.get("latest_report_date");
        let total_statements: i64 = row.get("total_statements");
        let ttm_statements: i64 = row.get("ttm_statements");

        let (status, staleness_days, message) = if let Some(ref date_str) = latest_date_str {
            let latest_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")?;
            let staleness = self.today.signed_duration_since(latest_date).num_days();

            // Financial data has different staleness criteria (quarterly updates)
            let status = if staleness <= 120 { // ~4 months for quarterly data
                FreshnessStatus::Current
            } else if staleness <= 180 { // ~6 months
                FreshnessStatus::Stale
            } else {
                FreshnessStatus::Error
            };

            let message = format!("Financial data: {} statements ({} TTM), latest {}, {} days old",
                                total_statements, ttm_statements, latest_date, staleness);
            (status, Some(staleness), message)
        } else {
            (FreshnessStatus::Missing, None, "No financial statement data found".to_string())
        };

        let priority = match status {
            FreshnessStatus::Current => RefreshPriority::Low,
            FreshnessStatus::Stale => RefreshPriority::Medium,
            FreshnessStatus::Missing | FreshnessStatus::Error => RefreshPriority::High,
        };

        Ok(DataFreshnessStatus {
            data_source: "financial_statements".to_string(),
            status,
            latest_data_date: latest_date_str.clone().and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
            last_refresh: None,
            staleness_days,
            records_count: total_statements,
            message,
            refresh_priority: priority,
        })
    }

    /// Check company metadata freshness
    async fn check_company_metadata(&self) -> Result<DataFreshnessStatus> {
        let query = r#"
            SELECT
                MAX(updated_at) as last_update,
                COUNT(*) as total_companies,
                COUNT(CASE WHEN earliest_data_date IS NOT NULL THEN 1 END) as with_coverage_info
            FROM company_metadata
        "#;

        let row = sqlx::query(query).fetch_one(&self.pool).await?;

        let last_update_str: Option<String> = row.get("last_update");
        let total_companies: i64 = row.get("total_companies");
        let with_coverage: i64 = row.get("with_coverage_info");

        let (status, message) = if total_companies > 0 {
            let coverage_pct = (with_coverage as f64 / total_companies as f64) * 100.0;

            let status = if coverage_pct >= 95.0 {
                FreshnessStatus::Current
            } else if coverage_pct >= 80.0 {
                FreshnessStatus::Stale
            } else {
                FreshnessStatus::Error
            };

            let message = format!("Company metadata: {} companies, {:.1}% with coverage info",
                                total_companies, coverage_pct);
            (status, message)
        } else {
            (FreshnessStatus::Missing, "No company metadata found".to_string())
        };

        Ok(DataFreshnessStatus {
            data_source: "company_metadata".to_string(),
            status,
            latest_data_date: None, // Metadata doesn't have a single date
            last_refresh: last_update_str.and_then(|s| DateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S%.f %z").ok().map(|dt| dt.with_timezone(&Utc))),
            staleness_days: None,
            records_count: total_companies,
            message,
            refresh_priority: RefreshPriority::Low,
        })
    }

    /// Determine overall system status
    fn determine_overall_status(&self, data_sources: &HashMap<String, DataFreshnessStatus>) -> FreshnessStatus {
        let statuses: Vec<&FreshnessStatus> = data_sources.values().map(|ds| &ds.status).collect();

        if statuses.iter().any(|s| **s == FreshnessStatus::Missing || **s == FreshnessStatus::Error) {
            FreshnessStatus::Error
        } else if statuses.iter().any(|s| **s == FreshnessStatus::Stale) {
            FreshnessStatus::Stale
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
    pub fn get_stale_components(&self) -> Vec<&str> {
        self.data_sources
            .iter()
            .filter(|(_, status)| status.status.needs_refresh())
            .map(|(name, _)| name.as_str())
            .collect()
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