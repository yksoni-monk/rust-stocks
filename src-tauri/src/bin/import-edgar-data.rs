/// EDGAR Financial Data Extraction Tool
/// 
/// Extracts financial statements from local EDGAR JSON files and imports
/// them into the database with comprehensive mapping, validation, and
/// progress tracking.

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::fs;
use tokio::fs as async_fs;
use tracing::{info, warn, debug};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    /// Path to EDGAR companyfacts directory
    #[arg(long, default_value = "/Users/yksoni/code/misc/rust-stocks/edgar_data/companyfacts")]
    edgar_path: String,
    
    /// Test mode - process only one CIK file
    #[arg(long)]
    test_cik: Option<String>,
    
    /// Resume previous incomplete extraction
    #[arg(long)]
    resume: bool,
    
    /// Validate existing data without extraction
    #[arg(long)]
    validate_only: bool,
    
    /// Progress file path
    #[arg(long, default_value = "edgar_extraction_progress.json")]
    progress_file: String,
    
    /// Batch size for database operations
    #[arg(long, default_value = "500")]
    batch_size: usize,
    
    /// CIK range for partial processing (e.g., "1000-2000")
    #[arg(long)]
    cik_range: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract financial data from all EDGAR files
    Extract,
    /// Resume an interrupted extraction
    Resume,
    /// Validate data quality
    Validate,
    /// Show progress of current/last extraction
    Status,
    /// Scan EDGAR files and show inventory
    Scan,
}

// SEC Company Tickers JSON structure
#[derive(Debug, Deserialize)]
struct SecCompanyTickers {
    _fields: Vec<String>,
    data: Vec<Vec<serde_json::Value>>,
}

// EDGAR JSON structures
#[derive(Debug, Deserialize)]
struct EdgarCompanyFacts {
    cik: i64,
    #[serde(rename = "entityName")]
    entity_name: String,
    facts: EdgarFacts,
}

#[derive(Debug, Deserialize)]
struct EdgarFacts {
    #[serde(rename = "us-gaap")]
    us_gaap: HashMap<String, EdgarConcept>,
}

#[derive(Debug, Deserialize)]
struct EdgarConcept {
    units: HashMap<String, Vec<EdgarFactValue>>,
}

#[derive(Debug, Deserialize)]
struct EdgarFactValue {
    end: String,  // Date in YYYY-MM-DD format
    val: f64,
    _form: Option<String>,  // "10-K", "10-Q", etc. (can be null)
    fy: Option<i32>,      // Fiscal year (can be null)
    fp: Option<String>,   // Fiscal period "Q1", "Q2", "Q3", "Q4", "FY" (can be null)
}

