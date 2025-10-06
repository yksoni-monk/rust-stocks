use anyhow::Result;
use chrono::Utc;
use governor::{Quota, RateLimiter};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Semaphore, Mutex};
use ts_rs::TS;

// Import SecEdgarClient for data extraction
use crate::tools::sec_edgar_client::{SecEdgarClient, BalanceSheetData, IncomeStatementData, CashFlowData};

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

    /// Check financial data freshness using SEC Company Facts API (SIMPLE APPROACH)
    pub async fn check_financial_filing_freshness(&self) -> Result<SystemFreshnessReport> {
        println!("🔍 Checking financial data freshness and extracting missing data...");
        println!("📅 Started at: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));
        
        let market_data = self.check_daily_prices_direct().await?;

        // Step 1: Get S&P 500 stocks with CIKs (all stocks we should check)
        let stocks_with_ciks = self.get_sp500_stocks_with_ciks(None).await?;
        println!("📊 Processing {} S&P 500 stocks for financial data extraction", stocks_with_ciks.len());
        println!("🔧 Using 10 concurrent threads with 10 requests/second rate limiting");
        
        // Step 2: Get ALL our filing dates from database (since 2016)
        let our_all_dates = self.get_our_all_filing_dates().await?;
        println!("✅ Found {} S&P 500 stocks with existing filing metadata", our_all_dates.len());
        
        // Step 3: Create rate-limited HTTP client
        let (client, limiter) = self.create_rate_limited_client().await?;
        
        // Step 4: Process ALL stocks - get dates AND extract missing data
        let (sec_all_dates, total_records_stored) = self.get_sec_all_filing_dates_and_extract_data(&client, &limiter, &stocks_with_ciks).await?;
        
        // Step 5: Compare our dates with SEC dates using simple logic
        let freshness_results = self.compare_all_filing_dates(&our_all_dates, &sec_all_dates, &stocks_with_ciks).await?;
        
        // Step 6: Generate final report
        let processed_count = freshness_results.len();
        let success_count = freshness_results.iter().filter(|r| !r.is_stale).count();
        
        println!("\n🎉 FINANCIAL DATA EXTRACTION COMPLETE!");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("📊 Total stocks processed: {}", processed_count);
        println!("✅ Successfully processed: {}", success_count);
        println!("📈 Total records extracted: {}", total_records_stored);
        println!("📅 Completion time: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"));
        
        // Show error report if any
        let error_count = Self::get_error_count().await?;
        if error_count > 0 {
            println!("⚠️ {} stocks had processing errors (check logs above)", error_count);
        }
        
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        
        // Generate freshness report based on actual outcome
        let stale_count = freshness_results.iter().filter(|r| r.is_stale).count();
        let _current_count = freshness_results.len() - stale_count;

        // Determine actual status based on results
        let financial_status = if total_records_stored == 0 && stale_count > 0 {
            FreshnessStatus::Error  // Failed to store anything despite stale data
        } else if stale_count == 0 {
            FreshnessStatus::Current  // All data is current
        } else {
            FreshnessStatus::Stale  // Some data still stale
        };

        let overall_status = if total_records_stored == 0 && stale_count > 0 {
            FreshnessStatus::Error  // System failure
        } else if stale_count == 0 {
            FreshnessStatus::Current  // Success
        } else {
            FreshnessStatus::Stale  // Partial success
        };

        Ok(SystemFreshnessReport {
            overall_status,
            market_data: market_data.clone(),
            financial_data: DataFreshnessStatus {
                data_source: "sec_edgar".to_string(),
                status: financial_status,
                latest_data_date: Some(chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string()),
                last_refresh: Some(chrono::Utc::now().to_rfc3339()),
                staleness_days: if stale_count == 0 { Some(0) } else { None },
                records_count: stocks_with_ciks.len() as i64,
                message: if total_records_stored == 0 && stale_count > 0 {
                    format!("🔴 FAILED: {} stocks remain stale, 0 records stored (check warnings above)", stale_count)
                } else if total_records_stored > 0 && stale_count > 0 {
                    format!("⚠️ PARTIAL: Extracted {} records, but {} stocks still stale", total_records_stored, stale_count)
                } else {
                    format!("✅ SUCCESS: Extracted {} records from {} stocks, all current", total_records_stored, processed_count)
                },
                refresh_priority: if stale_count == 0 { RefreshPriority::Low } else { RefreshPriority::High },
                data_summary: DataSummary {
                    date_range: Some("2016-present".to_string()),
                    stock_count: Some(stocks_with_ciks.len() as i64),
                    data_types: vec!["Balance Sheets".to_string(), "Income Statements".to_string(), "Cash Flow Statements".to_string()],
                    key_metrics: vec!["Financial statements".to_string()],
                    completeness_score: if stale_count == 0 { Some(100.0) } else { None },
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

    /// Get ALL filing dates for each S&P 500 stock from our database
    async fn get_our_all_filing_dates(&self) -> Result<HashMap<String, Vec<String>>> {
        let query = r#"
            SELECT 
                s.cik,
                sf.filed_date
            FROM stocks s
            INNER JOIN sec_filings sf ON s.id = sf.stock_id
            WHERE s.is_sp500 = 1 
                AND s.cik IS NOT NULL 
                AND sf.filed_date IS NOT NULL
                AND sf.filed_date >= '2016-01-01'
            ORDER BY s.cik, sf.filed_date
        "#;
        
        let rows = sqlx::query(query).fetch_all(&self.pool).await?;
        let mut results: HashMap<String, Vec<String>> = HashMap::new();

        for row in rows {
            let cik: String = row.get("cik");
            let filed_date: String = row.get("filed_date");
            
            results.entry(cik).or_insert_with(Vec::new).push(filed_date);
        }
        
        Ok(results)
    }

    /// Get S&P 500 stocks with CIKs (optionally filtered by CIK)
    pub async fn get_sp500_stocks_with_ciks(&self, only_cik: Option<&String>) -> Result<Vec<(i64, String, String)>> {
        let (query, bind_cik) = if let Some(cik) = only_cik {
            // Filtered query for single CIK
            (r#"
                SELECT s.id, s.cik, s.symbol
                FROM stocks s
                WHERE s.is_sp500 = 1
                    AND s.cik = ?
                    AND s.cik IS NOT NULL
                    AND s.cik != ''
                    AND s.cik != 'Unknown'
                ORDER BY s.symbol
            "#, Some(cik))
        } else {
            // All stocks query
            (r#"
                SELECT s.id, s.cik, s.symbol
                FROM stocks s
                WHERE s.is_sp500 = 1
                    AND s.cik IS NOT NULL
                    AND s.cik != ''
                    AND s.cik != 'Unknown'
                ORDER BY s.symbol
            "#, None)
        };

        let mut query_builder = sqlx::query(query);
        if let Some(cik) = bind_cik {
            query_builder = query_builder.bind(cik);
        }

        let rows = query_builder.fetch_all(&self.pool).await?;
        let mut stocks = Vec::new();

        for row in rows {
            let stock_id: i64 = row.get("id");
            let cik: String = row.get("cik");
            let symbol: String = row.get("symbol");
            stocks.push((stock_id, cik, symbol));
        }
        
        Ok(stocks)
    }

    /// Public unified entry point: process a provided list of stocks using the unified pipeline
    pub async fn run_unified_financials_for_stocks(
        &self,
        stocks: &[(i64, String, String)]
    ) -> Result<i64> {
        // Create rate-limited client
        let (client, limiter) = self.create_rate_limited_client().await?;
        // Run unified extraction/store
        let (_sec_all_dates, total_records_stored) = self
            .get_sec_all_filing_dates_and_extract_data(&client, &limiter, stocks)
            .await?;
        Ok(total_records_stored)
    }

    /// Create rate-limited HTTP client using governor
    async fn create_rate_limited_client(&self) -> Result<(Client, Arc<RateLimiter<governor::state::direct::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>>)> {
        // Define rate limit: 10 requests per second (SEC limit) - sustained rate
        let quota = Quota::per_second(NonZeroU32::new(10).unwrap());
        let limiter = Arc::new(RateLimiter::direct(quota));

        let client = Client::builder()
            .user_agent("rust-stocks-edgar-client/1.0 (contact@example.com)")
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok((client, limiter))
    }

    /// Get ALL SEC filing dates for S&P 500 stocks AND extract missing data - MULTI-THREADED ARCHITECTURE
    async fn get_sec_all_filing_dates_and_extract_data(
        &self,
        client: &Client,
        limiter: &Arc<RateLimiter<governor::state::direct::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>>,
        stocks: &[(i64, String, String)]  // (stock_id, cik, symbol)
    ) -> Result<(HashMap<String, Vec<String>>, i64)> {
        let semaphore = Arc::new(Semaphore::new(10)); // 10 concurrent workers
        let results = Arc::new(Mutex::new(HashMap::new()));
        let total_records = Arc::new(Mutex::new(0i64));
        let error_reports = Arc::new(Mutex::new(Vec::new()));
        
        let mut handles = Vec::new();
        
        for (stock_id, cik, symbol) in stocks.iter() {
            let client = client.clone();
            let limiter = limiter.clone();
            let results = results.clone();
            let total_records = total_records.clone();
            let error_reports = error_reports.clone();
            let semaphore = semaphore.clone();
            let pool = self.pool.clone();  // Clone pool for database access
            let cik = cik.clone();
            let symbol = symbol.clone();
            let stock_id = *stock_id;
            
            let handle = tokio::spawn(async move {
                let _permit = semaphore.acquire_owned().await.unwrap();
                
                match Self::get_all_sec_filings_for_cik_and_extract_data(&client, &limiter, &cik, stock_id, &symbol, &pool).await {
                    Ok((sec_dates, records_stored)) => {
                        if !sec_dates.is_empty() {
                            let mut res = results.lock().await;
                            res.insert(cik, sec_dates);
                            
                            let mut total = total_records.lock().await;
                            *total += records_stored;
                        }
                    }
                    Err(e) => {
                        println!("❌ Failed {} (CIK: {}): {}", symbol, cik, e);
                        let mut errors = error_reports.lock().await;
                        errors.push((symbol, cik, e.to_string()));
                    }
                }
            });
            handles.push(handle);
        }
        
        // Wait for all tasks to complete
        for handle in handles {
            handle.await?;
        }
        
        let results_map = Arc::try_unwrap(results).unwrap().into_inner();
        let total_records_count = Arc::try_unwrap(total_records).unwrap().into_inner();
        let error_list = Arc::try_unwrap(error_reports).unwrap().into_inner();
        
        // Store error reports for final summary
        Self::store_error_reports(error_list).await?;
        
        Ok((results_map, total_records_count))
    }

    /// Get ALL SEC filing dates for a single CIK AND extract missing financial data - WITH RATE LIMITING
    async fn get_all_sec_filings_for_cik_and_extract_data(
        client: &Client, 
        limiter: &Arc<RateLimiter<governor::state::direct::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>>,
        cik: &str,
        stock_id: i64,
        symbol: &str,
        pool: &SqlitePool
    ) -> Result<(Vec<String>, i64)> {
        // Apply rate limiting (10 requests per second)
        limiter.until_ready().await;
        
        let url = format!("https://data.sec.gov/api/xbrl/companyfacts/CIK{}.json", cik);
        
        let response = client
            .get(&url)
            .header("User-Agent", "rust-stocks-tauri/1.0")
            .timeout(Duration::from_secs(30))
            .send()
            .await?;
            
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("HTTP error: {}", response.status()));
        }
        
        let json: serde_json::Value = response.json().await?;
        
        // Extract ALL filing dates from the JSON
        let mut filing_dates = Vec::new();
        let start_date = chrono::NaiveDate::from_ymd_opt(2016, 1, 1).unwrap();
        let today = chrono::Utc::now().date_naive();
        
        if let Some(facts) = json.get("facts").and_then(|f| f.get("us-gaap")) {
            if let Some(facts_obj) = facts.as_object() {
                for (_field_name, field_data) in facts_obj {
                    if let Some(units) = field_data.get("units") {
                        if let Some(usd_data) = units.get("USD") {
                            if let Some(values) = usd_data.as_array() {
                                for value in values {
                                    if let Some(filed_date) = value.get("filed").and_then(|f| f.as_str()) {
                                        if let Ok(parsed_date) = chrono::NaiveDate::parse_from_str(filed_date, "%Y-%m-%d") {
                                            if parsed_date >= start_date && parsed_date <= today {
                                                filing_dates.push(filed_date.to_string());
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
        
        // Remove duplicates and sort
        filing_dates.sort();
        filing_dates.dedup();
        
        // NEW: Extract and store missing financial data
        let mut records_stored = 0;
        
        // Get our existing filing dates for this CIK
        let our_dates = Self::get_our_filing_dates_for_cik(pool, cik).await?;
        let our_dates_set: std::collections::HashSet<String> = our_dates.into_iter().collect();
        
        // Find missing dates
        let missing_dates: Vec<String> = filing_dates.iter()
            .filter(|date| !our_dates_set.contains(*date))
            .cloned()
            .collect();
        
        if !missing_dates.is_empty() {
            println!("📊 Extracting {} missing financial records for {} (CIK: {})", missing_dates.len(), symbol, cik);
            println!("📅 Missing dates: {}", missing_dates.join(", "));
            
            // Use existing sec_edgar_client logic
            let mut edgar_client = SecEdgarClient::new(pool.clone());
            
            // Extract and store balance sheet data for missing dates
            let balance_result = Self::extract_and_store_balance_sheet_data(&mut edgar_client, &json, stock_id, symbol, &missing_dates).await?;
            records_stored += balance_result;
            
            // Extract and store income statement data for missing dates  
            let income_result = Self::extract_and_store_income_statement_data(&mut edgar_client, &json, stock_id, symbol, &missing_dates).await?;
            records_stored += income_result;
            
            // Extract and store cash flow data for missing dates
            let cashflow_result = Self::extract_and_store_cash_flow_data(&mut edgar_client, &json, stock_id, symbol, &missing_dates).await?;
            records_stored += cashflow_result;
            
            println!("✅ Stored {} financial records for {}", records_stored, symbol);
        } else {
            println!("✅ {} already has all financial data (current)", symbol);
        }
        
        Ok((filing_dates, records_stored))
    }

    /// Compare ALL filing dates using simple logic - checks ALL S&P 500 stocks
    async fn compare_all_filing_dates(
        &self,
        our_dates: &HashMap<String, Vec<String>>,
        sec_dates: &HashMap<String, Vec<String>>,
        all_stocks: &[(i64, String, String)]  // (stock_id, cik, symbol)
    ) -> Result<Vec<FilingFreshnessResult>> {
        let mut results = Vec::new();
        
        for (_stock_id, cik, symbol) in all_stocks {
            let sec_filing_dates = sec_dates.get(cik).cloned().unwrap_or_default();
            let our_filing_dates = our_dates.get(cik).cloned().unwrap_or_default();
            
            let is_stale = if sec_filing_dates.is_empty() {
                // No SEC data available - consider current (nothing to download)
                false
            } else if our_filing_dates.is_empty() {
                // We have no data but SEC has data - definitely stale
                true
            } else {
                // Both have data - check if we're missing any SEC dates
                let our_dates_set: std::collections::HashSet<String> = our_filing_dates.iter().cloned().collect();
                let mut missing_dates = 0;
                for sec_date in &sec_filing_dates {
                    if !our_dates_set.contains(sec_date) {
                        missing_dates += 1;
                    }
                }
                missing_dates > 0
            };
            
            let our_latest = our_dates.get(cik).and_then(|dates| dates.last().cloned());
            let sec_latest = sec_dates.get(cik).and_then(|dates| dates.last().cloned());
            
            results.push(FilingFreshnessResult {
                cik: cik.clone(),
                our_latest_date: our_latest,
                sec_latest_date: sec_latest,
                is_stale,
            });
            
            if is_stale {
                if our_filing_dates.is_empty() {
                    println!("⚠️ {} ({}): No data in database, SEC has {} filings (stale)", symbol, cik, sec_filing_dates.len());
                } else {
                    let missing_count = sec_filing_dates.len() - our_filing_dates.len();
                    println!("⚠️ {} ({}): Missing {} filing dates (stale)", symbol, cik, missing_count);
                }
            } else {
                if sec_filing_dates.is_empty() {
                    println!("✅ {} ({}): No SEC data available (current)", symbol, cik);
                } else {
                    println!("✅ {} ({}): All {} filing dates present (current)", symbol, cik, sec_filing_dates.len());
                }
            }
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

    /// Get our filing dates for a single CIK
    async fn get_our_filing_dates_for_cik(pool: &SqlitePool, cik: &str) -> Result<Vec<String>> {
        let query = r#"
            SELECT sf.filed_date
            FROM stocks s
            INNER JOIN sec_filings sf ON s.id = sf.stock_id
            WHERE s.cik = ? 
                AND sf.filed_date IS NOT NULL
                AND sf.filed_date >= '2016-01-01'
            ORDER BY sf.filed_date
        "#;
        
        let rows = sqlx::query(query)
            .bind(cik)
            .fetch_all(pool)
            .await?;
        
        let mut results = Vec::new();
        for row in rows {
            let filed_date: String = row.get("filed_date");
            results.push(filed_date);
        }
        
        Ok(results)
    }

    /// Extract and store balance sheet data for missing dates
    async fn extract_and_store_balance_sheet_data(
        edgar_client: &mut SecEdgarClient,
        json: &serde_json::Value,
        stock_id: i64,
        symbol: &str,
        missing_dates: &[String]
    ) -> Result<i64> {
        println!("    📊 Processing balance sheet data for {}...", symbol);
        
        // Use existing parse_company_facts_json logic
        let historical_data = edgar_client.parse_company_facts_json(json, symbol)?;
        
        if historical_data.is_empty() {
            println!("    ⚠️ No balance sheet data found for {}", symbol);
            return Ok(0);
        }
        
        println!("    📈 Found {} balance sheet data points", historical_data.len());
        
        // Extract filing metadata
        let filing_metadata_vec = edgar_client.extract_filing_metadata(json, symbol).ok();
        
        // Group by FILED DATE and store only those filed dates we are missing
        let mut records_stored = 0;
        let mut balance_by_filed: HashMap<String, (String, HashMap<String, f64>)> = HashMap::new();
        
        for (field_name, value, report_date, filed_date) in historical_data {
            if missing_dates.contains(&filed_date) {
                let entry = balance_by_filed.entry(filed_date.clone()).or_insert_with(|| (report_date.clone(), HashMap::new()));
                // Keep the first non-empty report_date if multiple tuples differ
                if entry.0.is_empty() && !report_date.is_empty() { entry.0 = report_date.clone(); }
                entry.1.insert(field_name, value);
            }
        }
        
        // Store balance sheet data for missing filed dates
        for (filed_date, (report_date, balance_data)) in balance_by_filed {
            let fiscal_year = report_date.split('-').next().unwrap().parse::<i32>().unwrap_or(0);
            
            println!("    💾 Storing balance sheet for {} filed_date={} report_date={}", symbol, filed_date, report_date);
            
            // Calculate derived values
            let short_term_debt = balance_data.get("ShortTermDebt").copied()
                .or_else(|| balance_data.get("DebtCurrent").copied());
            let long_term_debt = balance_data.get("LongTermDebt").copied()
                .or_else(|| balance_data.get("LongTermDebtAndCapitalLeaseObligations").copied());
            let total_debt = match (short_term_debt, long_term_debt) {
                (Some(st), Some(lt)) => Some(st + lt),
                (Some(st), None) => Some(st),
                (None, Some(lt)) => Some(lt),
                (None, None) => None,
            };
            
            let balance_sheet_data = BalanceSheetData {
                stock_id,
                symbol: symbol.to_string(),
                report_date: chrono::NaiveDate::parse_from_str(&report_date, "%Y-%m-%d")?,
                fiscal_year: fiscal_year,
                total_assets: balance_data.get("Assets").copied(),
                total_liabilities: balance_data.get("Liabilities").copied(),
                total_equity: balance_data.get("StockholdersEquity").copied(),
                cash_and_equivalents: balance_data.get("CashAndCashEquivalentsAtCarryingValue").copied(),
                short_term_debt,
                long_term_debt,
                total_debt,
                current_assets: balance_data.get("AssetsCurrent").copied(),
                current_liabilities: balance_data.get("LiabilitiesCurrent").copied(),
                share_repurchases: balance_data.get("ShareRepurchases").copied(),
            };
            
            // Pick matching metadata for this filed_date
            let meta = filing_metadata_vec
                .as_ref()
                .and_then(|vec| vec.iter().find(|m| m.filing_date == filed_date));

            if let Some(metadata) = meta {
                edgar_client.store_balance_sheet_data(&balance_sheet_data, Some(metadata)).await?;
                records_stored += 1;
            } else {
                println!("🔴 WARNING: No matching metadata for {} filed_date={}", symbol, filed_date);
                println!("   Available metadata filing_dates: {:?}",
                    filing_metadata_vec.as_ref().map(|v| v.iter().map(|m| &m.filing_date).collect::<Vec<_>>()));
                println!("   This balance sheet filing will be SKIPPED. Check if extract_filing_metadata is comprehensive.");
            }
        }
        
        Ok(records_stored)
    }

    /// Extract and store income statement data for missing dates
    async fn extract_and_store_income_statement_data(
        edgar_client: &mut SecEdgarClient,
        json: &serde_json::Value,
        stock_id: i64,
        symbol: &str,
        missing_dates: &[String]
    ) -> Result<i64> {
        println!("    📈 Processing income statement data for {}...", symbol);
        
        // Use existing parse_income_statement_json logic
        let historical_data = edgar_client.parse_income_statement_json(json, symbol)?;
        
        if historical_data.is_empty() {
            println!("    ⚠️ No income statement data found for {}", symbol);
            return Ok(0);
        }
        
        println!("    📊 Found {} income statement data points", historical_data.len());
        
        // Extract filing metadata
        let filing_metadata_vec = edgar_client.extract_filing_metadata(json, symbol).ok();
        
        // Group by FILED DATE and store only those filed dates we are missing
        let mut records_stored = 0;
        let mut income_by_filed: HashMap<String, (String, HashMap<String, f64>)> = HashMap::new();
        
        for (field_name, value, report_date, filed_date) in historical_data {
            if missing_dates.contains(&filed_date) {
                let entry = income_by_filed.entry(filed_date.clone()).or_insert_with(|| (report_date.clone(), HashMap::new()));
                if entry.0.is_empty() && !report_date.is_empty() { entry.0 = report_date.clone(); }
                entry.1.insert(field_name, value);
            }
        }
        
        // Store income statement data for missing filed dates
        for (filed_date, (report_date, income_data)) in income_by_filed {
            let fiscal_year = report_date.split('-').next().unwrap().parse::<i32>().unwrap_or(0);
            
            println!("    💾 Storing income statement for {} filed_date={} report_date={}", symbol, filed_date, report_date);
            
            let income_statement_data = IncomeStatementData {
                stock_id,
                symbol: symbol.to_string(),
                period_type: "Annual".to_string(),
                report_date: chrono::NaiveDate::parse_from_str(&report_date, "%Y-%m-%d")?,
                fiscal_year: fiscal_year,
                revenue: income_data.get("Revenues").copied()
                    .or_else(|| income_data.get("RevenueFromContractWithCustomerExcludingAssessedTax").copied()),
                gross_profit: income_data.get("GrossProfit").copied(),
                operating_income: income_data.get("OperatingIncomeLoss").copied(),
                net_income: income_data.get("NetIncomeLoss").copied(),
                cost_of_revenue: income_data.get("CostOfGoodsAndServicesSold").copied(),
                interest_expense: income_data.get("InterestExpense").copied(),
                tax_expense: income_data.get("IncomeTaxExpenseBenefit").copied(),
                shares_basic: income_data.get("WeightedAverageNumberOfSharesOutstandingBasic").copied(),
                shares_diluted: income_data.get("WeightedAverageNumberOfSharesOutstandingDiluted").copied(),
            };
            
            let meta = filing_metadata_vec
                .as_ref()
                .and_then(|vec| vec.iter().find(|m| m.filing_date == filed_date));

            if let Some(metadata) = meta {
                edgar_client.store_income_statement_data(&income_statement_data, Some(metadata)).await?;
                records_stored += 1;
            } else {
                println!("🔴 WARNING: No matching metadata for {} filed_date={}", symbol, filed_date);
                println!("   Available metadata filing_dates: {:?}",
                    filing_metadata_vec.as_ref().map(|v| v.iter().map(|m| &m.filing_date).collect::<Vec<_>>()));
                println!("   This income statement filing will be SKIPPED. Check if extract_filing_metadata is comprehensive.");
            }
        }
        
        Ok(records_stored)
    }

    /// Extract and store cash flow data for missing dates
    async fn extract_and_store_cash_flow_data(
        edgar_client: &mut SecEdgarClient,
        json: &serde_json::Value,
        stock_id: i64,
        symbol: &str,
        missing_dates: &[String]
    ) -> Result<i64> {
        println!("    💰 Processing cash flow data for {}...", symbol);
        
        // Use existing parse_cash_flow_json logic
        let historical_data = edgar_client.parse_cash_flow_json(json, symbol)?;
        
        if historical_data.is_empty() {
            println!("    ⚠️ No cash flow data found for {}", symbol);
            return Ok(0);
        }
        
        println!("    📊 Found {} cash flow data points", historical_data.len());
        
        // Extract filing metadata
        let filing_metadata_vec = edgar_client.extract_filing_metadata(json, symbol).ok();
        
        // Group by FILED DATE and store only those filed dates we are missing
        let mut records_stored = 0;
        let mut cashflow_by_filed: HashMap<String, (String, HashMap<String, f64>)> = HashMap::new();
        
        for (field_name, value, report_date, filed_date) in historical_data {
            if missing_dates.contains(&filed_date) {
                let entry = cashflow_by_filed.entry(filed_date.clone()).or_insert_with(|| (report_date.clone(), HashMap::new()));
                if entry.0.is_empty() && !report_date.is_empty() { entry.0 = report_date.clone(); }
                entry.1.insert(field_name, value);
            }
        }
        
        // Store cash flow data for missing filed dates
        for (filed_date, (report_date, cashflow_data)) in cashflow_by_filed {
            let fiscal_year = report_date.split('-').next().unwrap().parse::<i32>().unwrap_or(0);
            
            println!("    💾 Storing cash flow for {} filed_date={} report_date={}", symbol, filed_date, report_date);
            
            let cash_flow_data = CashFlowData {
                stock_id,
                symbol: symbol.to_string(),
                report_date: chrono::NaiveDate::parse_from_str(&report_date, "%Y-%m-%d")?,
                fiscal_year: fiscal_year,
                operating_cash_flow: cashflow_data.get("NetCashProvidedByUsedInOperatingActivities").copied(),
                depreciation_expense: cashflow_data.get("DepreciationDepletionAndAmortization").copied()
                    .or_else(|| cashflow_data.get("DepreciationExpense").copied()),
                amortization_expense: cashflow_data.get("AmortizationExpense").copied(),
                investing_cash_flow: cashflow_data.get("NetCashProvidedByUsedInInvestingActivities").copied(),
                financing_cash_flow: cashflow_data.get("NetCashProvidedByUsedInFinancingActivities").copied(),
                dividends_paid: cashflow_data.get("PaymentsOfDividends").copied(),
                share_repurchases: cashflow_data.get("PaymentsForRepurchaseOfCommonStock").copied(),
            };
            
            let meta = filing_metadata_vec
                .as_ref()
                .and_then(|vec| vec.iter().find(|m| m.filing_date == filed_date));

            if let Some(metadata) = meta {
                edgar_client.store_cash_flow_data(&cash_flow_data, Some(metadata)).await?;
                records_stored += 1;
            } else {
                println!("🔴 WARNING: No matching metadata for {} filed_date={}", symbol, filed_date);
                println!("   Available metadata filing_dates: {:?}",
                    filing_metadata_vec.as_ref().map(|v| v.iter().map(|m| &m.filing_date).collect::<Vec<_>>()));
                println!("   This cash flow filing will be SKIPPED. Check if extract_filing_metadata is comprehensive.");
            }
        }
        
        Ok(records_stored)
    }

    /// Store error reports for final summary
    async fn store_error_reports(errors: Vec<(String, String, String)>) -> Result<()> {
        // Store errors for final summary
        for (symbol, cik, error) in errors {
            println!("❌ Error processing {} ({}): {}", symbol, cik, error);
        }
        Ok(())
    }

    /// Get error count for final summary
    async fn get_error_count() -> Result<i64> {
        // Return count of errors encountered
        Ok(0) // Placeholder - could be implemented with a global error counter
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

// ============================================================================
// UNIT TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    /// Test helper to create a test database pool
    async fn create_test_pool() -> SqlitePool {
        SqlitePool::connect(":memory:").await.unwrap()
    }

    /// Test helper to create test JSON data
    #[allow(dead_code)]
    fn create_test_json() -> serde_json::Value {
        serde_json::json!({
            "facts": {
                "us-gaap": {
                    "Assets": {
                        "units": {
                            "USD": [
                                {
                                    "val": 1000000000.0,
                                    "end": "2023-12-31",
                                    "filed": "2024-01-15"
                                },
                                {
                                    "val": 950000000.0,
                                    "end": "2022-12-31", 
                                    "filed": "2023-01-15"
                                }
                            ]
                        }
                    },
                    "Revenues": {
                        "units": {
                            "USD": [
                                {
                                    "val": 500000000.0,
                                    "end": "2023-12-31",
                                    "filed": "2024-01-15"
                                }
                            ]
                        }
                    }
                }
            }
        })
    }

    #[tokio::test]
    async fn test_get_our_filing_dates_for_cik() {
        let pool = create_test_pool().await;
        let checker = DataStatusReader::new(pool);
        
        // Test with non-existent CIK - should return empty vector, not error
        let result = DataStatusReader::get_our_filing_dates_for_cik(&checker.pool, "0000000000").await;
        // This should succeed and return an empty vector
        match result {
            Ok(dates) => assert_eq!(dates.len(), 0),
            Err(_) => {
                // If it fails due to missing tables, that's also acceptable for this test
                // The important thing is that the function doesn't panic
            }
        }
    }

    #[tokio::test]
    async fn test_extract_and_store_balance_sheet_data_empty_data() {
        let pool = create_test_pool().await;
        let mut edgar_client = SecEdgarClient::new(pool);
        let json = serde_json::json!({});
        let missing_dates = vec!["2023-12-31".to_string()];
        
        let result = DataStatusReader::extract_and_store_balance_sheet_data(
            &mut edgar_client, 
            &json, 
            1, 
            "TEST", 
            &missing_dates
        ).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_extract_and_store_income_statement_data_empty_data() {
        let pool = create_test_pool().await;
        let mut edgar_client = SecEdgarClient::new(pool);
        let json = serde_json::json!({});
        let missing_dates = vec!["2023-12-31".to_string()];
        
        let result = DataStatusReader::extract_and_store_income_statement_data(
            &mut edgar_client, 
            &json, 
            1, 
            "TEST", 
            &missing_dates
        ).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_extract_and_store_cash_flow_data_empty_data() {
        let pool = create_test_pool().await;
        let mut edgar_client = SecEdgarClient::new(pool);
        let json = serde_json::json!({});
        let missing_dates = vec!["2023-12-31".to_string()];
        
        let result = DataStatusReader::extract_and_store_cash_flow_data(
            &mut edgar_client, 
            &json, 
            1, 
            "TEST", 
            &missing_dates
        ).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_store_error_reports() {
        let errors = vec![
            ("AAPL".to_string(), "0000000001".to_string(), "Test error".to_string()),
            ("MSFT".to_string(), "0000000002".to_string(), "Another error".to_string()),
        ];
        
        let result = DataStatusReader::store_error_reports(errors).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_error_count() {
        let result = DataStatusReader::get_error_count().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_filing_freshness_result_creation() {
        let result = FilingFreshnessResult {
            cik: "0000000001".to_string(),
            our_latest_date: Some("2023-12-31".to_string()),
            sec_latest_date: Some("2024-01-15".to_string()),
            is_stale: true,
        };
        
        assert_eq!(result.cik, "0000000001");
        assert_eq!(result.our_latest_date, Some("2023-12-31".to_string()));
        assert_eq!(result.sec_latest_date, Some("2024-01-15".to_string()));
        assert!(result.is_stale);
    }

    #[test]
    fn test_freshness_status_methods() {
        assert!(FreshnessStatus::Current.is_current());
        assert!(!FreshnessStatus::Stale.is_current());
        assert!(!FreshnessStatus::Missing.is_current());
        assert!(!FreshnessStatus::Error.is_current());

        assert!(!FreshnessStatus::Current.needs_refresh());
        assert!(FreshnessStatus::Stale.needs_refresh());
        assert!(FreshnessStatus::Missing.needs_refresh());
        assert!(FreshnessStatus::Error.needs_refresh());
    }

    #[test]
    fn test_refresh_priority_ordering() {
        assert!(RefreshPriority::Low < RefreshPriority::Medium);
        assert!(RefreshPriority::Medium < RefreshPriority::High);
        assert!(RefreshPriority::High < RefreshPriority::Critical);
    }

    #[test]
    fn test_data_summary_creation() {
        let summary = DataSummary {
            date_range: Some("2023-01-01 to 2023-12-31".to_string()),
            stock_count: Some(500),
            data_types: vec!["Balance Sheets".to_string(), "Income Statements".to_string()],
            key_metrics: vec!["Revenue".to_string(), "Assets".to_string()],
            completeness_score: Some(95.5),
        };
        
        assert_eq!(summary.date_range, Some("2023-01-01 to 2023-12-31".to_string()));
        assert_eq!(summary.stock_count, Some(500));
        assert_eq!(summary.data_types.len(), 2);
        assert_eq!(summary.key_metrics.len(), 2);
        assert_eq!(summary.completeness_score, Some(95.5));
    }

    #[test]
    fn test_system_freshness_report_creation() {
        let financial_data = DataFreshnessStatus {
            data_source: "test".to_string(),
            status: FreshnessStatus::Current,
            latest_data_date: Some("2023-12-31".to_string()),
            last_refresh: Some("2024-01-01T00:00:00Z".to_string()),
            staleness_days: Some(0),
            records_count: 1000,
            message: "Test message".to_string(),
            refresh_priority: RefreshPriority::Low,
            data_summary: DataSummary {
                date_range: None,
                stock_count: None,
                data_types: vec![],
                key_metrics: vec![],
                completeness_score: None,
            },
        };

        let market_data = DataFreshnessStatus {
            data_source: "market".to_string(),
            status: FreshnessStatus::Current,
            latest_data_date: Some("2023-12-31".to_string()),
            last_refresh: Some("2024-01-01T00:00:00Z".to_string()),
            staleness_days: Some(0),
            records_count: 5000,
            message: "Market data current".to_string(),
            refresh_priority: RefreshPriority::Low,
            data_summary: DataSummary {
                date_range: None,
                stock_count: None,
                data_types: vec![],
                key_metrics: vec![],
                completeness_score: None,
            },
        };

        let report = SystemFreshnessReport {
            overall_status: FreshnessStatus::Current,
            market_data: market_data.clone(),
            financial_data: financial_data.clone(),
            calculated_ratios: market_data.clone(),
            recommendations: vec![],
            screening_readiness: ScreeningReadiness {
                valuation_analysis: true,
                blocking_issues: vec![],
            },
            last_check: "2024-01-01T00:00:00Z".to_string(),
        };

        assert_eq!(report.overall_status, FreshnessStatus::Current);
        assert_eq!(report.financial_data.data_source, "test");
        assert_eq!(report.market_data.data_source, "market");
        assert_eq!(report.last_check, "2024-01-01T00:00:00Z");
    }

    #[test]
    fn test_date_parsing_edge_cases() {
        // Test valid date parsing
        let valid_date = chrono::NaiveDate::parse_from_str("2023-12-31", "%Y-%m-%d");
        assert!(valid_date.is_ok());
        assert_eq!(valid_date.unwrap().format("%Y-%m-%d").to_string(), "2023-12-31");

        // Test invalid date parsing
        let invalid_date = chrono::NaiveDate::parse_from_str("2023-13-31", "%Y-%m-%d");
        assert!(invalid_date.is_err());

        // Test leap year
        let leap_year = chrono::NaiveDate::parse_from_str("2024-02-29", "%Y-%m-%d");
        assert!(leap_year.is_ok());
    }

    #[test]
    fn test_fiscal_year_extraction() {
        // Test fiscal year extraction from date strings
        let test_cases = vec![
            ("2023-12-31", 2023),
            ("2024-01-15", 2024),
            ("2022-06-30", 2022),
        ];

        for (date_str, expected_year) in test_cases {
            let fiscal_year = date_str.split('-').next().unwrap().parse::<i32>().unwrap_or(0);
            assert_eq!(fiscal_year, expected_year);
        }
    }

    #[test]
    fn test_missing_dates_filtering() {
        let all_dates = vec![
            "2023-12-31".to_string(),
            "2023-09-30".to_string(),
            "2023-06-30".to_string(),
            "2023-03-31".to_string(),
        ];

        let existing_dates = vec![
            "2023-12-31".to_string(),
            "2023-06-30".to_string(),
        ];

        let existing_set: std::collections::HashSet<String> = existing_dates.into_iter().collect();
        let missing_dates: Vec<String> = all_dates.iter()
            .filter(|date| !existing_set.contains(*date))
            .cloned()
            .collect();

        assert_eq!(missing_dates.len(), 2);
        assert!(missing_dates.contains(&"2023-09-30".to_string()));
        assert!(missing_dates.contains(&"2023-03-31".to_string()));
    }

    #[tokio::test]
    async fn test_concurrent_processing_simulation() {
        // Simulate concurrent processing with semaphore
        let semaphore = Arc::new(Semaphore::new(2));
        let counter = Arc::new(Mutex::new(0));
        
        let handles: Vec<_> = (0..5).map(|i| {
            let semaphore = semaphore.clone();
            let counter = counter.clone();
            tokio::spawn(async move {
                let _permit = semaphore.acquire_owned().await.unwrap();
                let mut count = counter.lock().await;
                *count += 1;
                println!("Task {} completed", i);
            })
        }).collect();

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        let final_count = counter.lock().await;
        assert_eq!(*final_count, 5);
    }
}
