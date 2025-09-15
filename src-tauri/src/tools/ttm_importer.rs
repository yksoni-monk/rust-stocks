use std::collections::HashMap;
use std::path::Path;
use csv::ReaderBuilder;
use sqlx::{SqlitePool, Row};
use chrono::NaiveDate;
use serde::Deserialize;
use indicatif::{ProgressBar, ProgressStyle};
use anyhow::{Result, anyhow};

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Fields may be used in future versions or tests
struct SimFinTTMIncome {
    #[serde(rename = "Ticker")]
    ticker: String,
    #[serde(rename = "SimFinId")]
    simfin_id: i64,
    #[serde(rename = "Currency")]
    currency: String,
    #[serde(rename = "Fiscal Year")]
    fiscal_year: i32,
    #[serde(rename = "Fiscal Period")]
    fiscal_period: String,
    #[serde(rename = "Report Date")]
    report_date: String,
    #[serde(rename = "Publish Date")]
    publish_date: Option<String>,
    #[serde(rename = "Restated Date")]
    restated_date: Option<String>,
    #[serde(rename = "Shares (Basic)")]
    shares_basic: Option<String>,
    #[serde(rename = "Shares (Diluted)")]
    shares_diluted: Option<String>,
    #[serde(rename = "Revenue")]
    revenue: Option<String>,
    #[serde(rename = "Cost of Revenue")]
    cost_of_revenue: Option<String>,
    #[serde(rename = "Gross Profit")]
    gross_profit: Option<String>,
    #[serde(rename = "Operating Expenses")]
    operating_expenses: Option<String>,
    #[serde(rename = "Selling, General & Administrative")]
    selling_general_admin: Option<String>,
    #[serde(rename = "Research & Development")]
    research_development: Option<String>,
    #[serde(rename = "Depreciation & Amortization")]
    depreciation_amortization: Option<String>,
    #[serde(rename = "Operating Income (Loss)")]
    operating_income: Option<String>,
    #[serde(rename = "Non-Operating Income (Loss)")]
    non_operating_income: Option<String>,
    #[serde(rename = "Interest Expense, Net")]
    interest_expense_net: Option<String>,
    #[serde(rename = "Pretax Income (Loss), Adj.")]
    pretax_income_adj: Option<String>,
    #[serde(rename = "Abnormal Gains (Losses)")]
    abnormal_gains_losses: Option<String>,
    #[serde(rename = "Pretax Income (Loss)")]
    pretax_income: Option<String>,
    #[serde(rename = "Income Tax (Expense) Benefit, Net")]
    income_tax_expense: Option<String>,
    #[serde(rename = "Income (Loss) from Continuing Operations")]
    income_continuing_ops: Option<String>,
    #[serde(rename = "Net Extraordinary Gains (Losses)")]
    net_extraordinary_gains: Option<String>,
    #[serde(rename = "Net Income")]
    net_income: Option<String>,
    #[serde(rename = "Net Income (Common)")]
    net_income_common: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Fields may be used in future versions or tests
struct SimFinTTMBalance {
    #[serde(rename = "Ticker")]
    ticker: String,
    #[serde(rename = "SimFinId")]
    simfin_id: i64,
    #[serde(rename = "Currency")]
    currency: String,
    #[serde(rename = "Fiscal Year")]
    fiscal_year: i32,
    #[serde(rename = "Fiscal Period")]
    fiscal_period: String,
    #[serde(rename = "Report Date")]
    report_date: String,
    #[serde(rename = "Publish Date")]
    publish_date: Option<String>,
    #[serde(rename = "Restated Date")]
    restated_date: Option<String>,
    #[serde(rename = "Shares (Basic)")]
    shares_basic: Option<String>,
    #[serde(rename = "Shares (Diluted)")]
    shares_diluted: Option<String>,
    #[serde(rename = "Cash, Cash Equivalents & Short Term Investments")]
    cash_and_equivalents: Option<String>,
    #[serde(rename = "Accounts & Notes Receivable")]
    accounts_receivable: Option<String>,
    #[serde(rename = "Inventories")]
    inventories: Option<String>,
    #[serde(rename = "Total Current Assets")]
    total_current_assets: Option<String>,
    #[serde(rename = "Property, Plant & Equipment, Net")]
    ppe_net: Option<String>,
    #[serde(rename = "Long Term Investments & Receivables")]
    long_term_investments: Option<String>,
    #[serde(rename = "Other Long Term Assets")]
    other_long_term_assets: Option<String>,
    #[serde(rename = "Total Noncurrent Assets")]
    total_noncurrent_assets: Option<String>,
    #[serde(rename = "Total Assets")]
    total_assets: Option<String>,
    #[serde(rename = "Payables & Accruals")]
    payables_accruals: Option<String>,
    #[serde(rename = "Short Term Debt")]
    short_term_debt: Option<String>,
    #[serde(rename = "Total Current Liabilities")]
    total_current_liabilities: Option<String>,
    #[serde(rename = "Long Term Debt")]
    long_term_debt: Option<String>,
    #[serde(rename = "Total Noncurrent Liabilities")]
    total_noncurrent_liabilities: Option<String>,
    #[serde(rename = "Total Liabilities")]
    total_liabilities: Option<String>,
    #[serde(rename = "Share Capital & Additional Paid-In Capital")]
    share_capital: Option<String>,
    #[serde(rename = "Treasury Stock")]
    treasury_stock: Option<String>,
    #[serde(rename = "Retained Earnings")]
    retained_earnings: Option<String>,
    #[serde(rename = "Total Equity")]
    total_equity: Option<String>,
    #[serde(rename = "Total Liabilities & Equity")]
    total_liabilities_equity: Option<String>,
}

#[derive(Debug)]
pub struct TTMImportStats {
    pub income_statements_imported: usize,
    pub balance_sheets_imported: usize,
    pub stocks_processed: usize,
    pub errors: usize,
}

impl Default for TTMImportStats {
    fn default() -> Self {
        Self {
            income_statements_imported: 0,
            balance_sheets_imported: 0,
            stocks_processed: 0,
            errors: 0,
        }
    }
}

/// Parse optional string field to f64
fn parse_optional_f64(value: &Option<String>) -> Option<f64> {
    value.as_ref().and_then(|s| {
        if s.trim().is_empty() {
            None
        } else {
            s.trim().parse().ok()
        }
    })
}

/// Parse optional string field to i64
/// Note: Currently unused but may be needed for future financial data parsing
#[allow(dead_code)]
fn parse_optional_i64(value: &Option<String>) -> Option<i64> {
    value.as_ref().and_then(|s| {
        if s.trim().is_empty() {
            None
        } else {
            s.trim().parse().ok()
        }
    })
}

/// Get stock ID mapping for ticker lookup
async fn get_stock_id_mapping(pool: &SqlitePool) -> Result<HashMap<String, i64>> {
    println!("  Building stock ID mapping...");
    
    let results = sqlx::query("SELECT id, symbol FROM stocks")
        .fetch_all(pool)
        .await?;

    let mut mapping = HashMap::new();
    for row in results {
        let id: i64 = row.get("id");
        let symbol: String = row.get("symbol");
        mapping.insert(symbol, id);
    }

    println!("  Mapped {} stocks", mapping.len());
    Ok(mapping)
}

/// Import TTM income statements
pub async fn import_ttm_income_statements(
    pool: &SqlitePool,
    csv_path: &str,
) -> Result<usize> {
    println!("💰 Importing TTM income statements from CSV...");

    let stock_mapping = get_stock_id_mapping(pool).await?;
    
    let path = Path::new(csv_path);
    if !path.exists() {
        return Err(anyhow!("CSV file not found: {}", csv_path));
    }

    let mut rdr = ReaderBuilder::new()
        .delimiter(b';')
        .from_path(csv_path)?;

    // Count total rows
    println!("  Counting TTM income records...");
    let total_rows = rdr.records().count();
    println!("  Total TTM income records to process: {}", total_rows);

    // Reset reader
    let mut rdr = ReaderBuilder::new()
        .delimiter(b';')
        .from_path(csv_path)?;

    let pb = ProgressBar::new(total_rows as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("#>-")
    );

    let mut imported_count = 0;
    let mut _error_count = 0;

    for (row_num, result) in rdr.deserialize().enumerate() {
        let record: SimFinTTMIncome = match result {
            Ok(record) => record,
            Err(e) => {
                eprintln!("Failed to parse TTM income row {}: {}", row_num + 1, e);
                _error_count += 1;
                pb.inc(1);
                continue;
            }
        };

        if let Some(&stock_id) = stock_mapping.get(&record.ticker) {
            let insert_result = insert_ttm_income_statement(pool, stock_id, &record).await;
            match insert_result {
                Ok(_) => imported_count += 1,
                Err(e) => {
                    eprintln!("Failed to insert TTM income for {}: {}", record.ticker, e);
                    _error_count += 1;
                }
            }
        } else {
            _error_count += 1;
        }

        pb.inc(1);
        if (row_num + 1) % 1000 == 0 {
            pb.set_message("Importing TTM income...");
        }
    }

    pb.finish_with_message("✅ TTM income statements imported successfully");
    Ok(imported_count)
}

/// Import TTM balance sheets
pub async fn import_ttm_balance_sheets(
    pool: &SqlitePool,
    csv_path: &str,
) -> Result<usize> {
    println!("🏦 Importing TTM balance sheets from CSV...");

    let stock_mapping = get_stock_id_mapping(pool).await?;
    
    let path = Path::new(csv_path);
    if !path.exists() {
        return Err(anyhow!("CSV file not found: {}", csv_path));
    }

    let mut rdr = ReaderBuilder::new()
        .delimiter(b';')
        .from_path(csv_path)?;

    // Count total rows
    println!("  Counting TTM balance sheet records...");
    let total_rows = rdr.records().count();
    println!("  Total TTM balance sheet records to process: {}", total_rows);

    // Reset reader
    let mut rdr = ReaderBuilder::new()
        .delimiter(b';')
        .from_path(csv_path)?;

    let pb = ProgressBar::new(total_rows as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("#>-")
    );

    let mut imported_count = 0;
    let mut _error_count = 0;

    for (row_num, result) in rdr.deserialize().enumerate() {
        let record: SimFinTTMBalance = match result {
            Ok(record) => record,
            Err(e) => {
                eprintln!("Failed to parse TTM balance row {}: {}", row_num + 1, e);
                _error_count += 1;
                pb.inc(1);
                continue;
            }
        };

        if let Some(&stock_id) = stock_mapping.get(&record.ticker) {
            let insert_result = insert_ttm_balance_sheet(pool, stock_id, &record).await;
            match insert_result {
                Ok(_) => imported_count += 1,
                Err(e) => {
                    eprintln!("Failed to insert TTM balance sheet for {}: {}", record.ticker, e);
                    _error_count += 1;
                }
            }
        } else {
            _error_count += 1;
        }

        pb.inc(1);
        if (row_num + 1) % 1000 == 0 {
            pb.set_message("Importing TTM balance sheets...");
        }
    }

    pb.finish_with_message("✅ TTM balance sheets imported successfully");
    Ok(imported_count)
}

/// Insert single TTM income statement record
async fn insert_ttm_income_statement(
    pool: &SqlitePool,
    stock_id: i64,
    record: &SimFinTTMIncome,
) -> Result<()> {
    let report_date = NaiveDate::parse_from_str(&record.report_date, "%Y-%m-%d")
        .map_err(|e| anyhow!("Failed to parse report date {}: {}", record.report_date, e))?;

    let publish_date = if let Some(ref date_str) = record.publish_date {
        if !date_str.trim().is_empty() {
            Some(NaiveDate::parse_from_str(date_str, "%Y-%m-%d")?)
        } else {
            None
        }
    } else {
        None
    };

    sqlx::query(
        "INSERT OR REPLACE INTO income_statements (
            stock_id, period_type, report_date, fiscal_year, fiscal_period,
            revenue, gross_profit, operating_income, net_income,
            shares_basic, shares_diluted, currency, simfin_id, publish_date, 
            data_source, created_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, CURRENT_TIMESTAMP
        )"
    )
    .bind(stock_id)
    .bind("TTM") // period_type
    .bind(report_date)
    .bind(record.fiscal_year)
    .bind(&record.fiscal_period)
    .bind(parse_optional_f64(&record.revenue))
    .bind(parse_optional_f64(&record.gross_profit))
    .bind(parse_optional_f64(&record.operating_income))
    .bind(parse_optional_f64(&record.net_income))
    .bind(parse_optional_f64(&record.shares_basic))
    .bind(parse_optional_f64(&record.shares_diluted))
    .bind(&record.currency)
    .bind(record.simfin_id)
    .bind(publish_date)
    .bind("simfin")
    .execute(pool)
    .await?;