// Progress tracking structures
#[derive(Debug, Serialize, Deserialize, Clone)]
struct ExtractionProgress {
    session_id: String,
    start_time: DateTime<Utc>,
    total_files: usize,
    processed_files: usize,
    successful_extractions: usize,
    failed_extractions: usize,
    current_file: Option<String>,
    cik_mapping_stats: CikMappingStats,
    data_quality_stats: DataQualityStats,
    settings: ExtractionSettings,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct CikMappingStats {
    mapped_ciks: usize,
    unmapped_ciks: usize,
    mapping_confidence: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct DataQualityStats {
    complete_income_statements: usize,
    complete_balance_sheets: usize,
    validation_warnings: usize,
    validation_errors: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ExtractionSettings {
    edgar_path: String,
    batch_size: usize,
    test_mode: bool,
    cik_range: Option<String>,
}

impl Default for CikMappingStats {
    fn default() -> Self {
        Self {
            mapped_ciks: 0,
            unmapped_ciks: 0,
            mapping_confidence: 0.0,
        }
    }
}

impl Default for DataQualityStats {
    fn default() -> Self {
        Self {
            complete_income_statements: 0,
            complete_balance_sheets: 0,
            validation_warnings: 0,
            validation_errors: 0,
        }
    }
}

// Extracted financial data structures
#[derive(Debug)]
struct ExtractedFinancialData {
    income_statements: Vec<IncomeStatementData>,
    balance_sheets: Vec<BalanceSheetData>,
}

#[derive(Debug, Clone)]
struct PeriodInfo {
    year: i32,
    period: String,
    end_date: String,
}

#[derive(Debug)]
struct IncomeStatementData {
    stock_id: i64,
    period: String,  // "Q1", "Q2", "Q3", "Q4", "FY"
    year: i32,
    end_date: String,
    revenue: Option<f64>,
    net_income: Option<f64>,
    operating_income: Option<f64>,
    shares_basic: Option<f64>,
    shares_diluted: Option<f64>,
}

#[derive(Debug)]
struct BalanceSheetData {
    stock_id: i64,
    period: String,
    year: i32,
    end_date: String,
    total_assets: Option<f64>,
    total_debt: Option<f64>,
    total_equity: Option<f64>,
    cash_and_equivalents: Option<f64>,
    shares_outstanding: Option<f64>,
}

// Core extractor structure
struct EdgarExtractor {
    db_pool: SqlitePool,
    edgar_path: PathBuf,
    progress: ExtractionProgress,
    progress_file: PathBuf,
    cik_symbol_map: HashMap<String, String>,
    symbol_stock_id_map: HashMap<String, i64>,
    gaap_mapping: GaapFieldMapping,
}

// GAAP field mapping configuration
struct GaapFieldMapping {
    income_statement_fields: HashMap<String, Vec<String>>,
    balance_sheet_fields: HashMap<String, Vec<String>>,
}

impl GaapFieldMapping {
    fn new() -> Self {
        let mut income_statement_fields = HashMap::new();
        let mut balance_sheet_fields = HashMap::new();
        
        // Income statement field mappings (priority order)
        income_statement_fields.insert("revenue".to_string(), vec![
            "RevenueFromContractWithCustomerExcludingAssessedTax".to_string(),
            "SalesRevenueNet".to_string(),
            "Revenues".to_string(),
            "RevenueFromContractWithCustomerIncludingAssessedTax".to_string(),
        ]);
        
        income_statement_fields.insert("net_income".to_string(), vec![
            "NetIncomeLoss".to_string(),
            "NetIncomeLossAvailableToCommonStockholdersBasic".to_string(),
            "ProfitLoss".to_string(),
        ]);
        
        income_statement_fields.insert("operating_income".to_string(), vec![
            "IncomeLossFromContinuingOperations".to_string(),
            "OperatingIncomeLoss".to_string(),
            "IncomeLossFromContinuingOperationsBeforeIncomeTaxesExtraordinaryItemsNoncontrollingInterest".to_string(),
        ]);
        
        income_statement_fields.insert("shares_basic".to_string(), vec![
            "WeightedAverageNumberOfSharesOutstandingBasic".to_string(),
            "CommonStockSharesOutstanding".to_string(),
        ]);
        
        income_statement_fields.insert("shares_diluted".to_string(), vec![
            "WeightedAverageNumberOfDilutedSharesOutstanding".to_string(),
            "WeightedAverageNumberOfSharesOutstandingBasic".to_string(),
        ]);
        
        // Balance sheet field mappings (priority order)
        balance_sheet_fields.insert("total_assets".to_string(), vec![
            "Assets".to_string(),
            "AssetsTotal".to_string(),
        ]);
        
        balance_sheet_fields.insert("total_debt".to_string(), vec![
            "LongTermDebt".to_string(),
            "DebtAndCapitalLeaseObligations".to_string(),
            "LongTermDebtAndCapitalLeaseObligations".to_string(),
        ]);
        
        balance_sheet_fields.insert("total_equity".to_string(), vec![
            "StockholdersEquity".to_string(),
            "ShareholdersEquity".to_string(),
            "StockholdersEquityIncludingPortionAttributableToNoncontrollingInterest".to_string(),
        ]);
        
        balance_sheet_fields.insert("cash_and_equivalents".to_string(), vec![
            "CashAndCashEquivalentsAtCarryingValue".to_string(),
            "CashCashEquivalentsAndShortTermInvestments".to_string(),
            "Cash".to_string(),
        ]);
        
        balance_sheet_fields.insert("shares_outstanding".to_string(), vec![
            "CommonStockSharesOutstanding".to_string(),
            "CommonStockSharesIssued".to_string(),
        ]);
        
        Self {
            income_statement_fields,
            balance_sheet_fields,
        }
    }
}

impl EdgarExtractor {
    async fn new(
        edgar_path: PathBuf,
        progress_file: PathBuf,
        settings: ExtractionSettings,
    ) -> Result<Self> {
        // Connect to database
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite:./db/stocks.db".to_string());
        let db_pool = SqlitePool::connect(&database_url).await?;
        
        // Initialize or load progress
        let progress = if progress_file.exists() {
            Self::load_progress(&progress_file).await?
        } else {
            let file_count = Self::count_edgar_files(&edgar_path)?;
            ExtractionProgress {
                session_id: uuid::Uuid::new_v4().to_string(),
                start_time: Utc::now(),
                total_files: file_count,
                processed_files: 0,
                successful_extractions: 0,
                failed_extractions: 0,
                current_file: None,
                cik_mapping_stats: CikMappingStats::default(),
                data_quality_stats: DataQualityStats::default(),
                settings,
            }
        };
        
        // Build CIK-to-symbol mapping
        let (cik_symbol_map, symbol_stock_id_map) = Self::build_cik_symbol_mapping(&db_pool).await?;
        
        // Initialize GAAP field mapping
        let gaap_mapping = GaapFieldMapping::new();
        
        Ok(Self {
            db_pool,
            edgar_path,
            progress,
            progress_file,
            cik_symbol_map,
            symbol_stock_id_map,
            gaap_mapping,
        })
    }
    
    fn count_edgar_files(edgar_path: &Path) -> Result<usize> {
        let count = fs::read_dir(edgar_path)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.file_name()
                    .to_string_lossy()
                    .starts_with("CIK") && 
                entry.file_name()
                    .to_string_lossy()
                    .ends_with(".json")
            })
            .count();
        
        info!("Found {} EDGAR CIK files in {}", count, edgar_path.display());
        Ok(count)
    }
    
    async fn load_progress(progress_file: &Path) -> Result<ExtractionProgress> {
        let content = async_fs::read_to_string(progress_file).await?;
        let progress: ExtractionProgress = serde_json::from_str(&content)?;
        info!("Loaded existing progress: {}/{} files processed", 
               progress.processed_files, progress.total_files);
        Ok(progress)
    }
    
    async fn save_progress(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.progress)?;
        async_fs::write(&self.progress_file, content).await?;
        Ok(())
    }
    
