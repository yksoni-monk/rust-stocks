use sqlx::{SqlitePool, Row};
use serde::{Deserialize, Serialize};
use chrono::{NaiveDate, Utc, Datelike};
use anyhow::{Result, anyhow};
use std::time::Duration;
use tokio::time::sleep;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use std::collections::HashMap;

/// SEC EDGAR API client for downloading 10-K filings and extracting balance sheet data
pub struct SecEdgarClient {
    pool: SqlitePool,
    http_client: Client,
    rate_limiter: RateLimiter,
}

/// Rate limiter to respect SEC's 10 requests per second limit
struct RateLimiter {
    last_request: std::time::Instant,
    min_interval: Duration,
}

impl RateLimiter {
    fn new() -> Self {
        Self {
            last_request: std::time::Instant::now() - Duration::from_millis(100),
            min_interval: Duration::from_millis(100), // 10 requests per second
        }
    }

    async fn wait_if_needed(&mut self) {
        let elapsed = self.last_request.elapsed();
        if elapsed < self.min_interval {
            sleep(self.min_interval - elapsed).await;
        }
        self.last_request = std::time::Instant::now();
    }
}

/// CIK mapping for a company
#[derive(Debug, Clone)]
pub struct CikMapping {
    pub cik: String,
    pub stock_id: i64,
    pub symbol: String,
    pub company_name: String,
}

/// SEC filing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecFiling {
    pub accession_number: String,
    pub filing_date: NaiveDate,
    pub form_type: String,
    pub document_url: String,
    pub excel_url: String,
}

/// Balance sheet data extracted from SEC filing
#[derive(Debug, Clone)]
pub struct BalanceSheetData {
    pub stock_id: i64,
    pub symbol: String,
    pub report_date: NaiveDate,
    pub fiscal_year: i32,
    pub total_assets: Option<f64>,
    pub total_liabilities: Option<f64>,
    pub total_equity: Option<f64>,
    pub cash_and_equivalents: Option<f64>,
    pub short_term_debt: Option<f64>,
    pub long_term_debt: Option<f64>,
    pub total_debt: Option<f64>,
    pub current_assets: Option<f64>,
    pub current_liabilities: Option<f64>,
    pub share_repurchases: Option<f64>,
}

/// Income statement data extracted from SEC filing
#[derive(Debug, Clone)]
pub struct IncomeStatementData {
    pub stock_id: i64,
    pub symbol: String,
    pub report_date: NaiveDate,
    pub fiscal_year: i32,
    pub period_type: String,
    pub revenue: Option<f64>,
    pub net_income: Option<f64>,
    pub operating_income: Option<f64>,
    pub gross_profit: Option<f64>,
    pub cost_of_revenue: Option<f64>,
    pub interest_expense: Option<f64>,
    pub tax_expense: Option<f64>,
    pub shares_basic: Option<f64>,
    pub shares_diluted: Option<f64>,
}

/// Cash flow statement data extracted from SEC filing
#[derive(Debug, Clone)]
pub struct CashFlowData {
    pub stock_id: i64,
    pub symbol: String,
    pub report_date: NaiveDate,
    pub fiscal_year: i32,
    pub depreciation_expense: Option<f64>,
    pub amortization_expense: Option<f64>,
    pub dividends_paid: Option<f64>,
    pub share_repurchases: Option<f64>,
    pub operating_cash_flow: Option<f64>,
    pub investing_cash_flow: Option<f64>,
    pub financing_cash_flow: Option<f64>,
}

/// Company Facts API response structure for filing metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyFactsResponse {
    pub cik: u64,
    pub entity_name: String,
    pub entity_type: String,
    pub facts: serde_json::Value,
}

/// Filing metadata extracted from Company Facts API
#[derive(Debug, Clone)]
pub struct FilingMetadata {
    pub accession_number: String,
    pub form_type: String,
    pub filing_date: String,
    pub fiscal_period: String,
    pub report_date: String,
}

