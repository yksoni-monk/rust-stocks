use std::collections::HashMap;
use std::path::Path;
use csv::ReaderBuilder;
use sqlx::{SqlitePool, Row};
use chrono::NaiveDate;
use serde::Deserialize;
use indicatif::{ProgressBar, ProgressStyle};
use anyhow::{Result, anyhow};

#[derive(Debug, Deserialize)]
struct SimFinDailyPrice {
    #[serde(rename = "Ticker")]
    ticker: String,
    #[serde(rename = "SimFinId")]
    simfin_id: i64,
    #[serde(rename = "Date")]
    date: String,
    #[serde(rename = "Open")]
    open: Option<String>,
    #[serde(rename = "High")]
    high: Option<String>,
    #[serde(rename = "Low")]
    low: Option<String>,
    #[serde(rename = "Close")]
    close: Option<String>,
    #[serde(rename = "Adj. Close")]
    adj_close: Option<String>,
    #[serde(rename = "Volume")]
    volume: Option<String>,
    #[serde(rename = "Dividend")]
    dividend: Option<String>,
    #[serde(rename = "Shares Outstanding")]
    shares_outstanding: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SimFinQuarterlyIncome {
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
    #[serde(rename = "Selling, General & Admin")]
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

#[derive(Debug)]
pub struct ImportStats {
    pub stocks_imported: usize,
    pub prices_imported: usize,
    pub financials_imported: usize,
    pub eps_calculated: usize,
    pub pe_ratios_calculated: usize,
    pub errors: usize,
}

impl Default for ImportStats {
    fn default() -> Self {
        Self {
            stocks_imported: 0,
            prices_imported: 0,
            financials_imported: 0,
            eps_calculated: 0,
            pe_ratios_calculated: 0,
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
fn parse_optional_i64(value: &Option<String>) -> Option<i64> {
    value.as_ref().and_then(|s| {
        if s.trim().is_empty() {
            None
        } else {
            s.trim().parse().ok()
        }
    })
}

/// Import stocks from daily prices CSV (extract unique tickers)
pub async fn import_stocks_from_daily_prices(
    pool: &SqlitePool,
    csv_path: &str,
) -> Result<usize> {
    println!("📊 Extracting unique stocks from daily prices CSV...");
    
    let path = Path::new(csv_path);
    if !path.exists() {
        return Err(anyhow!("CSV file not found: {}", csv_path));
    }

    let mut rdr = ReaderBuilder::new()
        .delimiter(b';')
        .from_path(csv_path)?;

    let mut unique_stocks: HashMap<String, (i64, String)> = HashMap::new();
    let mut total_rows = 0;

    // First pass: collect unique stocks
    for result in rdr.deserialize() {
        total_rows += 1;
        if total_rows % 100000 == 0 {
            println!("  Processed {} rows...", total_rows);
        }

        let record: SimFinDailyPrice = result.map_err(|e| {
            anyhow!("Failed to parse CSV row {}: {}", total_rows, e)
        })?;

        unique_stocks.insert(
            record.ticker.clone(),
            (record.simfin_id, record.ticker.clone()),
        );
    }

    println!("  Found {} unique stocks from {} total rows", unique_stocks.len(), total_rows);

    // Insert stocks into database
    let pb = ProgressBar::new(unique_stocks.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("#>-")
    );
    pb.set_message("Importing stocks...");

    let mut inserted_count = 0;
    for (ticker, (simfin_id, _)) in unique_stocks.iter() {
        let result = sqlx::query(
            "INSERT OR IGNORE INTO stocks (symbol, company_name, simfin_id, created_at) 
             VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP)"
        )
        .bind(ticker)
        .bind(format!("{} Inc.", ticker)) // Placeholder company name
        .bind(simfin_id)
        .execute(pool)
        .await;

        match result {
            Ok(query_result) => {
                if query_result.rows_affected() > 0 {
                    inserted_count += 1;
                }
            }
            Err(e) => {
                eprintln!("Failed to insert stock {}: {}", ticker, e);
            }
        }

        pb.inc(1);
    }

    pb.finish_with_message("✅ Stocks imported successfully");
    Ok(inserted_count)
}

/// Import daily prices with batch processing
pub async fn import_daily_prices(
    pool: &SqlitePool, 
    csv_path: &str, 
    batch_size: usize
) -> Result<usize> {
    println!("📈 Importing daily prices from CSV...");

    // First get stock_id mapping
    let stock_mapping = get_stock_id_mapping(pool).await?;
    
    let path = Path::new(csv_path);
    if !path.exists() {
        return Err(anyhow!("CSV file not found: {}", csv_path));
    }

    let mut rdr = ReaderBuilder::new()
        .delimiter(b';')
        .from_path(csv_path)?;

    // Count total rows for progress tracking
    println!("  Counting total rows...");
    let total_rows = rdr.records().count();
    println!("  Total rows to process: {}", total_rows);

    // Reset reader
    let mut rdr = ReaderBuilder::new()
        .delimiter(b';')
        .from_path(csv_path)?;

    let pb = ProgressBar::new(total_rows as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg} [{eta}]")
            .unwrap()
            .progress_chars("#>-")
    );

    let mut batch = Vec::new();
    let mut imported_count = 0;
    let mut error_count = 0;

    for (row_num, result) in rdr.deserialize().enumerate() {
        let record: SimFinDailyPrice = match result {
            Ok(record) => record,
            Err(e) => {
                eprintln!("Failed to parse row {}: {}", row_num + 1, e);
                error_count += 1;
                pb.inc(1);
                continue;
            }
        };

        if let Some(&stock_id) = stock_mapping.get(&record.ticker) {
            batch.push((stock_id, record));

            if batch.len() >= batch_size {
                let batch_result = insert_price_batch(pool, &batch).await;
                match batch_result {
                    Ok(count) => imported_count += count,
                    Err(e) => {
                        eprintln!("Batch insert failed: {}", e);
                        error_count += batch.len();
                    }
                }
                pb.inc(batch.len() as u64);
                pb.set_message("Importing prices...");
                batch.clear();
            }
        } else {
            error_count += 1;
        }
    }

    // Process remaining batch
    if !batch.is_empty() {
        let batch_result = insert_price_batch(pool, &batch).await;
        match batch_result {
            Ok(count) => imported_count += count,
            Err(e) => {
                eprintln!("Final batch insert failed: {}", e);
                error_count += batch.len();
            }
        }
        pb.inc(batch.len() as u64);
    }

    pb.finish_with_message("✅ Daily prices imported successfully");
    println!("📊 Import summary: {} records imported, {} errors", imported_count, error_count);
    Ok(imported_count)
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

/// Insert batch of daily prices
async fn insert_price_batch(
    pool: &SqlitePool, 
    batch: &[(i64, SimFinDailyPrice)]
) -> Result<usize> {
    let mut tx = pool.begin().await?;
    let mut inserted = 0;

    for (stock_id, record) in batch {
        let date = NaiveDate::parse_from_str(&record.date, "%Y-%m-%d")
            .map_err(|e| anyhow!("Failed to parse date {}: {}", record.date, e))?;

        let open = parse_optional_f64(&record.open).unwrap_or(0.0);
        let high = parse_optional_f64(&record.high).unwrap_or(0.0);
        let low = parse_optional_f64(&record.low).unwrap_or(0.0);
        let close = parse_optional_f64(&record.close).unwrap_or(0.0);
        let volume = parse_optional_i64(&record.volume).unwrap_or(0);
        let shares_outstanding = parse_optional_i64(&record.shares_outstanding);

        let result = sqlx::query(
            "INSERT OR REPLACE INTO daily_prices (
                stock_id, date, open_price, high_price, low_price, close_price, 
                volume, shares_outstanding, data_source, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, CURRENT_TIMESTAMP)"
        )
        .bind(stock_id)
        .bind(date)
        .bind(open)
        .bind(high)
        .bind(low)
        .bind(close)
        .bind(volume)
        .bind(shares_outstanding)
        .bind("simfin")
        .execute(&mut *tx)
        .await;

        if result.is_ok() {
            inserted += 1;
        }
    }

    tx.commit().await?;
    Ok(inserted)
}

/// Import quarterly financials
pub async fn import_quarterly_financials(
    pool: &SqlitePool,
    csv_path: &str,
) -> Result<usize> {
    println!("🏢 Importing quarterly financials from CSV...");

    let stock_mapping = get_stock_id_mapping(pool).await?;
    
    let path = Path::new(csv_path);
    if !path.exists() {
        return Err(anyhow!("CSV file not found: {}", csv_path));
    }

    let mut rdr = ReaderBuilder::new()
        .delimiter(b';')
        .from_path(csv_path)?;

    // Count total rows
    println!("  Counting financial records...");
    let total_rows = rdr.records().count();
    println!("  Total financial records to process: {}", total_rows);

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
    let mut error_count = 0;

    for (row_num, result) in rdr.deserialize().enumerate() {
        let record: SimFinQuarterlyIncome = match result {
            Ok(record) => record,
            Err(e) => {
                eprintln!("Failed to parse financial row {}: {}", row_num + 1, e);
                error_count += 1;
                pb.inc(1);
                continue;
            }
        };

        if let Some(&stock_id) = stock_mapping.get(&record.ticker) {
            let insert_result = insert_quarterly_financial(pool, stock_id, &record).await;
            match insert_result {
                Ok(_) => imported_count += 1,
                Err(e) => {
                    eprintln!("Failed to insert financial record for {}: {}", record.ticker, e);
                    error_count += 1;
                }
            }
        } else {
            error_count += 1;
        }

        pb.inc(1);
        if (row_num + 1) % 1000 == 0 {
            pb.set_message("Importing financials...");
        }
    }

    pb.finish_with_message("✅ Quarterly financials imported successfully");
    println!("📊 Import summary: {} records imported, {} errors", imported_count, error_count);
    Ok(imported_count)
}

/// Insert single quarterly financial record
async fn insert_quarterly_financial(
    pool: &SqlitePool,
    stock_id: i64,
    record: &SimFinQuarterlyIncome,
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

    let restated_date = if let Some(ref date_str) = record.restated_date {
        if !date_str.trim().is_empty() {
            Some(NaiveDate::parse_from_str(date_str, "%Y-%m-%d")?)
        } else {
            None
        }
    } else {
        None
    };

    sqlx::query(
        "INSERT OR REPLACE INTO quarterly_financials (
            stock_id, simfin_id, currency, fiscal_year, fiscal_period, 
            report_date, publish_date, restated_date,
            shares_basic, shares_diluted, revenue, cost_of_revenue, gross_profit,
            operating_expenses, selling_general_admin, research_development,
            depreciation_amortization, operating_income, non_operating_income,
            interest_expense_net, pretax_income_adj, pretax_income,
            income_tax_expense, income_continuing_ops, net_extraordinary_gains,
            net_income, net_income_common, created_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, 
            ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, CURRENT_TIMESTAMP
        )"
    )
    .bind(stock_id)
    .bind(record.simfin_id)
    .bind(&record.currency)
    .bind(record.fiscal_year)
    .bind(&record.fiscal_period)
    .bind(report_date)
    .bind(publish_date)
    .bind(restated_date)
    .bind(parse_optional_i64(&record.shares_basic))
    .bind(parse_optional_i64(&record.shares_diluted))
    .bind(parse_optional_f64(&record.revenue))
    .bind(parse_optional_f64(&record.cost_of_revenue))
    .bind(parse_optional_f64(&record.gross_profit))
    .bind(parse_optional_f64(&record.operating_expenses))
    .bind(parse_optional_f64(&record.selling_general_admin))
    .bind(parse_optional_f64(&record.research_development))
    .bind(parse_optional_f64(&record.depreciation_amortization))
    .bind(parse_optional_f64(&record.operating_income))
    .bind(parse_optional_f64(&record.non_operating_income))
    .bind(parse_optional_f64(&record.interest_expense_net))
    .bind(parse_optional_f64(&record.pretax_income_adj))
    .bind(parse_optional_f64(&record.pretax_income))
    .bind(parse_optional_f64(&record.income_tax_expense))
    .bind(parse_optional_f64(&record.income_continuing_ops))
    .bind(parse_optional_f64(&record.net_extraordinary_gains))
    .bind(parse_optional_f64(&record.net_income))
    .bind(parse_optional_f64(&record.net_income_common))
    .execute(pool)
    .await?;

    Ok(())
}

/// Calculate and store EPS values (Net Income / Diluted Shares Outstanding)
pub async fn calculate_and_store_eps(pool: &SqlitePool) -> Result<usize> {
    println!("🧮 Calculating EPS values (Net Income ÷ Diluted Shares Outstanding)...");

    let financial_records = sqlx::query(
        "SELECT id, stock_id, fiscal_year, fiscal_period, net_income, shares_diluted 
         FROM quarterly_financials 
         WHERE net_income IS NOT NULL AND shares_diluted IS NOT NULL AND shares_diluted > 0"
    )
    .fetch_all(pool)
    .await?;

    let pb = ProgressBar::new(financial_records.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("#>-")
    );
    pb.set_message("Calculating EPS...");

    let mut calculated_count = 0;
    
    for record in financial_records {
        let id: i64 = record.get("id");
        let net_income: f64 = record.get("net_income");
        let shares_diluted: i64 = record.get("shares_diluted");
        
        // Calculate EPS = Net Income / Diluted Shares Outstanding
        let eps = net_income / (shares_diluted as f64);
        
        let result = sqlx::query(
            "UPDATE quarterly_financials 
             SET eps_calculated = ?1, eps_calculation_date = CURRENT_TIMESTAMP 
             WHERE id = ?2"
        )
        .bind(eps)
        .bind(id)
        .execute(pool)
        .await;

        match result {
            Ok(_) => calculated_count += 1,
            Err(e) => eprintln!("Failed to update EPS for record {}: {}", id, e),
        }

        pb.inc(1);
    }

    pb.finish_with_message("✅ EPS calculations completed");
    Ok(calculated_count)
}

/// Calculate and store P/E ratios using calculated EPS
pub async fn calculate_and_store_pe_ratios(pool: &SqlitePool) -> Result<usize> {
    println!("📊 Calculating P/E ratios (Close Price ÷ EPS)...");

    let price_records = sqlx::query(
        "SELECT id, stock_id, date, close_price 
         FROM daily_prices 
         WHERE close_price IS NOT NULL AND close_price > 0
         ORDER BY stock_id, date"
    )
    .fetch_all(pool)
    .await?;

    let pb = ProgressBar::new(price_records.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg} [{eta}]")
            .unwrap()
            .progress_chars("#>-")
    );
    pb.set_message("Calculating P/E ratios...");

    let mut calculated_count = 0;
    
    for price_record in price_records {
        let price_id: i64 = price_record.get("id");
        let stock_id: i64 = price_record.get("stock_id");
        let price_date: NaiveDate = price_record.get("date");
        let close_price: f64 = price_record.get("close_price");
        
        // Find latest calculated EPS before or on this date
        let eps_result = sqlx::query(
            "SELECT eps_calculated 
             FROM quarterly_financials 
             WHERE stock_id = ?1 AND report_date <= ?2 AND eps_calculated IS NOT NULL
             ORDER BY report_date DESC 
             LIMIT 1"
        )
        .bind(stock_id)
        .bind(price_date)
        .fetch_optional(pool)
        .await;

        if let Ok(Some(eps_row)) = eps_result {
            let eps: f64 = eps_row.get("eps_calculated");
            
            // Calculate P/E = Close Price / EPS (avoid division by zero)
            if eps != 0.0 {
                let pe_ratio = close_price / eps;
                
                let update_result = sqlx::query(
                    "UPDATE daily_prices SET pe_ratio = ?1 WHERE id = ?2"
                )
                .bind(pe_ratio)
                .bind(price_id)
                .execute(pool)
                .await;

                if update_result.is_ok() {
                    calculated_count += 1;
                }
            }
        }

        pb.inc(1);
        if calculated_count % 10000 == 0 {
            pb.set_message("Calculating P/E ratios...");
        }
    }

    pb.finish_with_message("✅ P/E ratio calculations completed");
    Ok(calculated_count)
}

/// Add performance indexes after import completion
pub async fn add_performance_indexes(pool: &SqlitePool) -> Result<()> {
    println!("⚡ Creating performance indexes...");

    let indexes = vec![
        "CREATE INDEX IF NOT EXISTS idx_stocks_simfin_id ON stocks(simfin_id)",
        "CREATE INDEX IF NOT EXISTS idx_quarterly_financials_stock_period ON quarterly_financials(stock_id, fiscal_year, fiscal_period)",
        "CREATE INDEX IF NOT EXISTS idx_quarterly_financials_report_date ON quarterly_financials(report_date)",
        "CREATE INDEX IF NOT EXISTS idx_quarterly_financials_eps ON quarterly_financials(eps_calculated)",
        "CREATE INDEX IF NOT EXISTS idx_daily_prices_simfin ON daily_prices(data_source)",
    ];

    for index_sql in indexes {
        sqlx::query(index_sql).execute(pool).await?;
        println!("  ✅ {}", index_sql.split("idx_").nth(1).unwrap_or("index"));
    }

    println!("✅ Performance indexes created");
    Ok(())
}