    async fn build_cik_symbol_mapping(db_pool: &SqlitePool) -> Result<(HashMap<String, String>, HashMap<String, i64>)> {
        info!("Building CIK-to-symbol mapping from SEC company tickers file...");
        
        // Load all stocks from database for symbol_stock_id mapping
        let stocks = sqlx::query_as::<_, (i64, String, String)>(
            "SELECT id, symbol, company_name FROM stocks WHERE is_sp500 = 1 ORDER BY symbol"
        )
        .fetch_all(db_pool)
        .await?;
        
        let mut symbol_stock_id_map = HashMap::new();
        
        // Build symbol to stock_id mapping for S&P 500 stocks
        for (id, symbol, _company_name) in &stocks {
            symbol_stock_id_map.insert(symbol.clone(), *id);
        }
        
        // Load existing CIK mappings from database
        let existing_mappings = sqlx::query_as::<_, (String, String)>(
            "SELECT cik, symbol FROM cik_mappings"
        )
        .fetch_all(db_pool)
        .await?;
        
        let mut cik_symbol_map = HashMap::new();
        for (cik, symbol) in existing_mappings {
            cik_symbol_map.insert(cik, symbol);
        }
        
        let db_count = cik_symbol_map.len();
        
        // Load CIK mappings from SEC company_tickers_exchange.json
        let sec_count = Self::load_sec_company_tickers(&mut cik_symbol_map, &symbol_stock_id_map).await?;
        
        info!("Built CIK mapping: {} from DB + {} from SEC = {} total mappings, {} S&P 500 stocks", 
               db_count, sec_count, cik_symbol_map.len(), stocks.len());
        
        Ok((cik_symbol_map, symbol_stock_id_map))
    }
    