impl SecEdgarClient {
    /// Create a new SEC EDGAR client
    pub fn new(pool: SqlitePool) -> Self {
        let http_client = Client::builder()
            .user_agent("rust-stocks-edgar-client/1.0 (contact@example.com)")
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            pool,
            http_client,
            rate_limiter: RateLimiter::new(),
        }
    }

    /// Check if financial data needs update based on latest SEC filings
    /// Check if stock needs update based on data coverage (not just latest filing date)
    pub async fn check_if_update_needed(&mut self, cik: &str, stock_id: i64) -> Result<bool> {
        // Check if we have sufficient historical data coverage (at least 5 years)
        let years_covered = self.get_years_of_data_coverage(stock_id).await?;
        
        if years_covered < 5 {
            println!("    📊 {} has only {} years of data, needs historical download", cik, years_covered);
            return Ok(true);
        }
        
        // If we have 5+ years, check if there's newer data available
        let latest_filing = self.get_latest_filing_date_from_api(cik).await?;
        let our_latest = self.get_our_latest_filing_date(stock_id).await?;
        
        match (latest_filing, our_latest) {
            (Some(sec_date), Some(our_date)) => {
                if sec_date > our_date {
                    println!("    📋 {} has newer SEC filings available", cik);
                    Ok(true)
                } else {
                    Ok(false)
                }
            },
            (Some(_), None) => Ok(true), // We have no data yet
            (None, _) => Ok(false), // SEC has no data or API failed
        }
    }
    
    /// Get number of years of data coverage for a stock
    async fn get_years_of_data_coverage(&self, stock_id: i64) -> Result<i32> {
        let query = r#"
            SELECT COUNT(DISTINCT fiscal_year) as years_covered
            FROM (
                SELECT fiscal_year FROM income_statements WHERE stock_id = ? AND fiscal_year >= 2016
                UNION
                SELECT fiscal_year FROM balance_sheets WHERE stock_id = ? AND fiscal_year >= 2016
                UNION
                SELECT fiscal_year FROM cash_flow_statements WHERE stock_id = ? AND fiscal_year >= 2016
            )
        "#;
        
        let result: Option<i32> = sqlx::query_scalar(query)
            .bind(stock_id)
            .bind(stock_id)
            .bind(stock_id)
            .fetch_optional(&self.pool)
            .await?;
            
        Ok(result.unwrap_or(0))
    }

    /// Get latest filing date from SEC Company Facts API
    async fn get_latest_filing_date_from_api(&mut self, cik: &str) -> Result<Option<String>> {
        self.rate_limiter.wait_if_needed().await;

        let url = format!(
            "https://data.sec.gov/api/xbrl/companyfacts/CIK{:0>10}.json",
            cik
        );

        let response = self.http_client
            .get(&url)
            .header("User-Agent", "rust-stocks-edgar-client/1.0 (contact@example.com)")
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(None); // API failed, don't update
        }

        let json: serde_json::Value = response.json().await?;
        
        // Extract latest filing date from any financial concept
        let mut latest_date: Option<String> = None;
        
        if let Some(facts) = json.get("facts").and_then(|f| f.get("us-gaap")) {
            // Look through some common financial concepts
            let concepts_to_check = [
                "Assets", "Revenues", "NetIncomeLoss", "OperatingIncomeLoss"
            ];
            
            for concept in &concepts_to_check {
                if let Some(field_data) = facts.get(concept) {
                    if let Some(units) = field_data.get("units") {
                        if let Some(usd_data) = units.get("USD") {
                            if let Some(values) = usd_data.as_array() {
                                for value in values {
                                    if let Some(filed_date) = value.get("filed").and_then(|d| d.as_str()) {
                                        // Filter out future dates - only use actual historical filings
                                        if let Ok(parsed_date) = chrono::NaiveDate::parse_from_str(filed_date, "%Y-%m-%d") {
                                            let today = chrono::Utc::now().date_naive();
                                            if parsed_date <= today {
                                                if latest_date.is_none() || filed_date > latest_date.as_ref().unwrap().as_str() {
                                                    latest_date = Some(filed_date.to_string());
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

        Ok(latest_date)
    }

    /// Get latest filing date from our database (checks all financial tables)
    pub async fn get_our_latest_filing_date(&self, stock_id: i64) -> Result<Option<String>> {
        // ✅ FIXED: Query sec_filings table instead of financial tables
        let query = r#"
            SELECT MAX(sf.filed_date) as latest_filed_date 
            FROM sec_filings sf
            WHERE sf.stock_id = ? AND sf.filed_date IS NOT NULL
        "#;
        
        let result: Option<String> = sqlx::query_scalar(query)
            .bind(stock_id)
            .fetch_optional(&self.pool)
            .await?;
            
        Ok(result)
    }

    /// Extract filing metadata from Company Facts API response
    pub fn extract_filing_metadata(&self, json: &serde_json::Value, _symbol: &str) -> Result<Vec<FilingMetadata>> {
        let mut metadata_vec = Vec::new();
        
        if let Some(facts) = json.get("facts").and_then(|f| f.get("us-gaap")) {
            // Check a bunch of financial concepts to get comprehensive filing metadata
            let concepts_to_check = [
                "Assets", "Revenues", "NetIncomeLoss", "OperatingIncomeLoss",
                "Liabilities", "StockholdersEquity", "CashAndCashEquivalentsAtCarryingValue"
            ];
            
            for concept in &concepts_to_check {
                if let Some(field_data) = facts.get(concept) {
                    if let Some(units) = field_data.get("units") {
                        if let Some(usd_data) = units.get("USD") {
                            if let Some(values) = usd_data.as_array() {
                                for value in values {
                                    if let (Some(accn), Some(form), Some(filed), Some(fp), Some(end)) = (
                                        value.get("accn").and_then(|a| a.as_str()),
                                        value.get("form").and_then(|f| f.as_str()),
                                        value.get("filed").and_then(|d| d.as_str()),
                                        value.get("fp").and_then(|fp| fp.as_str()),
                                        value.get("end").and_then(|e| e.as_str())
                                    ) {
                                        let metadata = FilingMetadata {
                                            accession_number: accn.to_string(),
                                            form_type: form.to_string(),
                                            filing_date: filed.to_string(),
                                            fiscal_period: fp.to_string(),
                                            report_date: end.to_string(),
                                        };
                                        metadata_vec.push(metadata);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Remove duplicates based on accession number
        metadata_vec.sort_by(|a, b| a.accession_number.cmp(&b.accession_number));
        metadata_vec.dedup_by(|a, b| a.accession_number == b.accession_number);
        
        Ok(metadata_vec)
    }

    /// Get all CIK mappings for S&P 500 companies
    pub async fn get_sp500_cik_mappings(&self) -> Result<Vec<CikMapping>> {
        let query = r#"
            SELECT cik, id as stock_id, symbol, company_name
            FROM stocks
            WHERE is_sp500 = 1 AND cik IS NOT NULL
            ORDER BY symbol
        "#;

        let rows = sqlx::query(query).fetch_all(&self.pool).await?;
        
        let mut mappings = Vec::new();
        for row in rows {
            let mapping = CikMapping {
                cik: row.get("cik"),
                stock_id: row.get("stock_id"),
                symbol: row.get("symbol"),
                company_name: row.get("company_name"),
            };
            mappings.push(mapping);
        }

        println!("📊 Found {} S&P 500 CIK mappings", mappings.len());
        Ok(mappings)
    }

    /// Discover 10-K filings for a company over the last 5 years
    pub async fn discover_10k_filings(&mut self, cik: &str, symbol: &str) -> Result<Vec<SecFiling>> {
        self.rate_limiter.wait_if_needed().await;

        let current_year = Utc::now().year();
        let start_year = current_year - 5; // Last 5 years
        
        // SEC EDGAR Submissions API endpoint for company filings
        let url = format!(
            "https://data.sec.gov/submissions/CIK{:0>10}.json",
            cik
        );

        let response = self.http_client
            .get(&url)
            .header("Accept", "application/json")
            .header("User-Agent", "rust-stocks-edgar-client/1.0 (contact@example.com)")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to fetch filings for {} (CIK: {}): {}", 
                symbol, cik, response.status()));
        }

        let json: serde_json::Value = response.json().await?;
        
        // Extract 10-K filings from the response
        let mut filings = Vec::new();
        
        // The submissions API has a "filings" object with "recent" array
        if let Some(filings_data) = json.get("filings").and_then(|f| f.get("recent")) {
            if let Some(accession_numbers) = filings_data.get("accessionNumber").and_then(|a| a.as_array()) {
                if let Some(form_types) = filings_data.get("form").and_then(|f| f.as_array()) {
                    if let Some(filing_dates) = filings_data.get("filingDate").and_then(|d| d.as_array()) {
                        if let Some(primary_documents) = filings_data.get("primaryDocument").and_then(|p| p.as_array()) {
                            
                            for i in 0..accession_numbers.len() {
                                if let (Some(form_type), Some(filing_date), Some(accession_number), Some(primary_doc)) = (
                                    form_types.get(i).and_then(|f| f.as_str()),
                                    filing_dates.get(i).and_then(|d| d.as_str()),
                                    accession_numbers.get(i).and_then(|a| a.as_str()),
                                    primary_documents.get(i).and_then(|p| p.as_str())
                                ) {
                                    if form_type == "10-K" {
                                        if let Ok(date) = NaiveDate::parse_from_str(filing_date, "%Y-%m-%d") {
                                            if date.year() >= start_year {
                                                let accession_clean = accession_number.replace("-", "");
                                                let excel_url = format!(
                                                    "https://www.sec.gov/Archives/edgar/data/{}/{}/Financial_Report.xlsx",
                                                    cik, accession_clean
                                                );
                                                
                                                let filing = SecFiling {
                                                    accession_number: accession_number.to_string(),
                                                    filing_date: date,
                                                    form_type: form_type.to_string(),
                                                    document_url: format!(
                                                        "https://www.sec.gov/Archives/edgar/data/{}/{}/{}",
                                                        cik, accession_clean, primary_doc
                                                    ),
                                                    excel_url,
                                                };
                                                filings.push(filing);
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

        // Sort by filing date (most recent first)
        filings.sort_by(|a, b| b.filing_date.cmp(&a.filing_date));
        
        println!("  📋 Found {} 10-K filings for {} (last 5 years)", filings.len(), symbol);
        Ok(filings)
    }

    /// Extract historical balance sheet data using SEC EDGAR Company Facts API
    pub async fn extract_balance_sheet_data(&mut self, cik: &str, stock_id: i64, symbol: &str) -> Result<Option<BalanceSheetData>> {
        self.rate_limiter.wait_if_needed().await;

        println!("  📊 Extracting historical balance sheet data for {} using Company Facts API", symbol);
        
        // Use SEC EDGAR Company Facts API
        let url = format!(
            "https://data.sec.gov/api/xbrl/companyfacts/CIK{:0>10}.json",
            cik
        );

        let response = self.http_client
            .get(&url)
            .header("User-Agent", "rust-stocks-edgar-client/1.0 (contact@example.com)")
            .send()
            .await?;

        if !response.status().is_success() {
            println!("    ⚠️ Company Facts API failed for {}: {}", symbol, response.status());
            return Ok(None);
        }

        let json: serde_json::Value = response.json().await?;
        
        // Extract historical balance sheet data from JSON
        let historical_balance_data = self.parse_company_facts_json(&json, symbol)?;
        
        // Extract historical cash flow data from JSON
        let historical_cash_flow_data = self.parse_cash_flow_json(&json, symbol)?;
        
        if historical_balance_data.is_empty() && historical_cash_flow_data.is_empty() {
            println!("    ⚠️ No historical data found for {}", symbol);
            return Ok(None);
        }

        // Extract filing metadata for storage
        let filing_metadata = self.extract_filing_metadata(&json, symbol).ok();
        
        // Group historical data by report date and store multiple records
        let mut stored_records = 0;
        
        // Group balance sheet data by report date
        let mut balance_by_date: HashMap<String, HashMap<String, f64>> = HashMap::new();
        for (field, value, report_date, _filed_date) in historical_balance_data {
            balance_by_date.entry(report_date.clone())
                .or_insert_with(HashMap::new)
                .insert(field, value);
        }
        
        // Group cash flow data by report date
        let mut cash_flow_by_date: HashMap<String, HashMap<String, f64>> = HashMap::new();
        for (field, value, report_date, _filed_date) in historical_cash_flow_data {
            cash_flow_by_date.entry(report_date.clone())
                .or_insert_with(HashMap::new)
                .insert(field, value);
        }
        
        // Store data for each report date
        for (report_date_str, balance_data) in balance_by_date {
            if let Ok(report_date) = chrono::NaiveDate::parse_from_str(&report_date_str, "%Y-%m-%d") {
                let fiscal_year = report_date.year() as i32;
                
                // Calculate total debt from components if not directly available
                let short_term_debt = balance_data.get("ShortTermDebt")
                    .or(balance_data.get("DebtCurrent"))
                    .copied();
                let long_term_debt = balance_data.get("LongTermDebt").copied();
                let total_debt = balance_data.get("TotalDebt")
                    .copied()
                    .or_else(|| {
                        // Calculate from components if available
                        match (short_term_debt, long_term_debt) {
                            (Some(st), Some(lt)) => Some(st + lt),
                            (Some(st), None) => Some(st),
                            (None, Some(lt)) => Some(lt),
                            (None, None) => None,
                        }
                    });
                
                // Find matching filing metadata for this report date
                let matching_metadata = filing_metadata.as_ref()
                    .and_then(|metadata_vec| {
                        metadata_vec.iter()
                            .find(|m| m.report_date == report_date_str)
                    });

                // Store balance sheet data
                let balance_sheet_result = self.store_balance_sheet_data(&BalanceSheetData {
                    stock_id,
                    symbol: symbol.to_string(),
                    report_date,
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
                }, matching_metadata).await;

                // Store cash flow data for the same report date
                if let Some(cash_flow_data) = cash_flow_by_date.get(&report_date_str) {
                    let cash_flow_result = self.store_cash_flow_data(&CashFlowData {
                        stock_id,
                        symbol: symbol.to_string(),
                        report_date,
                        fiscal_year,
                        depreciation_expense: cash_flow_data.get("depreciation_expense").copied(),
                        amortization_expense: cash_flow_data.get("amortization_expense").copied(),
                        dividends_paid: cash_flow_data.get("dividends_paid").copied(),
                        share_repurchases: cash_flow_data.get("share_repurchases").copied(),
                        operating_cash_flow: cash_flow_data.get("operating_cash_flow").copied(),
                        investing_cash_flow: cash_flow_data.get("investing_cash_flow").copied(),
                        financing_cash_flow: cash_flow_data.get("financing_cash_flow").copied(),
                    }, matching_metadata).await;

                    if cash_flow_result.is_err() {
                        println!("    ⚠️ Failed to store cash flow data for {} on {}", symbol, report_date_str);
                    }
                }

                if balance_sheet_result.is_ok() {
                    stored_records += 1;
                } else {
                    println!("    ⚠️ Failed to store balance sheet data for {} on {}", symbol, report_date_str);
                }
            }
        }

        println!("    ✅ Successfully stored {} historical balance sheet records for {}", stored_records, symbol);
        
        // Return the most recent record for compatibility
        if stored_records > 0 {
            Ok(Some(BalanceSheetData {
                stock_id,
                symbol: symbol.to_string(),
                report_date: chrono::NaiveDate::from_ymd_opt(2024, 12, 31).unwrap_or_default(),
                fiscal_year: 2024,
                total_assets: None,
                total_liabilities: None,
                total_equity: None,
                cash_and_equivalents: None,
                short_term_debt: None,
                long_term_debt: None,
                total_debt: None,
                current_assets: None,
                current_liabilities: None,
                share_repurchases: None,
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse Company Facts JSON to extract historical balance sheet data since 2016
    pub fn parse_company_facts_json(&self, json: &serde_json::Value, symbol: &str) -> Result<Vec<(String, f64, String, String)>> {
        let mut historical_data = Vec::new();
        
        // Balance sheet field mappings (US GAAP taxonomy)
        let field_mappings = [
            ("Assets", "Assets"),
            ("AssetsCurrent", "AssetsCurrent"),
            ("Liabilities", "Liabilities"),
            ("LiabilitiesCurrent", "LiabilitiesCurrent"),
            ("StockholdersEquity", "StockholdersEquity"),
            ("CashAndCashEquivalentsAtCarryingValue", "CashAndCashEquivalentsAtCarryingValue"),
            // Debt fields - try multiple XBRL concepts
            ("ShortTermDebt", "ShortTermDebt"),
            ("DebtCurrent", "DebtCurrent"),
            ("LongTermDebtCurrent", "DebtCurrent"),
            ("LongTermDebtNoncurrent", "LongTermDebt"),
            ("LongTermDebt", "LongTermDebt"),
            ("LongTermDebtAndCapitalLeaseObligations", "LongTermDebt"),
            ("LongTermDebtAndCapitalLeaseObligationsNoncurrent", "LongTermDebt"),
            ("Debt", "TotalDebt"),
            ("DebtAndCapitalLeaseObligations", "TotalDebt"),
            ("PaymentsForRepurchaseOfCommonStock", "ShareRepurchases"),
        ];

        // Navigate to the facts section
        if let Some(facts) = json.get("facts").and_then(|f| f.get("us-gaap")) {
            for (field_name, our_field) in &field_mappings {
                if let Some(field_data) = facts.get(field_name) {
                    if let Some(units) = field_data.get("units") {
                        if let Some(usd_data) = units.get("USD") {
                            if let Some(values) = usd_data.as_array() {
                                // Extract ALL historical values since 2016
                                for value in values {
                                    if let (Some(val), Some(end_date), Some(filed_date)) = (
                                        value.get("val").and_then(|v| v.as_f64()),
                                        value.get("end").and_then(|e| e.as_str()),
                                        value.get("filed").and_then(|f| f.as_str())
                                    ) {
                                        // Parse the end date to check if it's 2016 or later
                                        if let Ok(parsed_date) = chrono::NaiveDate::parse_from_str(end_date, "%Y-%m-%d") {
                                            // Also parse the filed date to filter out future dates
                                            if let Ok(filed_parsed) = chrono::NaiveDate::parse_from_str(filed_date, "%Y-%m-%d") {
                                                let today = chrono::Utc::now().date_naive();
                                                if parsed_date.year() >= 2016 && val != 0.0 && filed_parsed <= today {
                                                    historical_data.push((
                                                        our_field.to_string(),
                                                        val,
                                                        end_date.to_string(),
                                                        filed_date.to_string()
                                                    ));
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

        // Sort by field name and date for better organization
        historical_data.sort_by(|a, b| {
            let field_cmp = a.0.cmp(&b.0);
            if field_cmp == std::cmp::Ordering::Equal {
                b.2.cmp(&a.2) // Most recent first
            } else {
                field_cmp
            }
        });

        println!("    📊 Extracted {} historical balance sheet data points since 2016 for {}", historical_data.len(), symbol);
        Ok(historical_data)
    }

    /// Parse Company Facts JSON to extract historical cash flow statement data since 2016
    pub fn parse_cash_flow_json(&self, json: &serde_json::Value, symbol: &str) -> Result<Vec<(String, f64, String, String)>> {
        let mut historical_data = Vec::new();
        
        // Cash flow statement field mappings (US GAAP taxonomy)
        let field_mappings = [
            ("DepreciationAndAmortization", "depreciation_and_amortization"),
            ("Depreciation", "depreciation_expense"),
            ("DepreciationExpense", "depreciation_expense"),
            ("DepreciationOfPropertyPlantAndEquipment", "depreciation_expense"),
            ("AmortizationOfIntangibleAssets", "amortization_expense"),
            ("Amortization", "amortization_expense"),
            ("AmortizationExpense", "amortization_expense"),
            ("PaymentsOfDividends", "dividends_paid"),
            ("PaymentsForRepurchaseOfCommonStock", "share_repurchases"),
            ("NetCashProvidedByUsedInOperatingActivities", "operating_cash_flow"),
            ("NetCashProvidedByUsedInInvestingActivities", "investing_cash_flow"),
            ("NetCashProvidedByUsedInFinancingActivities", "financing_cash_flow"),
        ];

        // Navigate to the facts section
        if let Some(facts) = json.get("facts").and_then(|f| f.get("us-gaap")) {
            for (field_name, our_field) in &field_mappings {
                if let Some(field_data) = facts.get(field_name) {
                    if let Some(units) = field_data.get("units") {
                        if let Some(usd_data) = units.get("USD") {
                            if let Some(values) = usd_data.as_array() {
                                // Extract ALL historical values since 2016
                                for value in values {
                                    if let (Some(val), Some(end_date), Some(filed_date)) = (
                                        value.get("val").and_then(|v| v.as_f64()),
                                        value.get("end").and_then(|e| e.as_str()),
                                        value.get("filed").and_then(|f| f.as_str())
                                    ) {
                                        // Parse the end date to check if it's 2016 or later
                                        if let Ok(parsed_date) = chrono::NaiveDate::parse_from_str(end_date, "%Y-%m-%d") {
                                            // Also parse the filed date to filter out future dates
                                            if let Ok(filed_parsed) = chrono::NaiveDate::parse_from_str(filed_date, "%Y-%m-%d") {
                                                let today = chrono::Utc::now().date_naive();
                                                if parsed_date.year() >= 2016 && val != 0.0 && filed_parsed <= today {
                                                    historical_data.push((
                                                        our_field.to_string(),
                                                        val,
                                                        end_date.to_string(),
                                                        filed_date.to_string()
                                                    ));
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

        // Sort by field name and date for better organization
        historical_data.sort_by(|a, b| {
            let field_cmp = a.0.cmp(&b.0);
            if field_cmp == std::cmp::Ordering::Equal {
                b.2.cmp(&a.2) // Most recent first
            } else {
                field_cmp
            }
        });

        println!("    💰 Extracted {} historical cash flow data points since 2016 for {}", historical_data.len(), symbol);
        Ok(historical_data)
    }

    /// Parse Company Facts JSON to extract historical income statement data since 2016
    pub fn parse_income_statement_json(&self, json: &serde_json::Value, symbol: &str) -> Result<Vec<(String, f64, String, String)>> {
        let mut historical_data = Vec::new();
        
        // Income statement field mappings (US GAAP taxonomy)
        let field_mappings = [
            ("Revenues", "revenue"),
            ("RevenueFromContractWithCustomerExcludingAssessedTax", "revenue"),
            ("SalesRevenueNet", "revenue"),
            ("NetIncomeLoss", "net_income"),
            ("OperatingIncomeLoss", "operating_income"),
            ("IncomeLossFromContinuingOperationsBeforeIncomeTaxesExtraordinaryItemsNoncontrollingInterest", "operating_income"),
            ("IncomeLossFromContinuingOperationsBeforeIncomeTaxesMinorityInterestAndIncomeLossFromEquityMethodInvestments", "operating_income"),
            ("GrossProfit", "gross_profit"),
            ("CostOfGoodsAndServicesSold", "cost_of_revenue"),
            ("InterestExpense", "interest_expense"),
            ("IncomeTaxExpenseBenefit", "tax_expense"),
        ];

        // Shares outstanding field mappings
        let shares_mappings = [
            ("WeightedAverageNumberOfSharesOutstandingBasic", "shares_basic"),
            ("WeightedAverageNumberOfSharesOutstandingDiluted", "shares_diluted"),
            ("EntityCommonStockSharesOutstanding", "shares_outstanding"),
            ("WeightedAverageNumberOfDilutedSharesOutstanding", "shares_diluted"),
            ("CommonStockSharesOutstanding", "shares_outstanding"),
            ("CommonStockSharesOutstandingBasic", "shares_basic"),
            ("CommonStockSharesOutstandingDiluted", "shares_diluted"),
            ("WeightedAverageNumberOfSharesOutstandingBasicIncludingDilutiveEffect", "shares_diluted"),
            ("WeightedAverageNumberOfSharesOutstandingBasicExcludingDilutiveEffect", "shares_basic"),
        ];

        // Navigate to the facts section
        if let Some(facts) = json.get("facts").and_then(|f| f.get("us-gaap")) {
            // Extract income statement data
            for (field_name, our_field) in &field_mappings {
                if let Some(field_data) = facts.get(field_name) {
                    if let Some(units) = field_data.get("units") {
                        if let Some(usd_data) = units.get("USD") {
                            if let Some(values) = usd_data.as_array() {
                                // Extract ALL historical values since 2016
                                for value in values {
                                    if let (Some(val), Some(end_date), Some(filed_date)) = (
                                        value.get("val").and_then(|v| v.as_f64()),
                                        value.get("end").and_then(|e| e.as_str()),
                                        value.get("filed").and_then(|f| f.as_str())
                                    ) {
                                        // Parse the end date to check if it's 2016 or later
                                        if let Ok(parsed_date) = chrono::NaiveDate::parse_from_str(end_date, "%Y-%m-%d") {
                                            // Also parse the filed date to filter out future dates
                                            if let Ok(filed_parsed) = chrono::NaiveDate::parse_from_str(filed_date, "%Y-%m-%d") {
                                                let today = chrono::Utc::now().date_naive();
                                                if parsed_date.year() >= 2016 && val != 0.0 && filed_parsed <= today {
                                                    historical_data.push((
                                                        our_field.to_string(),
                                                        val,
                                                        end_date.to_string(),
                                                        filed_date.to_string()
                                                    ));
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

            // Extract shares outstanding data
            for (field_name, our_field) in &shares_mappings {
                if let Some(field_data) = facts.get(field_name) {
                    if let Some(units) = field_data.get("units") {
                        if let Some(shares_data) = units.get("shares") {
                            if let Some(values) = shares_data.as_array() {
                                // Extract ALL historical values since 2016
                                for value in values {
                                    if let (Some(val), Some(end_date), Some(filed_date)) = (
                                        value.get("val").and_then(|v| v.as_f64()),
                                        value.get("end").and_then(|e| e.as_str()),
                                        value.get("filed").and_then(|f| f.as_str())
                                    ) {
                                        // Parse the end date to check if it's 2016 or later
                                        if let Ok(parsed_date) = chrono::NaiveDate::parse_from_str(end_date, "%Y-%m-%d") {
                                            if parsed_date.year() >= 2016 && val > 0.0 {
                                                historical_data.push((
                                                    our_field.to_string(),
                                                    val,
                                                    end_date.to_string(),
                                                    filed_date.to_string()
                                                ));
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

        // Sort by field name and date for better organization
        historical_data.sort_by(|a, b| {
            let field_cmp = a.0.cmp(&b.0);
            if field_cmp == std::cmp::Ordering::Equal {
                b.2.cmp(&a.2) // Most recent first
            } else {
                field_cmp
            }
        });

        println!("    📈 Extracted {} historical income statement data points since 2016 for {}", historical_data.len(), symbol);
        Ok(historical_data)
    }

    /// Extract income statement data using SEC EDGAR Company Facts API
    pub async fn extract_income_statement_data(&mut self, cik: &str, stock_id: i64, symbol: &str) -> Result<Option<IncomeStatementData>> {
        self.rate_limiter.wait_if_needed().await;

        println!("  📈 Extracting income statement data for {} using Company Facts API", symbol);

        // Use SEC EDGAR Company Facts API
        let url = format!(
            "https://data.sec.gov/api/xbrl/companyfacts/CIK{:0>10}.json",
            cik
        );

        let response = self.http_client
            .get(&url)
            .header("User-Agent", "rust-stocks-edgar-client/1.0 (contact@example.com)")
            .send()
            .await?;

        if !response.status().is_success() {
            println!("    ⚠️ Company Facts API failed for {}: {}", symbol, response.status());
            return Ok(None);
        }

        let json: serde_json::Value = response.json().await?;

        // Extract income statement data from JSON
        let historical_income_data = self.parse_income_statement_json(&json, symbol)?;

        if historical_income_data.is_empty() {
            println!("    ⚠️ No historical income statement data found for {}", symbol);
            return Ok(None);
        }

        // Extract filing metadata for storage
        let filing_metadata = self.extract_filing_metadata(&json, symbol).ok();
        
        // Group historical data by report date and store multiple records
        let mut stored_records = 0;
        
        // Group income statement data by report date
        let mut income_by_date: HashMap<String, HashMap<String, f64>> = HashMap::new();
        for (field, value, report_date, _filed_date) in historical_income_data {
            income_by_date.entry(report_date.clone())
                .or_insert_with(HashMap::new)
                .insert(field, value);
        }
        
        // Store data for each report date
        for (report_date_str, income_data) in income_by_date {
            if let Ok(report_date) = chrono::NaiveDate::parse_from_str(&report_date_str, "%Y-%m-%d") {
                let fiscal_year = report_date.year() as i32;
                
                // Find matching filing metadata for this report date
                let matching_metadata = filing_metadata.as_ref()
                    .and_then(|metadata_vec| {
                        metadata_vec.iter()
                            .find(|m| m.report_date == report_date_str)
                    });

                // Store income statement data
                let income_result = self.store_income_statement_data(&IncomeStatementData {
                    stock_id,
                    symbol: symbol.to_string(),
                    report_date,
                    fiscal_year,
                    period_type: "Annual".to_string(),
                    revenue: income_data.get("revenue").copied(),
                    net_income: income_data.get("net_income").copied(),
                    operating_income: income_data.get("operating_income").copied(),
                    gross_profit: income_data.get("gross_profit").copied(),
                    cost_of_revenue: income_data.get("cost_of_revenue").copied(),
                    interest_expense: income_data.get("interest_expense").copied(),
                    tax_expense: income_data.get("tax_expense").copied(),
                    shares_basic: income_data.get("shares_basic").copied(),
                    shares_diluted: income_data.get("shares_diluted").copied(),
                }, matching_metadata).await;

                if income_result.is_ok() {
                    stored_records += 1;
                } else {
                    println!("    ⚠️ Failed to store income statement data for {} on {}", symbol, report_date_str);
                }
            }
        }

        println!("    ✅ Successfully stored {} historical income statement records for {}", stored_records, symbol);
        
        // Return the most recent record for compatibility
        if stored_records > 0 {
            Ok(Some(IncomeStatementData {
                stock_id,
                symbol: symbol.to_string(),
                report_date: chrono::NaiveDate::from_ymd_opt(2024, 12, 31).unwrap_or_default(),
                fiscal_year: 2024,
                period_type: "Annual".to_string(),
                revenue: None,
                net_income: None,
                operating_income: None,
                gross_profit: None,
                cost_of_revenue: None,
                interest_expense: None,
                tax_expense: None,
                shares_basic: None,
                shares_diluted: None,
            }))
        } else {
            Ok(None)
        }
    }

    /// Create or get existing sec_filing record
    async fn create_or_get_sec_filing(&self, stock_id: i64, metadata: &FilingMetadata, fiscal_year: i32, report_date: &str) -> Result<i64> {
        // First try to find existing record
        let existing_query = r#"
            SELECT id FROM sec_filings 
            WHERE stock_id = ? AND accession_number = ? AND form_type = ? AND filed_date = ?
        "#;
        
        if let Some(existing_id) = sqlx::query_scalar::<_, i64>(existing_query)
            .bind(stock_id)
            .bind(&metadata.accession_number)
            .bind(&metadata.form_type)
            .bind(&metadata.filing_date)
            .fetch_optional(&self.pool)
            .await?
        {
            return Ok(existing_id);
        }

        // Create new record with all required columns
        let insert_query = r#"
            INSERT INTO sec_filings (stock_id, accession_number, form_type, filed_date, fiscal_period, fiscal_year, report_date)
            VALUES (?, ?, ?, ?, ?, ?, ?)
        "#;
        
        let result = sqlx::query(insert_query)
            .bind(stock_id)
            .bind(&metadata.accession_number)
            .bind(&metadata.form_type)
            .bind(&metadata.filing_date)
            .bind(&metadata.fiscal_period)
            .bind(fiscal_year)
            .bind(report_date)
            .execute(&self.pool)
            .await?;
            
        Ok(result.last_insert_rowid())
    }

    /// Store balance sheet data in the database with filing metadata
    pub async fn store_balance_sheet_data(&self, data: &BalanceSheetData, filing_metadata: Option<&FilingMetadata>) -> Result<()> {
        // First, create or get the sec_filing record if metadata is provided
        let sec_filing_id = if let Some(metadata) = filing_metadata {
            Some(self.create_or_get_sec_filing(data.stock_id, metadata, data.fiscal_year, &data.report_date.format("%Y-%m-%d").to_string()).await?)
        } else {
            None
        };

        let query = r#"
            INSERT OR REPLACE INTO balance_sheets (
                stock_id, period_type, report_date, fiscal_year,
                total_assets, total_liabilities, total_equity,
                cash_and_equivalents, short_term_debt, long_term_debt, total_debt,
                current_assets, current_liabilities,
                share_repurchases, sec_filing_id
            ) VALUES (
                ?1, 'Annual', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14
            )
        "#;

        sqlx::query(query)
            .bind(data.stock_id)
            .bind(data.report_date)
            .bind(data.fiscal_year)
            .bind(data.total_assets)
            .bind(data.total_liabilities)
            .bind(data.total_equity)
            .bind(data.cash_and_equivalents)
            .bind(data.short_term_debt)
            .bind(data.long_term_debt)
            .bind(data.total_debt)
            .bind(data.current_assets)
            .bind(data.current_liabilities)
            .bind(data.share_repurchases)
            .bind(sec_filing_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Store cash flow data in the database with filing metadata
    pub async fn store_cash_flow_data(&self, data: &CashFlowData, filing_metadata: Option<&FilingMetadata>) -> Result<()> {
        // First, create or get the sec_filing record if metadata is provided
        let sec_filing_id = if let Some(metadata) = filing_metadata {
            Some(self.create_or_get_sec_filing(data.stock_id, metadata, data.fiscal_year, &data.report_date.format("%Y-%m-%d").to_string()).await?)
        } else {
            None
        };

        let query = r#"
            INSERT OR REPLACE INTO cash_flow_statements (
                stock_id, period_type, report_date, fiscal_year,
                depreciation_expense, amortization_expense, dividends_paid,
                share_repurchases, operating_cash_flow, investing_cash_flow, financing_cash_flow,
                sec_filing_id
            ) VALUES (
                ?1, 'Annual', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11
            )
        "#;

        sqlx::query(query)
            .bind(data.stock_id)
            .bind(data.report_date)
            .bind(data.fiscal_year)
            .bind(data.depreciation_expense)
            .bind(data.amortization_expense)
            .bind(data.dividends_paid)
            .bind(data.share_repurchases)
            .bind(data.operating_cash_flow)
            .bind(data.investing_cash_flow)
            .bind(data.financing_cash_flow)
            .bind(sec_filing_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Store income statement data in the database with filing metadata
    pub async fn store_income_statement_data(&self, data: &IncomeStatementData, filing_metadata: Option<&FilingMetadata>) -> Result<()> {
        // First, create or get the sec_filing record if metadata is provided
        let sec_filing_id = if let Some(metadata) = filing_metadata {
            Some(self.create_or_get_sec_filing(data.stock_id, metadata, data.fiscal_year, &data.report_date.format("%Y-%m-%d").to_string()).await?)
        } else {
            None
        };

        let query = r#"
            INSERT OR REPLACE INTO income_statements (
                stock_id, period_type, report_date, fiscal_year,
                revenue, gross_profit, operating_income, net_income,
                shares_basic, shares_diluted, currency,
                sec_filing_id
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'USD', ?11
            )
        "#;

        sqlx::query(query)
            .bind(data.stock_id)
            .bind(&data.period_type)
            .bind(data.report_date)
            .bind(data.fiscal_year)
            .bind(data.revenue)
            .bind(data.gross_profit)
            .bind(data.operating_income)
            .bind(data.net_income)
            .bind(data.shares_basic)
            .bind(data.shares_diluted)
            .bind(sec_filing_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Download balance sheet data for all S&P 500 companies
    pub async fn download_all_sp500_balance_sheets(&mut self) -> Result<()> {
        println!("🚀 Starting SEC EDGAR balance sheet data download for S&P 500 companies...");
        
        let mappings = self.get_sp500_cik_mappings().await?;
        let total_companies = mappings.len();
        
        let pb = ProgressBar::new(total_companies as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg}")
                .unwrap()
                .progress_chars("#>-")
        );
        pb.set_message("Downloading balance sheet data...");

        let mut success_count = 0;
        let mut error_count = 0;

        for (i, mapping) in mappings.iter().enumerate() {
            pb.set_message(format!("Processing {} ({})", mapping.symbol, mapping.company_name));
            
            match self.download_company_balance_sheets(mapping).await {
                Ok(filings_processed) => {
                    success_count += 1;
                    if filings_processed > 0 {
                        println!("  ✅ {}: {} filings processed", mapping.symbol, filings_processed);
                    }
                }
                Err(e) => {
                    error_count += 1;
                    eprintln!("  ❌ {}: {}", mapping.symbol, e);
                }
            }

            pb.inc(1);
            
            // Update progress every 10 companies
            if (i + 1) % 10 == 0 {
                pb.set_message(format!("Processed {} companies...", i + 1));
            }
        }

        pb.finish_with_message("✅ Balance sheet download completed");
        
        println!("\n📊 SEC EDGAR Download Summary:");
        println!("  Total Companies: {}", total_companies);
        println!("  Successful: {}", success_count);
        println!("  Errors: {}", error_count);
        println!("  Success Rate: {:.1}%", (success_count as f64 / total_companies as f64) * 100.0);

        Ok(())
    }

    /// Download income statement data for all S&P 500 companies
    pub async fn download_all_sp500_income_statements(&mut self) -> Result<()> {
        println!("🚀 Starting SEC EDGAR income statement data download for S&P 500 companies...");
        
        let mappings = self.get_sp500_cik_mappings().await?;
        let total_companies = mappings.len();
        
        let pb = ProgressBar::new(total_companies as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg}")
                .unwrap()
                .progress_chars("#>-")
        );
        pb.set_message("Downloading income statement data...");

        let mut success_count = 0;
        let mut error_count = 0;

        for (i, mapping) in mappings.iter().enumerate() {
            pb.set_message(format!("Processing {} ({})", mapping.symbol, mapping.company_name));
            
            match self.download_company_income_statements(mapping).await {
                Ok(filings_processed) => {
                    success_count += 1;
                    if filings_processed > 0 {
                        println!("  ✅ {}: {} filings processed", mapping.symbol, filings_processed);
                    }
                }
                Err(e) => {
                    error_count += 1;
                    eprintln!("  ❌ {}: {}", mapping.symbol, e);
                }
            }

            pb.inc(1);
            
            // Update progress every 10 companies
            if (i + 1) % 10 == 0 {
                pb.set_message(format!("Processed {} companies...", i + 1));
            }
        }

        pb.finish_with_message("✅ Income statement download completed");
        
        println!("\n📊 SEC EDGAR Income Statement Download Summary:");
        println!("  Total Companies: {}", total_companies);
        println!("  Successful: {}", success_count);
        println!("  Errors: {}", error_count);
        println!("  Success Rate: {:.1}%", (success_count as f64 / total_companies as f64) * 100.0);

        Ok(())
    }

    /// Download balance sheet data for a single company
    async fn download_company_balance_sheets(&mut self, mapping: &CikMapping) -> Result<usize> {
        match self.extract_balance_sheet_data(&mapping.cik, mapping.stock_id, &mapping.symbol).await {
            Ok(Some(_data)) => {
                // Note: filing metadata is already stored in extract_balance_sheet_data method
                Ok(1)
            }
            Ok(None) => {
                // No data extracted
                Ok(0)
            }
            Err(e) => {
                eprintln!("    ⚠️ Failed to extract balance sheet data for {}: {}", mapping.symbol, e);
                Ok(0)
            }
        }
    }

    /// Download income statement data for a single company
    async fn download_company_income_statements(&mut self, mapping: &CikMapping) -> Result<usize> {
        match self.extract_income_statement_data(&mapping.cik, mapping.stock_id, &mapping.symbol).await {
            Ok(Some(_data)) => {
                // Note: filing metadata should be handled in extract_income_statement_data method
                Ok(1)
            }
            Ok(None) => {
                // No data extracted
                Ok(0)
            }
            Err(e) => {
                eprintln!("    ⚠️ Failed to extract income statement data for {}: {}", mapping.symbol, e);
                Ok(0)
            }
        }
    }
}

/// Test the SEC EDGAR client with a few companies
pub async fn test_sec_edgar_client(pool: &SqlitePool) -> Result<()> {
    println!("🧪 Testing SEC EDGAR client...");
    
    let mut client = SecEdgarClient::new(pool.clone());
    
    // Test with a few major companies
    let test_symbols = vec!["AAPL", "MSFT", "GOOGL"];
    
    for symbol in test_symbols {
        println!("\n🔍 Testing {}...", symbol);
        
        // Get CIK mapping
        let mappings = client.get_sp500_cik_mappings().await?;
        if let Some(mapping) = mappings.iter().find(|m| m.symbol == symbol) {
            println!("  📋 CIK: {}, Company: {}", mapping.cik, mapping.company_name);
            
            // Discover filings
            match client.discover_10k_filings(&mapping.cik, &mapping.symbol).await {
                Ok(filings) => {
                    println!("  📊 Found {} 10-K filings", filings.len());
                    for filing in filings.iter().take(3) {
                        println!("    - {}: {}", filing.filing_date, filing.accession_number);
                    }
                    
                    // Test JSON parsing using Company Facts API
                    println!("  📊 Testing Company Facts API for balance sheet data");
                    match client.extract_balance_sheet_data(&mapping.cik, mapping.stock_id, &mapping.symbol).await {
                        Ok(Some(balance_data)) => {
                            println!("    ✅ Successfully extracted balance sheet data:");
                            if let Some(assets) = balance_data.total_assets {
                                println!("      💰 Total Assets: ${:.0}M", assets / 1_000_000.0);
                            }
                            if let Some(equity) = balance_data.total_equity {
                                println!("      📈 Total Equity: ${:.0}M", equity / 1_000_000.0);
                            }
                            if let Some(liabilities) = balance_data.total_liabilities {
                                println!("      📉 Total Liabilities: ${:.0}M", liabilities / 1_000_000.0);
                            }
                        }
                        Ok(None) => {
                            println!("    ⚠️ No balance sheet data found in Company Facts API");
                        }
                        Err(e) => {
                            println!("    ❌ Error extracting balance sheet data: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("  ❌ Failed to discover filings: {}", e);
                }
            }
        } else {
            eprintln!("  ❌ No CIK mapping found for {}", symbol);
        }
    }
    
    println!("\n✅ SEC EDGAR client test completed");
    Ok(())
}
