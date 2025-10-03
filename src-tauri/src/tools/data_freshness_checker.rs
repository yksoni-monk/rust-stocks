use anyhow::Result;
use chrono::Utc;
use governor::{Quota, RateLimiter};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{SqlitePool, Row};
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Semaphore, Mutex};
use ts_rs::TS;

// SEC Company Facts API data structures
#[derive(Debug, Clone)]
pub struct FilingFreshnessResult {
    pub cik: String,
    pub our_latest_date: Option<String>,
    pub sec_latest_date: Option<String>,
    pub is_stale: bool,
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

    /// Check freshness of all data sources and generate comprehensive report using SEC filing-based freshness
    pub async fn check_system_freshness(&self) -> Result<SystemFreshnessReport> {
        // Use our new SEC filing-based freshness checker for financial data
        self.check_financial_filing_freshness().await
    }

    /// Check financial data freshness using SEC Company Facts API (NEW APPROACH)
    pub async fn check_financial_filing_freshness(&self) -> Result<SystemFreshnessReport> {
        println!("🔍 Checking financial data freshness using SEC filing dates...");
        
        let market_data = self.check_daily_prices_direct().await?;
        
        // Step 1: Get our latest filing dates from database
        let our_latest_dates = self.get_our_latest_filing_dates().await?;
        println!("✅ Found {} S&P 500 stocks with filing metadata", our_latest_dates.len());
        
        // Step 2: Get S&P 500 stocks with CIKs
        let stocks_with_ciks = self.get_sp500_stocks_with_ciks().await?;
        
        // Step 3: Create rate-limited HTTP client
        let (client, limiter) = self.create_rate_limited_client().await?;
        
        // Step 4: Process CIKs concurrently to get SEC latest filing dates
        let sec_latest_dates = self.get_sec_latest_filing_dates(&client, &limiter, &stocks_with_ciks).await?;
        
        // Step 5: Compare our dates with SEC dates
        let freshness_results = self.compare_filing_dates(&our_latest_dates, &sec_latest_dates).await?;
        
        // Step 6: Generate freshness report
        let stale_count = freshness_results.iter().filter(|r| r.is_stale).count();
        let current_count = freshness_results.len() - stale_count;
        
        Ok(SystemFreshnessReport {
            overall_status: if stale_count == 0 { FreshnessStatus::Current } else { FreshnessStatus::Stale },
            market_data: market_data.clone(),
            financial_data: DataFreshnessStatus {
                data_source: "financial_statements".to_string(),
                status: if stale_count == 0 { FreshnessStatus::Current } else { FreshnessStatus::Stale },
                latest_data_date: freshness_results.iter()
                    .filter(|r| !r.is_stale)
                    .map(|r| r.our_latest_date.clone())
                    .flatten()
                    .max(),
                last_refresh: Some(Utc::now().to_rfc3339()),
                staleness_days: None,
                records_count: stale_count as i64,
                message: format!("{} out of {} stocks have latest SEC filings", current_count, stocks_with_ciks.len()),
                refresh_priority: if stale_count > 100 { RefreshPriority::High } else if stale_count > 50 { RefreshPriority::Medium } else { RefreshPriority::Low },
                data_summary: DataSummary {
                    date_range: None,
                    stock_count: Some(stocks_with_ciks.len() as i64),
                    data_types: vec!["SEC Filing Metadata".to_string()],
                    key_metrics: vec![
                        format!("{} stocks current", current_count),
                        format!("{} stocks stale", stale_count),
                        "SEC filing date comparison".to_string()
                    ],
                    completeness_score: Some((current_count as f32) / (stocks_with_ciks.len() as f32)),
                },
            },
            calculated_ratios: DataFreshnessStatus {
                data_source: "screening_readiness".to_string(),
                status: if stale_count == 0 { FreshnessStatus::Current } else { FreshnessStatus::Stale },
                latest_data_date: None,
                last_refresh: None,
                staleness_days: None,
                records_count: stale_count as i64,
                message: format!("Screening readiness depends on fresh financial data: {} stocks need updates", stale_count),
                refresh_priority: if stale_count > 100 { RefreshPriority::High } else { RefreshPriority::Low },
                data_summary: DataSummary {
                    date_range: None,
                    stock_count: None,
                    data_types: vec!["Piotroski F-Score".to_string(), "O'Shaughnessy Value".to_string()],
                    key_metrics: vec!["Financial data freshness required".to_string()],
                    completeness_score: None,
                },
            },
            recommendations: {
                let mut recs = vec![];
                if stale_count > 0 {
                    recs.push(RefreshRecommendation {
                        action: "refresh_data financials".to_string(),
                        reason: format!("{} stocks have stale SEC filing data", stale_count),
                        estimated_duration: "2-5 minutes".to_string(),
                        priority: if stale_count > 100 { RefreshPriority::High } else { RefreshPriority::Medium },
                    });
                }
                recs
            },
            screening_readiness: ScreeningReadiness {
                valuation_analysis: stale_count == 0,
                blocking_issues: if stale_count > 0 { 
                    vec![format!("Fresh financial data required for screening: {} stale stocks", stale_count)]
                } else { 
                    vec![] 
                },
            },
            last_check: Utc::now().to_rfc3339(),
        })
    }