    async fn load_sec_company_tickers(
        cik_symbol_map: &mut HashMap<String, String>,
        symbol_stock_id_map: &HashMap<String, i64>
    ) -> Result<usize> {
        let tickers_file_path = "/Users/yksoni/code/misc/rust-stocks/edgar_data/company_tickers_exchange.json";
        
        if !Path::new(tickers_file_path).exists() {
            warn!("SEC company tickers file not found at {}", tickers_file_path);
            return Ok(0);
        }
        
        // Read and parse the SEC company tickers JSON
        let content = fs::read_to_string(tickers_file_path)?;
        let sec_tickers: SecCompanyTickers = serde_json::from_str(&content)?;
        
        let mut added_count = 0;
        
        // Process each company entry
        for entry in &sec_tickers.data {
            if entry.len() >= 3 {
                // Extract CIK, name, and ticker from the data array
                if let (Some(cik_val), Some(ticker_val)) = (entry.get(0), entry.get(2)) {
                    if let (Some(cik_num), Some(ticker_str)) = (cik_val.as_i64(), ticker_val.as_str()) {
                        let cik = cik_num.to_string();
                        let ticker = ticker_str.to_string();
                        
                        // Only add if we don't already have this CIK and the ticker exists in our S&P 500 stocks
                        if !cik_symbol_map.contains_key(&cik) && symbol_stock_id_map.contains_key(&ticker) {
                            cik_symbol_map.insert(cik, ticker);
                            added_count += 1;
                        }
                    }
                }
            }
        }
        
        Ok(added_count)
    }
    
    
    async fn extract_financial_statements(
        &self, 
        edgar_data: &EdgarCompanyFacts, 
        stock_id: i64
    ) -> Result<ExtractedFinancialData> {
        let mut income_statements = Vec::new();
        let mut balance_sheets = Vec::new();
        
        // Extract periods from GAAP facts
        let periods = self.extract_available_periods(&edgar_data.facts.us_gaap)?;
        
        for period_info in periods {
            // Extract income statement data for this period
            if let Ok(income_stmt) = self.extract_income_statement_for_period(
                &edgar_data.facts.us_gaap, 
                stock_id, 
                &period_info
            ) {
                income_statements.push(income_stmt);
            }
            
            // Extract balance sheet data for this period
            if let Ok(balance_sheet) = self.extract_balance_sheet_for_period(
                &edgar_data.facts.us_gaap, 
                stock_id, 
                &period_info
            ) {
                balance_sheets.push(balance_sheet);
            }
        }
        
        Ok(ExtractedFinancialData {
            income_statements,
            balance_sheets,
        })
    }
    
    fn extract_available_periods(&self, gaap_facts: &HashMap<String, EdgarConcept>) -> Result<Vec<PeriodInfo>> {
        let mut periods = Vec::new();
        let mut seen_periods = HashSet::new();
        
        // Look through all fields to find available periods
        for (_field_name, concept) in gaap_facts {
            if let Some(usd_values) = concept.units.get("USD") {
                for fact_value in usd_values {
                    if let (Some(fy), Some(fp)) = (fact_value.fy, fact_value.fp.as_ref()) {
                        let period_key = format!("{}-{}", fy, fp);
                        if !seen_periods.contains(&period_key) {
                            seen_periods.insert(period_key);
                            periods.push(PeriodInfo {
                                year: fy,
                                period: fp.clone(),
                                end_date: fact_value.end.clone(),
                            });
                        }
                    }
                }
            }
        }
        
        // Sort periods by year and quarter
        periods.sort_by(|a, b| {
            a.year.cmp(&b.year).then_with(|| {
                let order_a = match a.period.as_str() {
                    "Q1" => 1, "Q2" => 2, "Q3" => 3, "Q4" => 4, "FY" => 5,
                    _ => 99,
                };
                let order_b = match b.period.as_str() {
                    "Q1" => 1, "Q2" => 2, "Q3" => 3, "Q4" => 4, "FY" => 5,
                    _ => 99,
                };
                order_a.cmp(&order_b)
            })
        });
        
        Ok(periods)
    }
    
    fn extract_income_statement_for_period(
        &self,
        gaap_facts: &HashMap<String, EdgarConcept>,
        stock_id: i64,
        period_info: &PeriodInfo,
    ) -> Result<IncomeStatementData> {
        let mut income_stmt = IncomeStatementData {
            stock_id,
            period: period_info.period.clone(),
            year: period_info.year,
            end_date: period_info.end_date.clone(),
            revenue: None,
            net_income: None,
            operating_income: None,
            shares_basic: None,
            shares_diluted: None,
        };
        
        // Extract revenue using priority field mapping
        income_stmt.revenue = self.extract_field_value_for_period(
            gaap_facts,
            &self.gaap_mapping.income_statement_fields["revenue"],
            period_info,
        );
        
        // Extract net income
        income_stmt.net_income = self.extract_field_value_for_period(
            gaap_facts,
            &self.gaap_mapping.income_statement_fields["net_income"],
            period_info,
        );
        
        // Extract operating income
        income_stmt.operating_income = self.extract_field_value_for_period(
            gaap_facts,
            &self.gaap_mapping.income_statement_fields["operating_income"],
            period_info,
        );
        
        // Extract basic shares
        income_stmt.shares_basic = self.extract_field_value_for_period(
            gaap_facts,
            &self.gaap_mapping.income_statement_fields["shares_basic"],
            period_info,
        );
        
        // Extract diluted shares
        income_stmt.shares_diluted = self.extract_field_value_for_period(
            gaap_facts,
            &self.gaap_mapping.income_statement_fields["shares_diluted"],
            period_info,
        );
        
        Ok(income_stmt)
    }
    
