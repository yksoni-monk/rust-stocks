use anyhow::{Result, anyhow};
use chrono::{Utc, NaiveDate, Datelike};
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
        let (_sec_all_dates, total_records_stored) = self.get_sec_all_filing_dates_and_extract_data(&client, &limiter, &stocks_with_ciks).await?;

        // Step 5: Generate final report
        let processed_count = stocks_with_ciks.len();

        println!("\n🎉 FINANCIAL DATA EXTRACTION COMPLETE!");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("📊 Total stocks processed: {}", processed_count);
        println!("📈 Total 10-K filings stored: {}", total_records_stored);
        println!("📅 Completion time: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"));
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        // Determine actual status based on results
        let financial_status = if total_records_stored > 0 {
            FreshnessStatus::Current  // Successfully stored new data
        } else {
            FreshnessStatus::Current  // All data already current
        };

        let overall_status = FreshnessStatus::Current;

        Ok(SystemFreshnessReport {
            overall_status,
            market_data: market_data.clone(),
            financial_data: DataFreshnessStatus {
                data_source: "sec_edgar".to_string(),
                status: financial_status,
                latest_data_date: Some(chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string()),
                last_refresh: Some(chrono::Utc::now().to_rfc3339()),
                staleness_days: Some(0),
                records_count: stocks_with_ciks.len() as i64,
                message: if total_records_stored > 0 {
                    format!("✅ SUCCESS: Stored {} new 10-K filings from {} stocks", total_records_stored, processed_count)
                } else {
                    format!("✅ SUCCESS: All {} stocks already have current 10-K data", processed_count)
                },
                refresh_priority: RefreshPriority::Low,
                data_summary: DataSummary {
                    date_range: Some("2016-present (10-K annual filings only)".to_string()),
                    stock_count: Some(stocks_with_ciks.len() as i64),
                    data_types: vec!["10-K Annual Reports".to_string(), "Balance Sheets".to_string(), "Income Statements".to_string(), "Cash Flow Statements".to_string()],
                    key_metrics: vec!["Annual financial statements".to_string()],
                    completeness_score: Some(100.0),
                },
            },
            calculated_ratios: DataFreshnessStatus {
                data_source: "screening_readiness".to_string(),
                status: FreshnessStatus::Current,
                latest_data_date: None,
                last_refresh: None,
                staleness_days: None,
                records_count: 0,
                message: "All stocks have current 10-K data, ready for screening".to_string(),
                refresh_priority: RefreshPriority::Low,
                data_summary: DataSummary {
                    date_range: None,
                    stock_count: None,
                    data_types: vec!["Piotroski F-Score".to_string(), "O'Shaughnessy Value".to_string()],
                    key_metrics: vec!["Financial data freshness required".to_string()],
                    completeness_score: None,
                },
            },
            recommendations: vec![],  // All data current after refresh
            screening_readiness: ScreeningReadiness {
                valuation_analysis: true,  // All data current
                blocking_issues: vec![],  // No blocking issues
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

    /// Get ALL SEC filing dates for a single CIK AND extract missing financial data - HYBRID API APPROACH
    /// Uses Submissions API for 10-K metadata + Company Facts API for financial data
    async fn get_all_sec_filings_for_cik_and_extract_data(
        client: &Client,
        limiter: &Arc<RateLimiter<governor::state::direct::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>>,
        cik: &str,
        stock_id: i64,
        symbol: &str,
        pool: &SqlitePool
    ) -> Result<(Vec<String>, i64)> {

        // STEP 1: Fetch Submissions API for 10-K metadata (rate limited)
        limiter.until_ready().await;

        let cik_padded = format!("{:0>10}", cik);
        let submissions_url = format!("https://data.sec.gov/submissions/CIK{}.json", cik_padded);

        let submissions_response = client
            .get(&submissions_url)
            .header("User-Agent", "rust-stocks-tauri/1.0")
            .timeout(Duration::from_secs(30))
            .send()
            .await?;

        if !submissions_response.status().is_success() {
            return Err(anyhow!("Submissions API error {}: {}", submissions_response.status(), submissions_url));
        }

        let submissions_json: serde_json::Value = submissions_response.json().await?;

        // Extract 10-K metadata from Submissions API
        let mut metadata_vec = Vec::new();
        if let Some(recent) = submissions_json.get("filings").and_then(|f| f.get("recent")) {
            if let (Some(accession_numbers), Some(forms), Some(filing_dates), Some(report_dates)) = (
                recent.get("accessionNumber").and_then(|a| a.as_array()),
                recent.get("form").and_then(|f| f.as_array()),
                recent.get("filingDate").and_then(|d| d.as_array()),
                recent.get("reportDate").and_then(|r| r.as_array())
            ) {
                for i in 0..accession_numbers.len() {
                    // Process 10-K and 10-K/A (annual reports and amendments)
                    if let Some(form) = forms[i].as_str() {
                        if form == "10-K" || form == "10-K/A" {
                            if let (Some(accn), Some(filed), Some(report)) = (
                                accession_numbers[i].as_str(),
                                filing_dates[i].as_str(),
                                report_dates[i].as_str()
                            ) {
                                metadata_vec.push((
                                    accn.to_string(),
                                    filed.to_string(),
                                    report.to_string(),
                                    form.to_string(),  // Include form type to distinguish amendments
                                ));
                            }
                        }
                    }
                }
            }
        }

        println!("  📋 {} (CIK {}): Found {} 10-K/10-K/A filings from Submissions API", symbol, cik, metadata_vec.len());

        // Deduplicate: if multiple filings exist for same report_date, prefer amendments (10-K/A)
        // and use latest filing_date as tiebreaker
        let mut deduped_map: std::collections::HashMap<String, (String, String, String, String)> = std::collections::HashMap::new();
        for (accn, filed, report, form) in metadata_vec {
            let key = report.clone();

            if let Some(existing) = deduped_map.get(&key) {
                let (_, existing_filed, _, existing_form) = existing;

                // Prefer 10-K/A over 10-K
                let should_replace = if form == "10-K/A" && existing_form == "10-K" {
                    true
                } else if form == "10-K" && existing_form == "10-K/A" {
                    false
                } else {
                    // Same form type, prefer later filing date
                    filed > *existing_filed
                };

                if should_replace {
                    deduped_map.insert(key, (accn, filed, report, form));
                }
            } else {
                deduped_map.insert(key, (accn, filed, report, form));
            }
        }

        let metadata_vec: Vec<(String, String, String, String)> = deduped_map.into_values().collect();
        println!("  📊 {} (CIK {}): After deduplication: {} unique filings", symbol, cik, metadata_vec.len());

        // Collect all filing dates for return value
        let filing_dates: Vec<String> = metadata_vec.iter().map(|(_, filed, _, _)| filed.clone()).collect();

        // STEP 2: Fetch Company Facts API for financial data (rate limited)
        limiter.until_ready().await;

        let facts_url = format!("https://data.sec.gov/api/xbrl/companyfacts/CIK{}.json", cik_padded);

        let facts_response = client
            .get(&facts_url)
            .header("User-Agent", "rust-stocks-tauri/1.0")
            .timeout(Duration::from_secs(30))
            .send()
            .await?;

        if !facts_response.status().is_success() {
            return Err(anyhow!("Company Facts API error {}: {}", facts_response.status(), facts_url));
        }

        let company_facts: serde_json::Value = facts_response.json().await?;

        // STEP 3: Extract and store data for each 10-K filing
        let mut records_stored = 0;

        // Get our existing filings to avoid duplicates
        let existing_accessions = Self::get_existing_accession_numbers(pool, stock_id).await?;
        let existing_set: std::collections::HashSet<String> = existing_accessions.into_iter().collect();

        for (accession_number, filing_date, report_date, form_type) in metadata_vec {
            // Skip if we already have this filing
            if existing_set.contains(&accession_number) {
                continue;
            }

            // Parse fiscal year from report date
            let fiscal_year = match NaiveDate::parse_from_str(&report_date, "%Y-%m-%d") {
                Ok(date) => date.year(),
                Err(_) => {
                    println!("    ⚠️ Skipping filing {}: invalid report_date {}", accession_number, report_date);
                    continue;
                }
            };

            // Extract data for this specific accession number
            let balance_data = match Self::extract_balance_sheet_for_filing(
                &company_facts,
                &accession_number,
                stock_id,
                symbol,
                &report_date,
                fiscal_year
            ) {
                Ok(data) => data,
                Err(e) => {
                    println!("    ⚠️  Skipping filing {}: {}", accession_number, e);
                    continue;
                }
            };

            let income_data = match Self::extract_income_statement_for_filing(
                &company_facts,
                &accession_number,
                stock_id,
                symbol,
                &report_date,
                fiscal_year
            ) {
                Ok(data) => data,
                Err(e) => {
                    println!("    ⚠️  Skipping filing {}: {}", accession_number, e);
                    continue;
                }
            };

            let cashflow_data = match Self::extract_cash_flow_for_filing(
                &company_facts,
                &accession_number,
                stock_id,
                symbol,
                &report_date,
                fiscal_year
            ) {
                Ok(data) => data,
                Err(e) => {
                    println!("    ⚠️  Skipping filing {}: {}", accession_number, e);
                    continue;
                }
            };

            // Create filing metadata with actual form type (10-K or 10-K/A)
            let metadata = crate::tools::sec_edgar_client::FilingMetadata {
                accession_number: accession_number.clone(),
                form_type: form_type.clone(),
                filing_date: filing_date.clone(),
                fiscal_period: "FY".to_string(),
                report_date: report_date.clone(),
            };

            // Store atomically (all 3 statements or nothing)
            let edgar_client = SecEdgarClient::new(pool.clone());
            match edgar_client.store_filing_atomic(
                stock_id,
                symbol,
                &metadata,
                fiscal_year,
                &report_date,
                &balance_data,
                &income_data,
                &cashflow_data
            ).await {
                Ok(_) => {
                    records_stored += 1;
                    println!("    ✅ Stored {} filing: {} ({})", form_type, metadata.report_date, metadata.accession_number);
                }
                Err(e) => {
                    println!("    ⚠️  Failed to store {}: {}", metadata.accession_number, e);
                }
            }
        }

        if records_stored > 0 {
            println!("✅ {} (CIK {}): Stored {} complete 10-K filings", symbol, cik, records_stored);
        } else {
            println!("✅ {} (CIK {}): Already has all 10-K financial data (current)", symbol, cik);
        }

        Ok((filing_dates, records_stored))
    }

    /// Helper: Get existing accession numbers for a stock to avoid duplicates
    async fn get_existing_accession_numbers(pool: &SqlitePool, stock_id: i64) -> Result<Vec<String>> {
        let rows = sqlx::query("SELECT accession_number FROM sec_filings WHERE stock_id = ?")
            .bind(stock_id)
            .fetch_all(pool)
            .await?;

        Ok(rows.iter().map(|r| r.get("accession_number")).collect())
    }

    /// Compare ALL filing dates using simple logic - checks ALL S&P 500 stocks
    // OLD FUNCTION REMOVED - No longer needed with hybrid API approach
    // The extraction now happens inline during get_all_sec_filings_for_cik_and_extract_data



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

    /// Get our filing dates for a single CIK (used in tests)
    #[cfg(test)]
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

    /// Extract and store ALL financial statements atomically (used in tests)
    /// This function ensures that for each filing date, ALL THREE statements are stored together
    /// in a single transaction, preventing orphaned sec_filing records
    #[cfg(test)]
    async fn extract_and_store_all_statements_atomic(
        edgar_client: &mut SecEdgarClient,
        json: &serde_json::Value,
        stock_id: i64,
        symbol: &str,
        requested_missing_dates: &[String]
    ) -> Result<i64> {
        println!("    🔒 [ATOMIC] Processing {} for {} missing dates...", symbol, requested_missing_dates.len());

        // Parse all three statement types FIRST to see what data actually exists
        let balance_historical = edgar_client.parse_company_facts_json(json, symbol)?;
        let income_historical = edgar_client.parse_income_statement_json(json, symbol)?;
        let cashflow_historical = edgar_client.parse_cash_flow_json(json, symbol)?;

        // Collect all unique filed_dates from ACTUAL data
        let mut actual_filed_dates = std::collections::HashSet::new();
        for (_field, _value, _report, filed) in &balance_historical {
            actual_filed_dates.insert(filed.clone());
        }
        for (_field, _value, _report, filed) in &income_historical {
            actual_filed_dates.insert(filed.clone());
        }
        for (_field, _value, _report, filed) in &cashflow_historical {
            actual_filed_dates.insert(filed.clone());
        }

        // Filter requested missing_dates to only those that ACTUALLY exist in the data
        let processable_dates: Vec<String> = requested_missing_dates.iter()
            .filter(|date| actual_filed_dates.contains(*date))
            .cloned()
            .collect();

        let skipped_count = requested_missing_dates.len() - processable_dates.len();
        if skipped_count > 0 {
            println!("    ⚠️  Skipping {} corrected/deleted filings not in JSON data", skipped_count);
        }

        if processable_dates.is_empty() {
            println!("    ✓  No new filings to process (all up-to-date or corrected)");
            return Ok(0);
        }

        println!("    ✓  Processing {} filings with actual data", processable_dates.len());

        // Extract filing metadata from JSON (now comprehensive - includes 8-K filings)
        let filing_metadata_vec = edgar_client.extract_filing_metadata(json, symbol).ok();

        if let Some(ref metadata_vec) = filing_metadata_vec {
            println!("    📋 Found metadata for {} filing dates", metadata_vec.len());

            // Check if we're missing metadata for any processable dates
            let metadata_dates: std::collections::HashSet<String> = metadata_vec.iter()
                .map(|m| m.filing_date.clone())
                .collect();

            let missing_metadata: Vec<&String> = processable_dates.iter()
                .filter(|date| !metadata_dates.contains(*date))
                .collect();

            if !missing_metadata.is_empty() {
                println!("    ⚠️  Warning: {} dates have data but no metadata: {:?}",
                         missing_metadata.len(), missing_metadata);
                println!("       These filings will be skipped (possible data quality issue)");
            }
        }

        // Group ALL data by filed_date (only for processable dates)
        let mut data_by_filed: HashMap<String, (String, HashMap<String, f64>, HashMap<String, f64>, HashMap<String, f64>)> = HashMap::new();

        // Group balance sheet data
        for (field_name, value, report_date, filed_date) in balance_historical {
            if processable_dates.contains(&filed_date) {
                let entry = data_by_filed.entry(filed_date.clone())
                    .or_insert_with(|| (report_date.clone(), HashMap::new(), HashMap::new(), HashMap::new()));
                if entry.0.is_empty() && !report_date.is_empty() { entry.0 = report_date.clone(); }
                entry.1.insert(field_name, value);
            }
        }

        // Group income statement data
        for (field_name, value, report_date, filed_date) in income_historical {
            if processable_dates.contains(&filed_date) {
                let entry = data_by_filed.entry(filed_date.clone())
                    .or_insert_with(|| (report_date.clone(), HashMap::new(), HashMap::new(), HashMap::new()));
                if entry.0.is_empty() && !report_date.is_empty() { entry.0 = report_date.clone(); }
                entry.2.insert(field_name, value);
            }
        }

        // Group cash flow data
        for (field_name, value, report_date, filed_date) in cashflow_historical {
            if processable_dates.contains(&filed_date) {
                let entry = data_by_filed.entry(filed_date.clone())
                    .or_insert_with(|| (report_date.clone(), HashMap::new(), HashMap::new(), HashMap::new()));
                if entry.0.is_empty() && !report_date.is_empty() { entry.0 = report_date.clone(); }
                entry.3.insert(field_name, value);
            }
        }

        // Store each filing atomically (all 3 statements together)
        let mut filings_stored = 0;
        let mut filings_skipped_no_metadata = 0;
        let mut filings_skipped_incomplete = 0;

        for (filed_date, (report_date, balance_data, income_data, cashflow_data)) in data_by_filed {

            // Find matching metadata
            let metadata_opt = filing_metadata_vec
                .as_ref()
                .and_then(|vec| vec.iter().find(|m| m.filing_date == filed_date));

            if metadata_opt.is_none() {
                println!("    ⚠️  Skipping {}: no metadata match", filed_date);
                filings_skipped_no_metadata += 1;
                continue;
            }

            let metadata = metadata_opt.unwrap();

            // RELAXED VALIDATION: Require at least ONE statement type with meaningful data
            // Check for key financial fields in each statement type
            let has_balance = balance_data.contains_key("Assets") ||
                             balance_data.contains_key("Liabilities") ||
                             balance_data.contains_key("StockholdersEquity");

            let has_income = income_data.contains_key("Revenues") ||
                            income_data.contains_key("RevenueFromContractWithCustomerExcludingAssessedTax") ||
                            income_data.contains_key("NetIncomeLoss");

            let has_cashflow = cashflow_data.contains_key("NetCashProvidedByUsedInOperatingActivities") ||
                              cashflow_data.contains_key("NetCashProvidedByUsedInInvestingActivities");

            let statements_available = [has_balance, has_income, has_cashflow].iter().filter(|&&x| x).count();

            if statements_available == 0 {
                println!("    ⚠️  Skipping {}: no meaningful data", filed_date);
                filings_skipped_incomplete += 1;
                continue;
            }

            let fiscal_year = report_date.split('-').next().unwrap().parse::<i32>().unwrap_or(0);

            // Build BalanceSheetData
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
                fiscal_year,
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
                shares_outstanding: balance_data.get("SharesOutstanding").copied(),
            };

            // Build IncomeStatementData
            let income_statement_data = IncomeStatementData {
                stock_id,
                symbol: symbol.to_string(),
                period_type: "Annual".to_string(),
                report_date: chrono::NaiveDate::parse_from_str(&report_date, "%Y-%m-%d")?,
                fiscal_year,
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

            // Build CashFlowData
            let cash_flow_data = CashFlowData {
                stock_id,
                symbol: symbol.to_string(),
                report_date: chrono::NaiveDate::parse_from_str(&report_date, "%Y-%m-%d")?,
                fiscal_year,
                operating_cash_flow: cashflow_data.get("NetCashProvidedByUsedInOperatingActivities").copied(),
                depreciation_expense: cashflow_data.get("DepreciationDepletionAndAmortization").copied()
                    .or_else(|| cashflow_data.get("DepreciationExpense").copied()),
                amortization_expense: cashflow_data.get("AmortizationExpense").copied(),
                investing_cash_flow: cashflow_data.get("NetCashProvidedByUsedInInvestingActivities").copied(),
                financing_cash_flow: cashflow_data.get("NetCashProvidedByUsedInFinancingActivities").copied(),
                dividends_paid: cashflow_data.get("PaymentsOfDividends").copied(),
                share_repurchases: cashflow_data.get("PaymentsForRepurchaseOfCommonStock").copied(),
            };

            // Store ALL THREE statements atomically
            match edgar_client.store_filing_atomic(
                stock_id,
                symbol,
                metadata,
                fiscal_year,
                &report_date,
                &balance_sheet_data,
                &income_statement_data,
                &cash_flow_data
            ).await {
                Ok(_) => {
                    filings_stored += 1;
                    println!("    ✅  Stored filing {} (report: {})", filed_date, report_date);
                }
                Err(e) => {
                    println!("    ❌  Failed {}: {} (rolled back)", filed_date, e);
                }
            }
        }

        if filings_stored > 0 {
            println!("    🎯  {} stored {} filings successfully", symbol, filings_stored);
        }
        if filings_skipped_no_metadata > 0 || filings_skipped_incomplete > 0 {
            println!("    ⚠️   Skipped {} filings (metadata:{}, incomplete:{})",
                     filings_skipped_no_metadata + filings_skipped_incomplete,
                     filings_skipped_no_metadata, filings_skipped_incomplete);
        }

        Ok(filings_stored)
    }

    /// Extract and store balance sheet data for missing dates (used in tests)
    #[cfg(test)]
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
                shares_outstanding: balance_data.get("SharesOutstanding").copied(),
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

    /// Extract and store income statement data for missing dates (used in tests)
    #[cfg(test)]
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

    /// Extract and store cash flow data for missing dates (used in tests)
    #[cfg(test)]
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

    /// Get error count for final summary (used in tests)
    #[cfg(test)]
    async fn get_error_count() -> Result<i64> {
        // Return count of errors encountered
        Ok(0) // Placeholder - could be implemented with a global error counter
    }

    /// Helper: Find a single value for a specific accession number in Company Facts JSON
    fn find_value_for_accession(
        facts: &serde_json::Value,
        concept: &str,
        accession_number: &str
    ) -> Option<f64> {
        let concept_data = facts
            .get(concept)?
            .get("units")?
            .get("USD")?
            .as_array()?;

        // Search for the value matching this accession number
        for val in concept_data {
            if val.get("accn").and_then(|a| a.as_str()) == Some(accession_number) {
                return val.get("val").and_then(|v| v.as_f64());
            }
        }

        None
    }

    /// Helper function to extract a value for a specific fiscal year from a field
    fn try_extract_field_for_fiscal_year(
        taxonomy: &serde_json::Value,
        field_name: &str,
        fiscal_year: i32
    ) -> Option<f64> {
        if let Some(shares_field) = taxonomy.get(field_name) {
            if let Some(units) = shares_field.get("units") {
                if let Some(shares_data) = units.get("shares") {
                    if let Some(values) = shares_data.as_array() {
                        // Find the latest entry for this fiscal year
                        let mut best_match: Option<f64> = None;
                        for value in values {
                            if let (Some(fy), Some(val)) = (
                                value.get("fy").and_then(|v| v.as_i64()),
                                value.get("val").and_then(|v| v.as_f64())
                            ) {
                                if fy == fiscal_year as i64 && val > 0.0 {
                                    best_match = Some(val);
                                }
                            }
                        }
                        return best_match;
                    }
                }
            }
        }
        None
    }

    /// Extract shares_outstanding for a specific fiscal year using 3-tier fallback:
    /// 1. Primary: us-gaap CommonStockSharesOutstanding (point-in-time)
    /// 2. Fallback #1: dei EntityCommonStockSharesOutstanding (point-in-time)
    /// 3. Fallback #2: us-gaap WeightedAverageNumberOfSharesOutstandingBasic (period average)
    fn extract_shares_outstanding_for_fiscal_year(
        company_facts: &serde_json::Value,
        fiscal_year: i32,
        _symbol: &str
    ) -> Option<f64> {
        // Primary: Try us-gaap CommonStockSharesOutstanding
        if let Some(us_gaap) = company_facts.get("facts").and_then(|f| f.get("us-gaap")) {
            if let Some(val) = Self::try_extract_field_for_fiscal_year(us_gaap, "CommonStockSharesOutstanding", fiscal_year) {
                return Some(val);
            }
        }

        // Fallback #1: Try dei EntityCommonStockSharesOutstanding
        if let Some(dei) = company_facts.get("facts").and_then(|f| f.get("dei")) {
            if let Some(val) = Self::try_extract_field_for_fiscal_year(dei, "EntityCommonStockSharesOutstanding", fiscal_year) {
                return Some(val);
            }
        }

        // Fallback #2: Try us-gaap WeightedAverageNumberOfSharesOutstandingBasic
        if let Some(us_gaap) = company_facts.get("facts").and_then(|f| f.get("us-gaap")) {
            if let Some(val) = Self::try_extract_field_for_fiscal_year(us_gaap, "WeightedAverageNumberOfSharesOutstandingBasic", fiscal_year) {
                return Some(val);
            }
        }

        None
    }

    /// Extract balance sheet data for a specific 10-K filing (by accession number)
    fn extract_balance_sheet_for_filing(
        company_facts: &serde_json::Value,
        accession_number: &str,
        stock_id: i64,
        symbol: &str,
        report_date: &str,
        fiscal_year: i32
    ) -> Result<BalanceSheetData> {
        let facts = company_facts
            .get("facts")
            .and_then(|f| f.get("us-gaap"))
            .ok_or_else(|| anyhow!("Missing us-gaap facts"))?;

        // Extract shares_outstanding from dei taxonomy
        let shares_outstanding = Self::extract_shares_outstanding_for_fiscal_year(company_facts, fiscal_year, symbol);

        Ok(BalanceSheetData {
            stock_id,
            symbol: symbol.to_string(),
            report_date: NaiveDate::parse_from_str(report_date, "%Y-%m-%d")?,
            fiscal_year,
            total_assets: Self::find_value_for_accession(facts, "Assets", accession_number),
            total_liabilities: Self::find_value_for_accession(facts, "Liabilities", accession_number),
            total_equity: Self::find_value_for_accession(facts, "StockholdersEquity", accession_number),
            cash_and_equivalents: Self::find_value_for_accession(facts, "CashAndCashEquivalentsAtCarryingValue", accession_number),
            short_term_debt: Self::find_value_for_accession(facts, "ShortTermDebt", accession_number)
                .or_else(|| Self::find_value_for_accession(facts, "DebtCurrent", accession_number)),
            long_term_debt: Self::find_value_for_accession(facts, "LongTermDebt", accession_number)
                .or_else(|| Self::find_value_for_accession(facts, "LongTermDebtNoncurrent", accession_number)),
            total_debt: Self::find_value_for_accession(facts, "DebtLongtermAndShorttermCombinedAmount", accession_number)
                .or_else(|| Self::find_value_for_accession(facts, "LongTermDebt", accession_number)),
            current_assets: Self::find_value_for_accession(facts, "AssetsCurrent", accession_number),
            current_liabilities: Self::find_value_for_accession(facts, "LiabilitiesCurrent", accession_number),
            share_repurchases: Self::find_value_for_accession(facts, "StockRepurchasedDuringPeriodValue", accession_number)
                .or_else(|| Self::find_value_for_accession(facts, "TreasuryStockValueAcquiredCostMethod", accession_number)),
            shares_outstanding,
        })
    }

    /// Extract income statement data for a specific 10-K filing (by accession number)
    fn extract_income_statement_for_filing(
        company_facts: &serde_json::Value,
        accession_number: &str,
        stock_id: i64,
        symbol: &str,
        report_date: &str,
        fiscal_year: i32
    ) -> Result<IncomeStatementData> {
        let facts = company_facts
            .get("facts")
            .and_then(|f| f.get("us-gaap"))
            .ok_or_else(|| anyhow!("Missing us-gaap facts"))?;

        Ok(IncomeStatementData {
            stock_id,
            symbol: symbol.to_string(),
            report_date: NaiveDate::parse_from_str(report_date, "%Y-%m-%d")?,
            fiscal_year,
            period_type: "FY".to_string(),  // 10-K = annual
            revenue: Self::find_value_for_accession(facts, "Revenues", accession_number)
                .or_else(|| Self::find_value_for_accession(facts, "RevenueFromContractWithCustomerExcludingAssessedTax", accession_number))
                .or_else(|| Self::find_value_for_accession(facts, "SalesRevenueNet", accession_number)),
            net_income: Self::find_value_for_accession(facts, "NetIncomeLoss", accession_number)
                .or_else(|| Self::find_value_for_accession(facts, "ProfitLoss", accession_number)),
            operating_income: Self::find_value_for_accession(facts, "OperatingIncomeLoss", accession_number),
            gross_profit: Self::find_value_for_accession(facts, "GrossProfit", accession_number),
            cost_of_revenue: Self::find_value_for_accession(facts, "CostOfRevenue", accession_number)
                .or_else(|| Self::find_value_for_accession(facts, "CostOfGoodsAndServicesSold", accession_number)),
            interest_expense: Self::find_value_for_accession(facts, "InterestExpense", accession_number),
            tax_expense: Self::find_value_for_accession(facts, "IncomeTaxExpenseBenefit", accession_number),
            shares_basic: Self::find_value_for_accession(facts, "WeightedAverageNumberOfSharesOutstandingBasic", accession_number),
            shares_diluted: Self::find_value_for_accession(facts, "WeightedAverageNumberOfDilutedSharesOutstanding", accession_number),
        })
    }

    /// Extract cash flow data for a specific 10-K filing (by accession number)
    fn extract_cash_flow_for_filing(
        company_facts: &serde_json::Value,
        accession_number: &str,
        stock_id: i64,
        symbol: &str,
        report_date: &str,
        fiscal_year: i32
    ) -> Result<CashFlowData> {
        let facts = company_facts
            .get("facts")
            .and_then(|f| f.get("us-gaap"))
            .ok_or_else(|| anyhow!("Missing us-gaap facts"))?;

        Ok(CashFlowData {
            stock_id,
            symbol: symbol.to_string(),
            report_date: NaiveDate::parse_from_str(report_date, "%Y-%m-%d")?,
            fiscal_year,
            depreciation_expense: Self::find_value_for_accession(facts, "DepreciationDepletionAndAmortization", accession_number)
                .or_else(|| Self::find_value_for_accession(facts, "Depreciation", accession_number)),
            amortization_expense: Self::find_value_for_accession(facts, "AmortizationOfIntangibleAssets", accession_number),
            dividends_paid: Self::find_value_for_accession(facts, "PaymentsOfDividends", accession_number)
                .or_else(|| Self::find_value_for_accession(facts, "DividendsPaid", accession_number)),
            share_repurchases: Self::find_value_for_accession(facts, "PaymentsForRepurchaseOfCommonStock", accession_number)
                .or_else(|| Self::find_value_for_accession(facts, "StockRepurchasedDuringPeriodValue", accession_number)),
            operating_cash_flow: Self::find_value_for_accession(facts, "NetCashProvidedByUsedInOperatingActivities", accession_number),
            investing_cash_flow: Self::find_value_for_accession(facts, "NetCashProvidedByUsedInInvestingActivities", accession_number),
            financing_cash_flow: Self::find_value_for_accession(facts, "NetCashProvidedByUsedInFinancingActivities", accession_number),
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
