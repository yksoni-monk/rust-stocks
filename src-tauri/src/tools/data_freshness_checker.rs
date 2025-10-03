use anyhow::Result;
use chrono::{Utc, NaiveDate};
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};
use ts_rs::TS;
// Removed unused imports to avoid warnings
// Removed unused SecEdgarClient import

// SEC Bulk Submissions data structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecSubmissionData {
    pub cik: String,
    pub entity_name: String,
    pub tickers: Vec<String>,
    pub filings: SecFilingsData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecFilingsData {
    pub recent: SecRecentFilings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecRecentFilings {
    #[serde(rename = "accessionNumber")]
    pub accession_number: Vec<String>,
    #[serde(rename = "filingDate")]
    pub filing_date: Vec<String>,
    #[serde(rename = "reportDate")]
    pub report_date: Vec<String>,
    pub form: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FreshnessResult {
    pub cik: String,
    pub stock_id: i64,
    pub symbol: String,
    pub latest_filing_date: Option<NaiveDate>,
    pub database_latest_date: Option<NaiveDate>,
    pub is_stale: bool,
    pub missing_filing_dates: Vec<NaiveDate>,
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
        self.check_financial_filing_freshness(&self.pool).await
    }

    /// Check financial data freshness using SEC Bulk Submissions (NEW FAST APPROACH)
    pub async fn check_financial_filing_freshness(&self, pool: &SqlitePool) -> Result<SystemFreshnessReport> {
        println!("🔍 Checking financial data freshness using SEC Bulk Submissions...");
        
        let market_data = self.check_daily_prices_direct().await?;
        
        // Step 1: Download SEC bulk submissions data  
        let submissions_data = self.download_bulk_submissions_with_cleanup().await?;
        
        // Step 2: Extract submission JSON files
        let extracted_files = self.extract_submissions_json_files_from_data(submissions_data).await?;
        
        // Step 3: Get S&P 500 stocks with CIKs from database
        let stocks_with_ciks = self.get_all_sp500_stocks_with_cik(pool).await?;
        println!("✅ Found {} S&P 500 stocks with CIKs", stocks_with_ciks.len());
        
        // Step 4: Get latest filing dates from database for all CIKs
        let database_filing_dates = self.get_database_latest_filing_dates(pool, &stocks_with_ciks).await?;
        
        // Step 5: Compare SEC submission dates with database dates concurrently
        let freshness_results = self.compare_submission_dates_with_database(
            &extracted_files,
            &stocks_with_ciks,
            &database_filing_dates,
        ).await?;
        
        // Step 6: Generate freshness report
        let stale_count = freshness_results.iter().filter(|r| r.is_stale).count();
        
        Ok(SystemFreshnessReport {
            overall_status: if stale_count == 0 { FreshnessStatus::Current } else { FreshnessStatus::Stale },
            market_data: market_data.clone(),
            financial_data: DataFreshnessStatus {
                data_source: "financial_statements".to_string(),
                status: if stale_count == 0 { FreshnessStatus::Current } else { FreshnessStatus::Stale },
                latest_data_date: None, // TODO: Add latest filing date
                last_refresh: Some(Utc::now().to_rfc3339()),
                staleness_days: None,
                records_count: stale_count as i64,
                message: format!("{} out of {} stocks need SEC updates - run 'cargo run --bin refresh_data financials'", stale_count, stocks_with_ciks.len()),
                refresh_priority: if stale_count > 100 { RefreshPriority::High } else if stale_count > 50 { RefreshPriority::Medium } else { RefreshPriority::Low },
                data_summary: DataSummary {
                    date_range: None,
                    stock_count: Some(stocks_with_ciks.len() as i64),
                    data_types: vec!["SEC Bulk Submissions".to_string(), "Financial Filings".to_string()],
                        key_metrics: vec![format!("{} stale stocks", stale_count), format!("{} stocks current", stocks_with_ciks.len() - stale_count), "Single bulk download + fast local processing".to_string()],
                    completeness_score: Some(((stocks_with_ciks.len() - stale_count) as f32) / (stocks_with_ciks.len() as f32)),
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
    
    /// Get all S&P 500 stocks with CIKs for bulk submissions checking
    async fn get_all_sp500_stocks_with_cik(&self, pool: &SqlitePool) -> Result<Vec<(i64, String, String)>> {
        let query = r#"
            SELECT s.id, s.cik, s.symbol
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
            let symbol: String = row.get("symbol");
            stocks.push((stock_id, cik, symbol));
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

    /// Download bulk submissions zip file with cleanup
    async fn download_bulk_submissions_with_cleanup(&self) -> Result<Vec<u8>> {
        let cache_dir = std::env::temp_dir().join("rust-stocks-sec");
        tokio::fs::create_dir_all(&cache_dir).await?;
        
        let zip_cache_path = cache_dir.join("submissions.zip");
        
        // Cleanup: Remove old zip file first to prevent disk storage overflow
        if zip_cache_path.exists() {
            tokio::fs::remove_file(&zip_cache_path).await?;
            println!("🗑️ Removed old submissions.zip to free disk space");
        }
        
        let url = "https://www.sec.gov/Archives/edgar/daily-index/bulkdata/submissions.zip";
        println!("🌐 Downloading bulk submissions from SEC...");
        
        let client = reqwest::Client::builder()
            .user_agent("rust-stocks-edgar-client/1.0 (contact@example.com)")
            .timeout(std::time::Duration::from_secs(300)) // 5 minute timeout for large downloads
            .build()?;
        
        // Add small delay to respect SEC rate limiting
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        let response = client.get(url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to download submissions.zip: {} - {}", response.status(), response.status().canonical_reason().unwrap_or("Unknown reason")));
        }
        
        let zip_buf = response.bytes().await?.to_vec();
        
        println!("📦 Downloaded bulk submissions successfully: {} MB", zip_buf.len() / 1024 / 1024);
        Ok(zip_buf)
    }

    /// Extract submission JSON files from zip data
    async fn extract_submissions_json_files_from_data(&self, zip_data: Vec<u8>) -> Result<std::collections::HashMap<String, std::path::PathBuf>> {
        use zip::ZipArchive;
        use std::io::{Cursor, Read};

        println!("📂 Extracting submission JSON files from zip...");
        
        let cache_dir = std::env::temp_dir().join("rust-stocks-sec");
        tokio::fs::create_dir_all(&cache_dir).await?;
        
        // Process zip file synchronously to avoid Send issues
        let extracted_files = tokio::task::spawn_blocking(move || {
            let cursor = Cursor::new(zip_data);
            let mut archive = ZipArchive::new(cursor)?;
            let mut files = std::collections::HashMap::new();
            
            for i in 0..archive.len() {
                let mut file = archive.by_index(i)?;
                let file_path = file.name().to_string();
                
                // Only extract JSON files
                if file_path.ends_with(".json") && file_path.contains("submissions") {
                    let output_path = cache_dir.join(&file_path);
                    
                    // Create parent directories if needed
                    if let Some(parent) = output_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    
                    let mut contents = Vec::new();
                    file.read_to_end(&mut contents)?;
                    
                    std::fs::write(&output_path, contents)?;
                    
                    // Extract CIK from filename (CIK0000320193.json -> 0000320193)
                    let path_buf = std::path::PathBuf::from(file_path);
                    if let Some(filename) = path_buf.file_name().and_then(|n| n.to_str()) {
                        if filename.starts_with("CIK") && filename.ends_with(".json") {
                            if let Some(cik) = filename.strip_prefix("CIK").and_then(|s| s.strip_suffix(".json")) {
                                files.insert(cik.to_string(), output_path);
                            }
                        }
                    }
                }
            }
            
            Ok::<std::collections::HashMap<String, std::path::PathBuf>, anyhow::Error>(files)
        }).await??;
        
        println!("✅ Extracted {} submission JSON files", extracted_files.len());
        Ok(extracted_files)
    }

    /// Get latest filing dates from database for all CIKs
    async fn get_database_latest_filing_dates(&self, pool: &SqlitePool, stocks: &[(i64, String, String)]) -> Result<std::collections::HashMap<String, Option<chrono::NaiveDate>>> {
        println!("🗄️ Fetching latest filing dates from database...");
        
        let mut db_dates = std::collections::HashMap::new();
        
        // Get CIKs as string slice for query
        let ciks: Vec<String> = stocks.iter().map(|(_, cik, _)| cik.clone()).collect();
        let cik_placeholders = format!("?{}", ",? ".repeat(ciks.len() - 1));
        
        let query = format!(r#"
            WITH latest_dates AS (
                SELECT DISTINCT stock_id, MAX(filed_date) as latest_date
                FROM (
                    SELECT stock_id, filed_date FROM income_statements WHERE stock_id IN ({}) AND filed_date IS NOT NULL
                    UNION ALL
                    SELECT stock_id, filed_date FROM balance_sheets WHERE stock_id IN ({}) AND filed_date IS NOT NULL
                    UNION ALL  
                    SELECT stock_id, filed_date FROM cash_flow_statements WHERE stock_id IN ({}) AND filed_date IS NOT NULL
                )
                GROUP BY stock_id
            )
            SELECT stock_id, latest_date FROM latest_dates
        "#, &cik_placeholders, &cik_placeholders, &cik_placeholders);
        
        // Prepare query parameters - use stock_ids instead of CIKs
        let mut params = vec![];
        let stock_ids: Vec<i64> = stocks.iter().map(|(stock_id, _, _)| *stock_id).collect();
        for &stock_id in &stock_ids {
            params.push(stock_id);
        }
        for &stock_id in &stock_ids {
            params.push(stock_id);
        }
        for &stock_id in &stock_ids {
            params.push(stock_id);
        }
        
        // Execute query
        let mut query_builder = sqlx::query(&query);
        for param in params {
            query_builder = query_builder.bind(param);
        }
        
        let rows = query_builder.fetch_all(pool).await?;
        
        // Map stock_id back to CIK for the results
        let mut stock_id_to_cik: std::collections::HashMap<i64, String> = stocks.iter().map(|(stock_id, cik, _)| (*stock_id, cik.clone())).collect();
        
        for row in rows {
            let stock_id: i64 = row.get("stock_id");
            let latest_date: Option<chrono::NaiveDate> = row.get("latest_date");
            if let Some(cik) = stock_id_to_cik.get(&stock_id) {
                db_dates.insert(cik.clone(), latest_date);
            }
        }
        
        // Fill missing entries with None
        for (_, cik, _) in stocks {
            if !db_dates.contains_key(cik) {
                db_dates.insert(cik.clone(), None);
            }
        }
        
        println!("✅ Found latest dates for {} CIKs", db_dates.len());
        Ok(db_dates)
    }

    /// Compare submission dates with database dates concurrently
    async fn compare_submission_dates_with_database(
        &self,
        submission_files: &std::collections::HashMap<String, std::path::PathBuf>,
        stocks: &[(i64, String, String)], 
        database_dates: &std::collections::HashMap<String, Option<chrono::NaiveDate>>,
    ) -> Result<Vec<FreshnessResult>> {
        println!("🔄 Comparing SEC submission dates with database dates...");
        
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(50)); // Limit concurrent file reads
        let results = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        
        // Clone data for tokio::spawn
        let submission_files = submission_files.clone();
        let database_dates = database_dates.clone();
        let stocks_clone: Vec<(i64, String, String)> = stocks.to_vec(); // Clone all stocks data
        
        let eligible_stocks: Vec<(i64, String, String)> = stocks_clone.iter()
            .filter(|(_, cik, _)| submission_files.contains_key(cik))
            .cloned()
            .collect();

        let handles: Vec<_> = eligible_stocks.into_iter()
            .map({
                let submission_files = submission_files.clone();
                let database_dates = database_dates.clone();
                let semaphore = std::sync::Arc::clone(&semaphore);
                let results = std::sync::Arc::clone(&results);
                
                move |(stock_id, cik, symbol)| {
                    let file_path = submission_files[&cik].clone();
                    let database_dates = database_dates.clone();
                    let semaphore = std::sync::Arc::clone(&semaphore);
                    let results = std::sync::Arc::clone(&results);
                    
                    tokio::spawn(async move {
                        let _permit = semaphore.acquire().await.unwrap();
                        
                        match Self::process_single_cik_submission(&file_path, stock_id, &cik, &symbol, database_dates.get(&cik)).await {
                            Ok(result) => {
                                let mut results = results.lock().unwrap();
                                results.push(result);
                            }
                            Err(e) => {
                                eprintln!("❌ Error processing CIK {}: {}", cik, e);
                            }
                        }
                    })
                }
            })
            .collect();
        
        // Wait for all tasks to complete
        for handle in handles {
            handle.await?;
        }
        
        let results = results.lock().unwrap();
        let stale_count = results.iter().filter(|r| r.is_stale).count();
        
        println!("✅ Comparison complete: {} stale CIKs found", stale_count);
        Ok(results.clone())
    }

    /// Process single CIK submission file
    async fn process_single_cik_submission(
        file_path: &std::path::PathBuf,
        stock_id: i64,
        cik: &str,
        symbol: &str,
        database_latest_date: Option<&Option<chrono::NaiveDate>>,
    ) -> Result<FreshnessResult> {
        let content = tokio::fs::read_to_string(file_path).await?;
        let submission_data: SecSubmissionData = serde_json::from_str(&content)?;
        
        // Parse filing dates and find annual reports (10-K) and quarterlies (10-Q)
        let mut annual_dates = Vec::new();
        let mut quarterly_dates = Vec::new();
        
        for i in 0..submission_data.filings.recent.form.len() {
            if i < submission_data.filings.recent.filing_date.len() {
                let form_type = &submission_data.filings.recent.form[i];
                let filing_date_str = &submission_data.filings.recent.filing_date[i];
                
                if let Ok(filing_date) = chrono::NaiveDate::parse_from_str(filing_date_str, "%Y-%m-%d") {
                    match form_type.as_str() {
                        "10-K" => annual_dates.push(filing_date),
                        "10-Q" => quarterly_dates.push(filing_date),
                        _ => {} // Skip other form types
                    }
                }
            }
        }
        
        // Find the latest filing date (annual takes precedence over quarterly)
        let latest_filing_date = annual_dates.iter().max()
            .or_else(|| quarterly_dates.iter().max())
            .copied();
        
        let db_latest_date = database_latest_date.and_then(|d| *d);
        
        // Determine if data is stale
        let is_stale = match (latest_filing_date, db_latest_date) {
            (Some(sec_date), Some(db_date)) => sec_date > db_date,
            (Some(_), None) => true, // SEC has data, DB doesn't
            (None, Some(_)) => false, // SEC has no recent filings, DB data is current
            (None, None) => false, // No data anywhere
        };
        
        // Find missing filing dates (dates in SEC but not in DB)
        let missing_dates = match (latest_filing_date, db_latest_date) {
            (Some(sec_date), Some(db_date)) if sec_date > db_date => {
                let all_dates = annual_dates.iter().chain(quarterly_dates.iter())
                    .filter(|&&date| date > db_date)
                    .copied()
                    .collect();
                all_dates
            }
            _ => Vec::new(),
        };
        
        Ok(FreshnessResult {
            cik: cik.to_string(),
            stock_id,
            symbol: symbol.to_string(),
            latest_filing_date,
            database_latest_date: db_latest_date,
            is_stale,
            missing_filing_dates: missing_dates,
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