    fn extract_balance_sheet_for_period(
        &self,
        gaap_facts: &HashMap<String, EdgarConcept>,
        stock_id: i64,
        period_info: &PeriodInfo,
    ) -> Result<BalanceSheetData> {
        let mut balance_sheet = BalanceSheetData {
            stock_id,
            period: period_info.period.clone(),
            year: period_info.year,
            end_date: period_info.end_date.clone(),
            total_assets: None,
            total_debt: None,
            total_equity: None,
            cash_and_equivalents: None,
            shares_outstanding: None,
        };
        
        // Extract total assets
        balance_sheet.total_assets = self.extract_field_value_for_period(
            gaap_facts,
            &self.gaap_mapping.balance_sheet_fields["total_assets"],
            period_info,
        );
        
        // Extract total debt
        balance_sheet.total_debt = self.extract_field_value_for_period(
            gaap_facts,
            &self.gaap_mapping.balance_sheet_fields["total_debt"],
            period_info,
        );
        
        // Extract total equity
        balance_sheet.total_equity = self.extract_field_value_for_period(
            gaap_facts,
            &self.gaap_mapping.balance_sheet_fields["total_equity"],
            period_info,
        );
        
        // Extract cash and equivalents
        balance_sheet.cash_and_equivalents = self.extract_field_value_for_period(
            gaap_facts,
            &self.gaap_mapping.balance_sheet_fields["cash_and_equivalents"],
            period_info,
        );
        
        // Extract shares outstanding
        balance_sheet.shares_outstanding = self.extract_field_value_for_period(
            gaap_facts,
            &self.gaap_mapping.balance_sheet_fields["shares_outstanding"],
            period_info,
        );
        
        Ok(balance_sheet)
    }
    
    fn extract_field_value_for_period(
        &self,
        gaap_facts: &HashMap<String, EdgarConcept>,
        field_priorities: &[String],
        period_info: &PeriodInfo,
    ) -> Option<f64> {
        // Try each field in priority order
        for field_name in field_priorities {
            if let Some(concept) = gaap_facts.get(field_name) {
                if let Some(usd_values) = concept.units.get("USD") {
                    // Find the value for this specific period
                    for fact_value in usd_values {
                        if fact_value.fy == Some(period_info.year) && 
                           fact_value.fp.as_ref() == Some(&period_info.period) &&
                           fact_value.end == period_info.end_date {
                            return Some(fact_value.val);
                        }
                    }
                }
            }
        }
        None
    }
    
    async fn insert_financial_data_to_db(&self, data: &ExtractedFinancialData) -> Result<()> {
        let mut tx = self.db_pool.begin().await?;
        
        // Insert income statements
        for income_stmt in &data.income_statements {
            sqlx::query(
                r#"
                INSERT OR REPLACE INTO income_statements 
                (stock_id, period, year, end_date, revenue, net_income, operating_income, shares_basic, shares_diluted)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#
            )
            .bind(income_stmt.stock_id)
            .bind(&income_stmt.period)
            .bind(income_stmt.year)
            .bind(&income_stmt.end_date)
            .bind(income_stmt.revenue)
            .bind(income_stmt.net_income)
            .bind(income_stmt.operating_income)
            .bind(income_stmt.shares_basic)
            .bind(income_stmt.shares_diluted)
            .execute(&mut *tx)
            .await?;
        }
        
        // Insert balance sheets
        for balance_sheet in &data.balance_sheets {
            sqlx::query(
                r#"
                INSERT OR REPLACE INTO balance_sheets 
                (stock_id, period, year, end_date, total_assets, total_debt, total_equity, cash_and_equivalents, shares_outstanding)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#
            )
            .bind(balance_sheet.stock_id)
            .bind(&balance_sheet.period)
            .bind(balance_sheet.year)
            .bind(&balance_sheet.end_date)
            .bind(balance_sheet.total_assets)
            .bind(balance_sheet.total_debt)
            .bind(balance_sheet.total_equity)
            .bind(balance_sheet.cash_and_equivalents)
            .bind(balance_sheet.shares_outstanding)
            .execute(&mut *tx)
            .await?;
        }
        
        tx.commit().await?;
        Ok(())
    }
    
