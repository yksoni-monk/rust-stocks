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
    pub data_source: String,
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
    pub data_source: String,
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

    /// Get all CIK mappings for S&P 500 companies
    pub async fn get_sp500_cik_mappings(&self) -> Result<Vec<CikMapping>> {
        let query = r#"
            SELECT cik, stock_id, symbol, company_name
            FROM cik_mappings_sp500
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

    /// Extract balance sheet data using SEC EDGAR Company Facts API
    pub async fn extract_balance_sheet_data(&mut self, cik: &str, stock_id: i64, symbol: &str) -> Result<Option<BalanceSheetData>> {
        self.rate_limiter.wait_if_needed().await;

        println!("  📊 Extracting balance sheet data for {} using Company Facts API", symbol);
        
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
        
        // Extract balance sheet data from JSON
        let balance_sheet_data = self.parse_company_facts_json(&json, symbol)?;
        
        if balance_sheet_data.is_empty() {
            println!("    ⚠️ No balance sheet data found for {}", symbol);
            return Ok(None);
        }

        // Get the most recent fiscal year
        let current_year = Utc::now().year();
        let fiscal_year = current_year - 1; // Most recent completed fiscal year
        
        Ok(Some(BalanceSheetData {
            stock_id,
            symbol: symbol.to_string(),
            report_date: NaiveDate::from_ymd_opt(fiscal_year, 12, 31).unwrap_or_default(),
            fiscal_year,
            total_assets: balance_sheet_data.get("Assets").copied(),
            total_liabilities: balance_sheet_data.get("Liabilities").copied(),
            total_equity: balance_sheet_data.get("StockholdersEquity").copied(),
            cash_and_equivalents: balance_sheet_data.get("CashAndCashEquivalentsAtCarryingValue").copied(),
            short_term_debt: balance_sheet_data.get("ShortTermDebt").copied(),
            long_term_debt: balance_sheet_data.get("LongTermDebt").copied(),
            total_debt: balance_sheet_data.get("Debt").copied(),
            data_source: "sec_edgar_json".to_string(),
        }))
    }

    /// Parse Company Facts JSON to extract balance sheet data
    fn parse_company_facts_json(&self, json: &serde_json::Value, symbol: &str) -> Result<HashMap<String, f64>> {
        let mut balance_sheet_data = HashMap::new();
        
        // Balance sheet field mappings (US GAAP taxonomy)
        let field_mappings = [
            ("Assets", "Assets"),
            ("AssetsCurrent", "AssetsCurrent"),
            ("Liabilities", "Liabilities"),
            ("LiabilitiesCurrent", "LiabilitiesCurrent"),
            ("StockholdersEquity", "StockholdersEquity"),
            ("CashAndCashEquivalentsAtCarryingValue", "CashAndCashEquivalentsAtCarryingValue"),
            ("ShortTermDebt", "ShortTermDebt"),
            ("LongTermDebt", "LongTermDebt"),
            ("Debt", "Debt"),
        ];

        // Navigate to the facts section
        if let Some(facts) = json.get("facts").and_then(|f| f.get("us-gaap")) {
            for (field_name, our_field) in &field_mappings {
                if let Some(field_data) = facts.get(field_name) {
                    if let Some(units) = field_data.get("units") {
                        if let Some(usd_data) = units.get("USD") {
                            if let Some(values) = usd_data.as_array() {
                                // Get the most recent value
                                if let Some(latest_value) = values.last() {
                                    if let Some(val) = latest_value.get("val").and_then(|v| v.as_f64()) {
                                        if val != 0.0 {
                                            balance_sheet_data.insert(our_field.to_string(), val);
                                            println!("    📊 Found {}: ${:.0}M", field_name, val / 1_000_000.0);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(balance_sheet_data)
    }

    /// Parse Company Facts JSON to extract income statement data including shares outstanding
    fn parse_income_statement_json(&self, json: &serde_json::Value, symbol: &str) -> Result<HashMap<String, f64>> {
        let mut income_data = HashMap::new();
        
        // Income statement field mappings (US GAAP taxonomy)
        let field_mappings = [
            ("Revenues", "revenue"),
            ("RevenueFromContractWithCustomerExcludingAssessedTax", "revenue"),
            ("SalesRevenueNet", "revenue"),
            ("NetIncomeLoss", "net_income"),
            ("OperatingIncomeLoss", "operating_income"),
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
        ];

        // Navigate to the facts section
        if let Some(facts) = json.get("facts").and_then(|f| f.get("us-gaap")) {
            // Extract income statement data
            for (field_name, our_field) in &field_mappings {
                if let Some(field_data) = facts.get(field_name) {
                    if let Some(units) = field_data.get("units") {
                        if let Some(usd_data) = units.get("USD") {
                            if let Some(values) = usd_data.as_array() {
                                // Get the most recent value
                                if let Some(latest_value) = values.last() {
                                    if let Some(val) = latest_value.get("val").and_then(|v| v.as_f64()) {
                                        if val != 0.0 {
                                            income_data.insert(our_field.to_string(), val);
                                            println!("    📈 Found {}: ${:.0}M", field_name, val / 1_000_000.0);
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
                                // Get the most recent value
                                if let Some(latest_value) = values.last() {
                                    if let Some(val) = latest_value.get("val").and_then(|v| v.as_f64()) {
                                        if val > 0.0 {
                                            income_data.insert(our_field.to_string(), val);
                                            println!("    📊 Found {}: {:.0}M shares", field_name, val / 1_000_000.0);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(income_data)
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
        let income_data = self.parse_income_statement_json(&json, symbol)?;

        if income_data.is_empty() {
            println!("    ⚠️ No income statement data found for {}", symbol);
            return Ok(None);
        }

        // Get the most recent fiscal year (simplified for now, will refine for historical data)
        let current_year = Utc::now().year();
        let fiscal_year = current_year - 1; // Most recent completed fiscal year

        Ok(Some(IncomeStatementData {
            stock_id,
            symbol: symbol.to_string(),
            report_date: NaiveDate::from_ymd_opt(fiscal_year, 12, 31).unwrap_or_default(), // Placeholder date
            fiscal_year,
            period_type: "TTM".to_string(),
            revenue: income_data.get("revenue").copied(),
            net_income: income_data.get("net_income").copied(),
            operating_income: income_data.get("operating_income").copied(),
            gross_profit: income_data.get("gross_profit").copied(),
            cost_of_revenue: income_data.get("cost_of_revenue").copied(),
            interest_expense: income_data.get("interest_expense").copied(),
            tax_expense: income_data.get("tax_expense").copied(),
            shares_basic: income_data.get("shares_basic").copied(),
            shares_diluted: income_data.get("shares_diluted").copied(),
            data_source: "sec_edgar_json".to_string(),
        }))
    }

    /// Store balance sheet data in the database
    pub async fn store_balance_sheet_data(&self, data: &BalanceSheetData) -> Result<()> {
        let query = r#"
            INSERT OR REPLACE INTO balance_sheets (
                stock_id, period_type, report_date, fiscal_year,
                total_assets, total_liabilities, total_equity,
                cash_and_equivalents, short_term_debt, long_term_debt, total_debt,
                data_source, created_at
            ) VALUES (
                ?1, 'Annual', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, CURRENT_TIMESTAMP
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
            .bind(&data.data_source)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Store income statement data in the database
    pub async fn store_income_statement_data(&self, data: &IncomeStatementData) -> Result<()> {
        let query = r#"
            INSERT OR REPLACE INTO income_statements (
                stock_id, period_type, report_date, fiscal_year,
                revenue, gross_profit, operating_income, net_income,
                shares_basic, shares_diluted, currency, data_source, created_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'USD', ?11, CURRENT_TIMESTAMP
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
            .bind(&data.data_source)
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
            Ok(Some(data)) => {
                self.store_balance_sheet_data(&data).await?;
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
            Ok(Some(data)) => {
                self.store_income_statement_data(&data).await?;
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