    Ok(())
}

/// Insert single TTM balance sheet record
async fn insert_ttm_balance_sheet(
    pool: &SqlitePool,
    stock_id: i64,
    record: &SimFinTTMBalance,
) -> Result<()> {
    let report_date = NaiveDate::parse_from_str(&record.report_date, "%Y-%m-%d")
        .map_err(|e| anyhow!("Failed to parse report date {}: {}", record.report_date, e))?;

    // Calculate total debt = short_term_debt + long_term_debt
    let short_term_debt = parse_optional_f64(&record.short_term_debt);
    let long_term_debt = parse_optional_f64(&record.long_term_debt);
    let total_debt = match (short_term_debt, long_term_debt) {
        (Some(st), Some(lt)) => Some(st + lt),
        (Some(st), None) => Some(st),
        (None, Some(lt)) => Some(lt),
        (None, None) => None,
    };

    sqlx::query(
        "INSERT OR REPLACE INTO balance_sheets (
            stock_id, period_type, report_date, fiscal_year, fiscal_period,
            cash_and_equivalents, short_term_debt, long_term_debt, total_debt,
            total_assets, total_liabilities, total_equity, shares_outstanding,
            currency, simfin_id, data_source, created_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, CURRENT_TIMESTAMP
        )"
    )
    .bind(stock_id)
    .bind("TTM") // period_type
    .bind(report_date)
    .bind(record.fiscal_year)
    .bind(&record.fiscal_period)
    .bind(parse_optional_f64(&record.cash_and_equivalents))
    .bind(short_term_debt)
    .bind(long_term_debt)
    .bind(total_debt)
    .bind(parse_optional_f64(&record.total_assets))
    .bind(parse_optional_f64(&record.total_liabilities))
    .bind(parse_optional_f64(&record.total_equity))
    .bind(parse_optional_f64(&record.shares_basic)) // Use basic shares for outstanding
    .bind(&record.currency)
    .bind(record.simfin_id)
    .bind("simfin")
    .execute(pool)
    .await?;

    Ok(())
}