    async fn _extract_entity_name_from_file(file_path: &std::path::Path) -> Result<String> {
        // Read just enough of the JSON to get the entity name
        let content = async_fs::read_to_string(file_path).await?;
        
        // Parse just the entityName field for efficiency
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(entity_name) = parsed.get("entityName").and_then(|v| v.as_str()) {
                return Ok(entity_name.to_string());
            }
        }
        
        Err(anyhow!("Could not extract entity name from file"))
    }
    
    async fn scan_edgar_files(&self) -> Result<()> {
        println!("🔍 Scanning EDGAR files...");
        println!("Path: {}", self.edgar_path.display());
        
        let entries = fs::read_dir(&self.edgar_path)?;
        let mut file_count = 0;
        let mut mapped_count = 0;
        let mut sample_files = Vec::new();
        
        for entry in entries {
            let entry = entry?;
            let filename = entry.file_name();
            let filename_str = filename.to_string_lossy();
            
            if filename_str.starts_with("CIK") && filename_str.ends_with(".json") {
                file_count += 1;
                
                // Extract CIK from filename
                if let Some(cik) = Self::extract_cik_from_filename(&filename_str) {
                    if self.cik_symbol_map.contains_key(&cik) {
                        mapped_count += 1;
                    }
                    
                    // Collect sample files for display
                    if sample_files.len() < 10 {
                        sample_files.push((filename_str.to_string(), cik));
                    }
                }
            }
        }
        
        println!("📊 EDGAR File Inventory:");
        println!("   Total CIK files: {}", file_count);
        println!("   Mapped to symbols: {} ({:.1}%)", mapped_count, 
                 (mapped_count as f64 / file_count as f64) * 100.0);
        println!("   Unmapped files: {}", file_count - mapped_count);
        
        println!("\n📋 Sample files:");
        for (filename, cik) in sample_files {
            let status = if self.cik_symbol_map.contains_key(&cik) {
                format!("→ {}", self.cik_symbol_map[&cik])
            } else {
                "→ UNMAPPED".to_string()
            };
            println!("   {} {}", filename, status);
        }
        
        Ok(())
    }
    
    fn extract_cik_from_filename(filename: &str) -> Option<String> {
        // Extract CIK from "CIK0000320193.json" format
        if let Some(start) = filename.find("CIK") {
            let cik_part = &filename[start + 3..];
            if let Some(end) = cik_part.find('.') {
                let cik_str = &cik_part[..end];
                // Remove leading zeros and return
                return Some(cik_str.trim_start_matches('0').to_string());
            }
        }
        None
    }
    
    async fn extract_all_files(&mut self) -> Result<()> {
        println!("🏗️ EDGAR Financial Data Extraction");
        println!("===================================");
        println!("Total files: {}", self.progress.total_files);
        println!("EDGAR path: {}", self.edgar_path.display());
        println!("Progress file: {}", self.progress_file.display());
        println!();
        
        let entries = fs::read_dir(&self.edgar_path)?;
        let mut edgar_files: Vec<PathBuf> = entries
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                let file_name = entry.file_name();
                let filename = file_name.to_string_lossy();
                filename.starts_with("CIK") && filename.ends_with(".json")
            })
            .map(|entry| entry.path())
            .collect();
        
        edgar_files.sort();
        
        for (index, file_path) in edgar_files.iter().enumerate() {
            let filename = file_path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown");
            
            self.progress.current_file = Some(filename.to_string());
            
            // Show progress
            self.display_progress(index, edgar_files.len());
            
            // Extract data from this file
            match self.extract_file_data(file_path).await {
                Ok(_) => {
                    self.progress.successful_extractions += 1;
                    debug!("✅ Processed: {}", filename);
                }
                Err(e) => {
                    self.progress.failed_extractions += 1;
                    warn!("❌ Failed: {} - {}", filename, e);
                }
            }
            
            self.progress.processed_files += 1;
            
            // Save progress periodically
            if index % 100 == 0 {
                if let Err(e) = self.save_progress().await {
                    warn!("Failed to save progress: {}", e);
                }
            }
        }
        
        // Final progress save
        self.save_progress().await?;
        self.display_final_summary();
        
        Ok(())
    }
    
    async fn extract_file_data(&mut self, file_path: &Path) -> Result<()> {
        // Read and parse JSON file
        let content = async_fs::read_to_string(file_path).await?;
        let edgar_data: EdgarCompanyFacts = serde_json::from_str(&content)?;
        
        // Map CIK to symbol
        let cik = edgar_data.cik.to_string();
        let symbol = match self.cik_symbol_map.get(&cik) {
            Some(symbol) => symbol.clone(),
            None => {
                self.progress.cik_mapping_stats.unmapped_ciks += 1;
                return Err(anyhow!("CIK {} not mapped to any symbol", cik));
            }
        };
        
        // Get stock_id
        let stock_id = match self.symbol_stock_id_map.get(&symbol) {
            Some(&id) => id,
            None => return Err(anyhow!("Symbol {} not found in stocks table", symbol)),
        };
        
        self.progress.cik_mapping_stats.mapped_ciks += 1;
        
        // Extract financial data using GAAP field mapping
        debug!("Extracting financial data for {} (CIK: {}, stock_id: {})", 
               symbol, cik, stock_id);
        
        // Extract income statements and balance sheets
        let extracted_data = self.extract_financial_statements(&edgar_data, stock_id).await?;
        
        debug!("Extracted {} income statements and {} balance sheets for {}", 
               extracted_data.income_statements.len(), 
               extracted_data.balance_sheets.len(), 
               symbol);
        
        // Insert data into database
        self.insert_financial_data_to_db(&extracted_data).await?;
        
        // Update data quality stats
        if extracted_data.income_statements.len() > 0 {
            self.progress.data_quality_stats.complete_income_statements += 1;
        }
        if extracted_data.balance_sheets.len() > 0 {
            self.progress.data_quality_stats.complete_balance_sheets += 1;
        }
        
        Ok(())
    }
    
    fn display_progress(&self, _current_index: usize, _total_files: usize) {
        let processed = self.progress.processed_files;
        let successful = self.progress.successful_extractions;
        let failed = self.progress.failed_extractions;
        let percentage = (processed as f64 / self.progress.total_files as f64) * 100.0;
        
        print!("\r🔄 Progress: {}/{} ({:.1}%) | Success: {} | Failed: {} | Current: {}",
               processed, self.progress.total_files, percentage, successful, failed,
               self.progress.current_file.as_deref().unwrap_or("unknown"));
        
        use std::io::{self, Write};
        io::stdout().flush().unwrap();
    }
    
    fn display_final_summary(&self) {
        println!("\n\n📊 Extraction Complete - Final Summary");
        println!("======================================");
        println!("✅ Successful: {} files", self.progress.successful_extractions);
        println!("❌ Failed: {} files", self.progress.failed_extractions);
        println!("📈 CIK Mapping: {} mapped, {} unmapped", 
                 self.progress.cik_mapping_stats.mapped_ciks,
                 self.progress.cik_mapping_stats.unmapped_ciks);
        
        let success_rate = if self.progress.processed_files > 0 {
            (self.progress.successful_extractions as f64 / self.progress.processed_files as f64) * 100.0
        } else {
            0.0
        };
        
        println!("💾 Success rate: {:.1}%", success_rate);
        println!("\n🎉 EDGAR extraction complete!");
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    let settings = ExtractionSettings {
        edgar_path: cli.edgar_path.clone(),
        batch_size: cli.batch_size,
        test_mode: cli.test_cik.is_some(),
        cik_range: cli.cik_range.clone(),
    };
    
    let edgar_path = PathBuf::from(cli.edgar_path);
    let progress_file = PathBuf::from(cli.progress_file);
    
    // Handle test mode
    if let Some(test_cik) = cli.test_cik {
        return test_single_cik(&edgar_path, &test_cik).await;
    }
    
    // Handle validation only mode
    if cli.validate_only {
        return validate_existing_data().await;
    }
    
    // Create extractor
    let mut extractor = EdgarExtractor::new(edgar_path, progress_file, settings).await?;
    
    // Execute command
    match cli.command {
        Some(Commands::Extract) | None => {
            extractor.extract_all_files().await?;
        }
        Some(Commands::Resume) => {
            info!("Resuming previous extraction...");
            extractor.extract_all_files().await?;
        }
        Some(Commands::Validate) => {
            return validate_existing_data().await;
        }
        Some(Commands::Status) => {
            show_progress_status(&extractor.progress_file).await?;
            return Ok(());
        }
        Some(Commands::Scan) => {
            extractor.scan_edgar_files().await?;
            return Ok(());
        }
    }
    
    Ok(())
}