    /// Get latest filing date for each S&P 500 stock from our database
    async fn get_our_latest_filing_dates(&self) -> Result<HashMap<String, Option<String>>> {
        let query = r#"
            SELECT 
                s.cik,
                MAX(i.filed_date) as latest_filed_date
            FROM stocks s
            INNER JOIN income_statements i ON s.id = i.stock_id
            WHERE s.is_sp500 = 1 AND s.cik IS NOT NULL AND i.filed_date IS NOT NULL
            GROUP BY s.cik
        "#;
        
        let rows = sqlx::query(query).fetch_all(&self.pool).await?;
        let mut results = HashMap::new();

        for row in rows {
            let cik: String = row.get("cik");
            let latest_date: Option<String> = row.get("latest_filed_date");
            results.insert(cik, latest_date);
        }
        
        Ok(results)
    }

    /// Get S&P 500 stocks with CIKs
    async fn get_sp500_stocks_with_ciks(&self) -> Result<Vec<(i64, String, String)>> {
        let query = r#"
            SELECT s.id, s.cik, s.symbol
            FROM stocks s
            WHERE s.is_sp500 = 1
                AND s.cik IS NOT NULL 
                AND s.cik != ''
                AND s.cik != 'Unknown'
            ORDER BY s.symbol
        "#;
        
        let rows = sqlx::query(query).fetch_all(&self.pool).await?;
        let mut stocks = Vec::new();
        
        for row in rows {
            let stock_id: i64 = row.get("id");
            let cik: String = row.get("cik");
            let symbol: String = row.get("symbol");
            stocks.push((stock_id, cik, symbol));
        }
        
        Ok(stocks)
    }

    /// Create rate-limited HTTP client using governor
    async fn create_rate_limited_client(&self) -> Result<(Client, Arc<RateLimiter<governor::state::direct::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>>)> {
        // Define rate limit: 10 requests per second (SEC limit)
        let quota = Quota::with_period(Duration::from_millis(100))
            .unwrap()
            .allow_burst(NonZeroU32::new(10).unwrap());
        let limiter = Arc::new(RateLimiter::direct(quota));

        let client = Client::builder()
            .user_agent("rust-stocks-edgar-client/1.0 (contact@example.com)")
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok((client, limiter))
    }

    /// Get latest filing dates from SEC Company Facts API concurrently
    async fn get_sec_latest_filing_dates(
        &self,
        client: &Client,
        limiter: &Arc<RateLimiter<governor::state::direct::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>>,
        stocks: &[(i64, String, String)]
    ) -> Result<HashMap<String, Option<String>>> {
        let semaphore = Arc::new(Semaphore::new(10)); // 10 concurrent workers
        let results = Arc::new(Mutex::new(HashMap::new()));
        
        let mut handles = Vec::new();
        
        for (_stock_id, cik, symbol) in stocks {
            let permit = semaphore.clone().acquire_owned().await?;
            let client = client.clone();
            let limiter = limiter.clone();
            let results = results.clone();
            let cik = cik.clone();
            let symbol = symbol.clone();
            
            let handle = tokio::spawn(async move {
                let _permit = permit;
                
                match get_sec_latest_filing_date(&client, &limiter, &cik).await {
                    Ok(latest_date) => {
                        let mut res = results.lock().await;
                        res.insert(cik.clone(), latest_date.clone());
                        println!("✅ {} (CIK: {}): Latest SEC filing {}", symbol, cik, 
                                latest_date.as_ref().unwrap_or(&"None".to_string()));
                    }
                    Err(e) => {
                        println!("❌ Failed {} (CIK: {}): {}", symbol, cik, e);
                        let mut res = results.lock().await;
                        res.insert(cik.clone(), None);
                    }
                }
            });
            handles.push(handle);
        }
        
        for handle in handles {
            handle.await?;
        }
        
        Ok(Arc::try_unwrap(results).unwrap().into_inner())
    }