/// Import complete TTM dataset (income + balance sheets)
pub async fn import_complete_ttm_dataset(
    pool: &SqlitePool,
    income_csv_path: &str,
    balance_csv_path: &str,
) -> Result<TTMImportStats> {
    let mut stats = TTMImportStats::default();

    println!("🚀 Starting complete TTM dataset import...");
    println!("  📊 Income statements: {}", income_csv_path);
    println!("  🏦 Balance sheets: {}", balance_csv_path);

    // Import TTM income statements
    match import_ttm_income_statements(pool, income_csv_path).await {
        Ok(count) => {
            stats.income_statements_imported = count;
            println!("✅ Imported {} TTM income statements", count);
        }
        Err(e) => {
            eprintln!("❌ Failed to import TTM income statements: {}", e);
            stats.errors += 1;
            return Err(e);
        }
    }

    // Import TTM balance sheets
    match import_ttm_balance_sheets(pool, balance_csv_path).await {
        Ok(count) => {
            stats.balance_sheets_imported = count;
            println!("✅ Imported {} TTM balance sheets", count);
        }
        Err(e) => {
            eprintln!("❌ Failed to import TTM balance sheets: {}", e);
            stats.errors += 1;
            return Err(e);
        }
    }

    // Create performance indexes for TTM data
    println!("⚡ Creating TTM performance indexes...");
    create_ttm_indexes(pool).await?;

    println!("🎉 TTM dataset import completed successfully!");
    println!("  📊 Income statements: {}", stats.income_statements_imported);
    println!("  🏦 Balance sheets: {}", stats.balance_sheets_imported);
    println!("  📈 Ready for P/S and EV/S ratio calculations");

    Ok(stats)
}

/// Create performance indexes for TTM data
async fn create_ttm_indexes(pool: &SqlitePool) -> Result<()> {
    let indexes = vec![
        "CREATE INDEX IF NOT EXISTS idx_income_statements_ttm_lookup ON income_statements(stock_id, period_type, report_date) WHERE period_type = 'TTM'",
        "CREATE INDEX IF NOT EXISTS idx_balance_sheets_ttm_lookup ON balance_sheets(stock_id, period_type, report_date) WHERE period_type = 'TTM'",
        "CREATE INDEX IF NOT EXISTS idx_income_statements_ttm_revenue ON income_statements(revenue) WHERE period_type = 'TTM' AND revenue IS NOT NULL",
        "CREATE INDEX IF NOT EXISTS idx_balance_sheets_ttm_debt_cash ON balance_sheets(total_debt, cash_and_equivalents) WHERE period_type = 'TTM'",
    ];

    for index_sql in indexes {
        sqlx::query(index_sql).execute(pool).await?;
        println!("  ✅ TTM performance index created");
    }

    Ok(())
}