async fn test_single_cik(edgar_path: &Path, test_cik: &str) -> Result<()> {
    println!("🧪 Testing single CIK: {}", test_cik);
    
    // Find the file for this CIK
    let filename = format!("CIK{:0>10}.json", test_cik);
    let file_path = edgar_path.join(&filename);
    
    if !file_path.exists() {
        return Err(anyhow!("File not found: {}", file_path.display()));
    }
    
    println!("📂 Reading file: {}", file_path.display());
    
    // Read and parse JSON
    let content = fs::read_to_string(&file_path)?;
    let edgar_data: EdgarCompanyFacts = serde_json::from_str(&content)?;
    
    println!("✅ Successfully parsed EDGAR data:");
    println!("   CIK: {}", edgar_data.cik);
    println!("   Entity: {}", edgar_data.entity_name);
    println!("   GAAP Fields: {}", edgar_data.facts.us_gaap.len());
    
    // Show sample fields and test extraction
    println!("\n🔍 Sample GAAP fields:");
    let mut field_count = 0;
    for (field_name, concept) in &edgar_data.facts.us_gaap {
        if field_count < 5 {
            if let Some(usd_values) = concept.units.get("USD") {
                println!("   Field: {} ({} values)", field_name, usd_values.len());
                // Show a sample value
                if let Some(sample_value) = usd_values.first() {
                    println!("     Sample: end={}, val={}, fy={:?}, fp={:?}", 
                             sample_value.end, sample_value.val, sample_value.fy, sample_value.fp);
                }
                field_count += 1;
            }
        }
    }
    
    // Test GAAP field extraction for key fields
    println!("\n🎯 Testing key field extraction:");
    if let Some(revenue_concept) = edgar_data.facts.us_gaap.get("RevenueFromContractWithCustomerExcludingAssessedTax") {
        if let Some(usd_values) = revenue_concept.units.get("USD") {
            println!("   Revenue field found with {} values", usd_values.len());
            // Look for Q3 2024 data
            for value in usd_values {
                if value.fy == Some(2024) && value.fp.as_ref() == Some(&"Q3".to_string()) {
                    println!("     ✅ Q3 2024 Revenue: ${}", value.val as u64);
                }
            }
        }
    }
    
    if let Some(net_income_concept) = edgar_data.facts.us_gaap.get("NetIncomeLoss") {
        if let Some(usd_values) = net_income_concept.units.get("USD") {
            println!("   Net Income field found with {} values", usd_values.len());
            // Look for Q3 2024 data
            for value in usd_values {
                if value.fy == Some(2024) && value.fp.as_ref() == Some(&"Q3".to_string()) {
                    println!("     ✅ Q3 2024 Net Income: ${}", value.val as u64);
                }
            }
        }
    }
    
    Ok(())
}