    /// Compare our filing dates with SEC filing dates
    async fn compare_filing_dates(
        &self,
        our_dates: &HashMap<String, Option<String>>,
        sec_dates: &HashMap<String, Option<String>>
    ) -> Result<Vec<FilingFreshnessResult>> {
        let mut results = Vec::new();
        
        for (cik, our_latest) in our_dates {
            let sec_latest = sec_dates.get(cik).unwrap_or(&None);
            
            let is_stale = match (our_latest, sec_latest) {
                (Some(our), Some(sec)) => {
                    // Both have dates - compare them
                    our < sec
                }
                (Some(_), None) => {
                    // We have data but SEC API failed - assume current
                    false
                }
                (None, Some(_)) => {
                    // SEC has data but we don't - definitely stale
                    true
                }
                (None, None) => {
                    // Neither has data - assume current
                    false
                }
            };
            
            results.push(FilingFreshnessResult {
                cik: cik.clone(),
                our_latest_date: our_latest.clone(),
                sec_latest_date: sec_latest.clone(),
                is_stale,
            });
        }
        
        Ok(results)
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

}

/// Get latest filing date for a single CIK from Company Facts API
async fn get_sec_latest_filing_date(
    client: &Client,
    limiter: &Arc<RateLimiter<governor::state::direct::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>>,
    cik: &str
) -> Result<Option<String>> {
    let url = format!("https://data.sec.gov/api/xbrl/companyfacts/CIK{}.json", cik);
    
    // Rate limiting
    limiter.until_ready().await;
    
    let response = client.get(&url).send().await?;
    
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("HTTP {} for CIK {}", response.status(), cik));
    }
    
    let json: Value = response.json().await?;
    
    // Extract latest filing date from Company Facts JSON
    let latest_filing_date = extract_latest_filing_date_from_company_facts(&json)?;
    
    Ok(latest_filing_date)
}

/// Extract latest filing date from Company Facts JSON structure
fn extract_latest_filing_date_from_company_facts(json: &Value) -> Result<Option<String>> {
    let mut filing_dates = Vec::new();
    
    if let Some(facts) = json.get("facts") {
        for (_category, category_data) in facts.as_object().unwrap_or(&serde_json::Map::new()) {
            if let Some(category_obj) = category_data.as_object() {
                for (_metric, metric_data) in category_obj {
                    if let Some(units) = metric_data.get("units") {
                        if let Some(units_obj) = units.as_object() {
                            for (_unit_type, unit_data) in units_obj {
                                if let Some(data_array) = unit_data.as_array() {
                                    for data_point in data_array {
                                        if let Some(obj) = data_point.as_object() {
                                            if let Some(filed) = obj.get("filed").and_then(|v| v.as_str()) {
                                                filing_dates.push(filed.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Return the latest (most recent) filing date
    filing_dates.sort();
    Ok(filing_dates.last().cloned())
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

    pub fn get_freshness_warning_message(&self) -> String {
        let stale_components = self.get_stale_components();
        if stale_components.len() == 3 {
            "All data sources need refresh - please run latest update".to_string()
        } else if !stale_components.is_empty() {
            format!("Stale data sources: {}", stale_components.join(", "))
        } else {
            "All data sources are current".to_string()
        }
    }

    pub fn get_overall_status(&self) -> FreshnessStatus {
        self.overall_status.clone()
    }

    pub fn is_data_fresh(&self) -> bool {
        self.market_data.status == FreshnessStatus::Current && self.financial_data.status == FreshnessStatus::Current
    }

    pub fn should_show_freshness_warning(&self) -> bool {
        if self.financial_data.records_count == 0 {
            return false;
        }
        
        match self.financial_data.status {
            FreshnessStatus::Stale => true,
            FreshnessStatus::Current => false,
            FreshnessStatus::Missing => false,
            FreshnessStatus::Error => false,
        }
    }

    pub fn get_freshness_specific_message(&self) -> String {
        let stale = self.get_stale_components();
        if stale.is_empty() {
            "All data sources are current".to_string()
        } else {
            format!("Stale data sources: {}", stale.join(", "))
        }
    }
}
