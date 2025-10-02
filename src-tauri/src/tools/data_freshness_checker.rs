use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};
// HashMap removed - no longer used with SEC filing-based approach
use ts_rs::TS;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use crate::tools::sec_edgar_client::SecEdgarClient;

// Legacy constants - no longer used with SEC filing-based freshness

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

    /// Check freshness of all data sources and generate comprehensive report using SEC filing-based freshness
    pub async fn check_system_freshness(&self) -> Result<SystemFreshnessReport> {
        // Use our new SEC filing-based freshness checker for financial data
        self.check_financial_filing_freshness(&self.pool).await
    }

    /// Check financial data freshness using SEC Company Facts API (filing-based)
    pub async fn check_financial_filing_freshness(&self, pool: &SqlitePool) -> Result<SystemFreshnessReport> {
        println!("🔍 Checking financial data freshness using SEC Company Facts API...");
        
        // Get all S&P 500 stocks with CIKs
        let all_stocks = self.get_all_sp500_stocks_with_cik(pool).await?;
        let total_stocks_count = all_stocks.len();
        println!("  📊 Checking all {} S&P 500 stocks with CIKs", total_stocks_count);
        
        // Concurrent freshness check with rate limiting (10 requests/second = 100ms between requests)
        let stale_count = Arc::new(Mutex::new(0));
        let current_count = Arc::new(Mutex::new(0));
        let api_errors = Arc::new(Mutex::new(0));
        
        // Create a semaphore to limit concurrent requests (SEC allows 10/second)
        let semaphore = Arc::new(Semaphore::new(10)); // Allow 10 concurrent requests
        let pool_clone = pool.clone();
        
        // Process stocks concurrently with controlled concurrency
        let mut handles = Vec::new();
        
        for (stock_id, cik) in all_stocks {
            let semaphore_clone = semaphore.clone();
            let stale_count_clone = stale_count.clone();
            let current_count_clone = current_count.clone();
            let api_errors_clone = api_errors.clone();
            let pool_clone = pool_clone.clone();
            
            let handle = tokio::spawn(async move {
                // Acquire permit before making request
                let _permit = semaphore_clone.acquire().await.unwrap();
                
                // Small delay to respect rate limits (100ms per request = 10/sec)
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                
                let mut edgar_client = SecEdgarClient::new(pool_clone);
                
                match edgar_client.check_if_update_needed(&cik, stock_id).await {
                    Ok(true) => {
                        let mut count = stale_count_clone.lock().await;
                        *count += 1;
                    }
                    Ok(false) => {
                        let mut count = current_count_clone.lock().await;
                        *count += 1;
                    }
                    Err(_e) => {
                        let mut count = api_errors_clone.lock().await;
                        *count += 1;
                    }
                }
            });
            
            handles.push(handle);
        }
        
        // Wait for all tasks to complete with progress updates
        let total_tasks = handles.len();
        let completed = Arc::new(Mutex::new(0));
        
        for (_i, handle) in handles.into_iter().enumerate() {
            tokio::select! {
                _ = handle => {
                    let mut completed_count = completed.lock().await;
                    *completed_count += 1;
                    
                    if *completed_count > 0 && *completed_count % 50 == 0 {
                        println!("  📊 Progress: completed {}/{} checks", *completed_count, total_tasks);
                    }
                }
            }
        }
        
        // Get final counts
        let stale_count = *(stale_count.lock().await);
        let current_count = *(current_count.lock().await);
        let api_errors = *(api_errors.lock().await);
        
        println!("  📈 Freshness Check Results:");
        println!("    Stale (need updates): {} stocks", stale_count);
        println!("    Current: {} stocks", current_count);
        println!("    API Errors: {} stocks", api_errors);
        
        // Proper status determination based on stale count
        let freshness_status = if stale_count == 0 {
            FreshnessStatus::Current
        } else {
            FreshnessStatus::Stale
        };
        
        // Get basic market data status (this isn't SEC-related, so keep existing)
        let market_financial_check = self.check_daily_prices_direct().await?;
        let market_cash_flow_check = self.check_cash_flow_statements_direct().await?;
        
        Ok(SystemFreshnessReport {
            overall_status: freshness_status.clone(),
            market_data: market_financial_check.clone(),
            financial_data: DataFreshnessStatus {
                data_source: "financial_statements".to_string(),
                status: freshness_status.clone(),
                latest_data_date: None,
                last_refresh: None,
                staleness_days: None,
                records_count: stale_count,
                message: if stale_count == 0 {
                    format!("All {} stocks are up-to-date with latest SEC filings", total_stocks_count)
                } else {
                    format!("{} out of {} stocks need SEC updates - run 'cargo run --bin refresh_data financials'", stale_count, total_stocks_count)
                },
                refresh_priority: RefreshPriority::High,
                data_summary: DataSummary {
                    date_range: None,
                    stock_count: Some(stale_count),
                    data_types: vec!["SEC Financials".to_string()],
                    key_metrics: vec![if stale_count == 0 { 
                        "All stocks current".to_string() 
                    } else { 
                        format!("{} stocks need refresh", stale_count) 
                    }],
                    completeness_score: Some(100.0),
                },
            },
            calculated_ratios: DataFreshnessStatus {
                data_source: "screening_readiness".to_string(),
                status: if freshness_status.is_current() && market_financial_check.status.is_current() {
                    FreshnessStatus::Current
                } else {
                    FreshnessStatus::Stale
                },
                latest_data_date: None,
                last_refresh: None,
                staleness_days: None,
                records_count: 0,
                message: if freshness_status.is_current() && market_financial_check.status.is_current() {
                    "Screening analysis ready".to_string()
                } else {
                    "Screening blocked: data issues".to_string()
                },
                refresh_priority: RefreshPriority::Medium,
                data_summary: DataSummary {
                    date_range: None,
                    stock_count: None,
                    data_types: vec!["Piotroski F-Score".to_string(), "O'Shaughnessy Value".to_string()],
                    key_metrics: vec!["Screening Ready".to_string()],
                    completeness_score: None,
                },
            },
            recommendations: vec![],
            screening_readiness: ScreeningReadiness {
                valuation_analysis: freshness_status.is_current() && market_cash_flow_check.status.is_current(),
                blocking_issues: if freshness_status.is_current() { vec![] } else { vec!["Financial data outdated based on SEC filings".to_string()] },
            },
            last_check: Utc::now().to_rfc3339(),
        })
    }
    
    /// Get all S&P 500 stocks with CIKs for freshness checking
    async fn get_all_sp500_stocks_with_cik(&self, pool: &SqlitePool) -> Result<Vec<(i64, String)>> {
        let query = r#"
            SELECT s.id, s.cik
            FROM stocks s
            WHERE s.is_sp500 = 1
                AND s.cik IS NOT NULL 
                AND s.cik != ''
                AND s.cik != 'Unknown'
            ORDER BY s.symbol
        "#;
        
        let rows = sqlx::query(query).fetch_all(pool).await?;
        let mut stocks = Vec::new();
        
        for row in rows {
            let stock_id: i64 = row.get("id");
            let cik: String = row.get("cik");
            stocks.push((stock_id, cik));
        }
        
        Ok(stocks)
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
        let latest_date: Option<chrono::NaiveDate> = row.get("latest_date");
        let _unique_stocks: i64 = row.get("unique_stocks");
        
        let latest_date_str = latest_date.map(|d| d.format("%Y-%m-%d").to_string());
        
        let staleness_days = match latest_date {
            Some(date) => {
                let days_diff = Utc::now().date_naive() - date;
                Some(days_diff.num_days())
            }
            None => None,
        };
        
        let status = match (latest_date, staleness_days) {
            (None, _) => FreshnessStatus::Missing,
            (_, Some(days)) if days <= 7 => FreshnessStatus::Current,
            (_, Some(days)) if days <= 30 => FreshnessStatus::Stale,
            (_, Some(_)) => FreshnessStatus::Stale, // Consider anything > 30 days as stale
            _ => FreshnessStatus::Current,
        };
        
        let message = match status {
            FreshnessStatus::Current => format!("Latest data: {} ({} records)", latest_date_str.as_deref().unwrap_or("N/A"), total_records),
            FreshnessStatus::Stale => format!("Latest data: {} days old ({} records)", staleness_days.unwrap_or(0), total_records),
            FreshnessStatus::Missing => "No market data available".to_string(),
            FreshnessStatus::Error => "Error accessing market data".to_string(),
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
            data_summary: DataSummary {
                date_range: latest_date_str.clone(),
                stock_count: None,
                data_types: vec!["Daily Prices".to_string()],
                key_metrics: vec![format!("{} records", total_records)],
                completeness_score: None,
            },
        })
    }

    /// Check cash flow statements table directly
    async fn check_cash_flow_statements_direct(&self) -> Result<DataFreshnessStatus> {
        let query = r#"
            SELECT 
                COUNT(*) as total_records,
                MAX(report_date) as latest_date,
                COUNT(DISTINCT stock_id) as unique_stocks
            FROM cash_flow_statements
        "#;
        
        let row = sqlx::query(query).fetch_one(&self.pool).await?;
        let total_records: i64 = row.get("total_records");
        let latest_date: Option<chrono::NaiveDate> = row.get("latest_date");
        let _unique_stocks: i64 = row.get("unique_stocks");
        
        let latest_date_str = latest_date.map(|d| d.format("%Y-%m-%d").to_string());
        
        let staleness_days = match latest_date {
            Some(date) => {
                let days_diff = Utc::now().date_naive() - date;
                Some(days_diff.num_days())
            }
            None => None,
        };
        
        // Cash flow statements are typically less frequent so use different thresholds
        let status = match (latest_date, staleness_days) {
            (None, _) => FreshnessStatus::Missing,
            (_, Some(days)) if days <= 14 => FreshnessStatus::Current,
            (_, Some(days)) if days <= 90 => FreshnessStatus::Stale,
            (_, Some(_)) => FreshnessStatus::Stale,
            _ => FreshnessStatus::Current,
        };
        
        let message = match status {
            FreshnessStatus::Current => format!("Latest data: {} ({} records)", latest_date_str.as_deref().unwrap_or("N/A"), total_records),
            FreshnessStatus::Stale => format!("Latest data: {} days old ({} records)", staleness_days.unwrap_or(0), total_records),
            FreshnessStatus::Missing => "No cash flow data available".to_string(),
            FreshnessStatus::Error => "Error accessing cash flow data".to_string(),
        };

        let priority = match status {
            FreshnessStatus::Current => RefreshPriority::Low,
            FreshnessStatus::Stale => RefreshPriority::Medium,
            FreshnessStatus::Missing | FreshnessStatus::Error => RefreshPriority::High,
        };

        Ok(DataFreshnessStatus {
            data_source: "cash_flow_statements".to_string(),
            status,
            latest_data_date: latest_date_str.clone(),
            last_refresh: None, // TODO: Get from refresh tracking table
            staleness_days,
            records_count: total_records,
            message,
            refresh_priority: priority,
            data_summary: DataSummary {
                date_range: latest_date_str.clone(),
                stock_count: Some(_unique_stocks),
                data_types: vec!["Cash Flow Statements".to_string()],
                key_metrics: vec![format!("{} records", total_records)],
                completeness_score: None,
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
            "valuation" => self.screening_readiness.valuation_analysis,
            _ => false,
        }
    }
}