async fn validate_existing_data() -> Result<()> {
    println!("🔍 Validating existing financial data...");
    
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:./db/stocks.db".to_string());
    let db_pool = SqlitePool::connect(&database_url).await?;
    
    let income_stats = sqlx::query_as::<_, (i64, i64)>(
        "SELECT COUNT(DISTINCT stock_id), COUNT(*) FROM income_statements"
    )
    .fetch_one(&db_pool)
    .await?;
    
    let balance_stats = sqlx::query_as::<_, (i64, i64)>(
        "SELECT COUNT(DISTINCT stock_id), COUNT(*) FROM balance_sheets"
    )
    .fetch_one(&db_pool)
    .await?;
    
    println!("📊 Current financial data:");
    println!("   Income statements: {} records for {} companies", income_stats.1, income_stats.0);
    println!("   Balance sheets: {} records for {} companies", balance_stats.1, balance_stats.0);
    
    Ok(())
}

async fn show_progress_status(progress_file: &Path) -> Result<()> {
    if !progress_file.exists() {
        println!("No progress file found at {}", progress_file.display());
        return Ok(());
    }
    
    let progress = EdgarExtractor::load_progress(progress_file).await?;
    
    println!("📊 EDGAR Extraction Progress");
    println!("============================");
    println!("Session ID: {}", progress.session_id);
    println!("Started: {}", progress.start_time.format("%Y-%m-%d %H:%M:%S UTC"));
    println!("Total files: {}", progress.total_files);
    println!("Processed: {} ({:.1}%)", 
             progress.processed_files,
             (progress.processed_files as f64 / progress.total_files as f64) * 100.0);
    println!("Successful: {}", progress.successful_extractions);
    println!("Failed: {}", progress.failed_extractions);
    
    if let Some(current) = progress.current_file {
        println!("Current file: {}", current);
    }
    
    println!("\nCIK Mapping:");
    println!("  Mapped: {}", progress.cik_mapping_stats.mapped_ciks);
    println!("  Unmapped: {}", progress.cik_mapping_stats.unmapped_ciks);
    
    Ok(())
}