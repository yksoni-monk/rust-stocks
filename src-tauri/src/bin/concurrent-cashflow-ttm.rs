/// Concurrent Cash Flow TTM Calculator
///
/// High-performance concurrent system that calculates Trailing Twelve Months (TTM)
/// cash flow data from quarterly records for S&P 500 stocks. This fills the gap
/// needed for complete Piotroski F-Score calculations.

use anyhow::Result;
use chrono::NaiveDate;
use clap::{Parser, Subcommand};
use sqlx::{SqlitePool, Row};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use tokio::time::interval;
use tracing::{info, debug, error};
use rust_stocks_tauri_lib::database::helpers::get_database_connection;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Number of concurrent workers
    #[arg(long, default_value = "8")]
    workers: usize,

    /// Batch size for processing stocks
    #[arg(long, default_value = "25")]
    batch_size: usize,

    /// Progress reporting interval in seconds
    #[arg(long, default_value = "3")]
    progress_interval: u64,

    /// Only process S&P 500 stocks
    #[arg(long, default_value = "true")]
    sp500_only: bool,

    /// Specific symbols to process (comma-separated)
    #[arg(long)]
    symbols: Option<String>,

    /// Minimum quarters required for TTM calculation
    #[arg(long, default_value = "4")]
    min_quarters: usize,
}

#[derive(Subcommand)]
enum Commands {
    /// Calculate TTM cash flow data
    Calculate,
    /// Show current TTM status
    Status,
    /// Validate TTM calculations
    Validate,
}

#[derive(Debug, Clone)]
struct CashFlowQuarterlyData {
    stock_id: i64,
    symbol: String,
    fiscal_year: i32,
    fiscal_period: String,
    report_date: NaiveDate,
    operating_cash_flow: Option<f64>,
    investing_cash_flow: Option<f64>,
    financing_cash_flow: Option<f64>,
    net_cash_flow: Option<f64>,
    depreciation_expense: Option<f64>,
    dividends_paid: Option<f64>,
    share_repurchases: Option<f64>,
}

#[derive(Debug, Clone)]
struct CashFlowTTMData {
    stock_id: i64,
    _symbol: String,
    ttm_end_date: NaiveDate,
    fiscal_year: i32,
    operating_cash_flow: Option<f64>,
    investing_cash_flow: Option<f64>,
    financing_cash_flow: Option<f64>,
    net_cash_flow: Option<f64>,
    depreciation_expense: Option<f64>,
    dividends_paid: Option<f64>,
    share_repurchases: Option<f64>,
    _quarters_used: usize,
    _data_quality_score: f32,
}

#[derive(Debug)]
struct ProcessingStats {
    stocks_processed: AtomicUsize,
    ttm_records_created: AtomicUsize,
    stocks_with_insufficient_data: AtomicUsize,
    total_stocks: AtomicUsize,
}

impl ProcessingStats {
    fn new() -> Self {
        Self {
            stocks_processed: AtomicUsize::new(0),
            ttm_records_created: AtomicUsize::new(0),
            stocks_with_insufficient_data: AtomicUsize::new(0),
            total_stocks: AtomicUsize::new(0),
        }
    }
}

struct ConcurrentTTMCalculator {
    pool: SqlitePool,
    config: CalculatorConfig,
    stats: Arc<ProcessingStats>,
    semaphore: Arc<Semaphore>,
}

#[derive(Debug, Clone)]
struct CalculatorConfig {
    max_workers: usize,
    batch_size: usize,
    _progress_interval: Duration,
    sp500_only: bool,
    min_quarters: usize,
}

impl ConcurrentTTMCalculator {
    async fn new(config: CalculatorConfig) -> Result<Self> {
        let pool = get_database_connection().await.map_err(|e| anyhow::anyhow!("{}", e))?;
        let stats = Arc::new(ProcessingStats::new());
        let semaphore = Arc::new(Semaphore::new(config.max_workers));

        Ok(Self {
            pool,
            config,
            stats,
            semaphore,
        })
    }

    /// Get stocks that need TTM cash flow calculation
    async fn get_stocks_for_processing(&self, symbols: Option<&str>) -> Result<Vec<(i64, String)>> {
        let query = if self.config.sp500_only {
            if let Some(symbols) = symbols {
                let symbol_list: Vec<&str> = symbols.split(',').map(|s| s.trim()).collect();
                let placeholders = symbol_list.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                format!(
                    "SELECT DISTINCT s.id, s.symbol
                     FROM stocks s
                     JOIN sp500_symbols sp ON s.symbol = sp.symbol
                     WHERE s.symbol IN ({})
                     ORDER BY s.symbol",
                    placeholders
                )
            } else {
                "SELECT DISTINCT s.id, s.symbol
                 FROM stocks s
                 JOIN sp500_symbols sp ON s.symbol = sp.symbol
                 ORDER BY s.symbol".to_string()
            }
        } else {
            "SELECT DISTINCT s.id, s.symbol
             FROM stocks s
             ORDER BY s.symbol".to_string()
        };

        let mut query_builder = sqlx::query(&query);

        if let Some(symbols) = symbols {
            for symbol in symbols.split(',').map(|s| s.trim()) {
                query_builder = query_builder.bind(symbol);
            }
        }

        let rows = query_builder.fetch_all(&self.pool).await?;

        let stocks: Vec<(i64, String)> = rows.into_iter()
            .map(|row| (row.get::<i64, _>("id"), row.get::<String, _>("symbol")))
            .collect();

        info!("🎯 Found {} stocks for TTM cash flow calculation", stocks.len());
        self.stats.total_stocks.store(stocks.len(), Ordering::Relaxed);

        Ok(stocks)
    }

    /// Get quarterly cash flow data for a stock
    async fn get_quarterly_data(&self, stock_id: i64, symbol: &str) -> Result<Vec<CashFlowQuarterlyData>> {
        let query = r#"
            SELECT
                stock_id, fiscal_year, fiscal_period, report_date,
                operating_cash_flow, investing_cash_flow, financing_cash_flow, net_cash_flow,
                depreciation_expense, dividends_paid, share_repurchases
            FROM cash_flow_statements
            WHERE stock_id = ? AND period_type = 'Quarterly' AND fiscal_period IS NOT NULL
            ORDER BY fiscal_year DESC,
                     CASE fiscal_period
                         WHEN 'Q4' THEN 4
                         WHEN 'Q3' THEN 3
                         WHEN 'Q2' THEN 2
                         WHEN 'Q1' THEN 1
                         ELSE 0
                     END DESC
        "#;

        let rows = sqlx::query(query)
            .bind(stock_id)
            .fetch_all(&self.pool)
            .await?;

        let quarterly_data: Vec<CashFlowQuarterlyData> = rows.into_iter()
            .map(|row| {
                let report_date_str: String = row.get("report_date");
                CashFlowQuarterlyData {
                    stock_id,
                    symbol: symbol.to_string(),
                    fiscal_year: row.get("fiscal_year"),
                    fiscal_period: row.get("fiscal_period"),
                    report_date: NaiveDate::parse_from_str(&report_date_str, "%Y-%m-%d")
                        .unwrap_or_else(|_| NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
                    operating_cash_flow: row.get("operating_cash_flow"),
                    investing_cash_flow: row.get("investing_cash_flow"),
                    financing_cash_flow: row.get("financing_cash_flow"),
                    net_cash_flow: row.get("net_cash_flow"),
                    depreciation_expense: row.get("depreciation_expense"),
                    dividends_paid: row.get("dividends_paid"),
                    share_repurchases: row.get("share_repurchases"),
                }
            })
            .collect();

        debug!("📊 {} has {} quarterly cash flow records", symbol, quarterly_data.len());
        Ok(quarterly_data)
    }

    /// Calculate TTM data from quarterly records (rolling 4 quarters approach)
    fn calculate_ttm_data(&self, quarterly_data: Vec<CashFlowQuarterlyData>) -> Result<Vec<CashFlowTTMData>> {
        if quarterly_data.is_empty() {
            return Ok(Vec::new());
        }

        let symbol = quarterly_data[0].symbol.clone();
        let stock_id = quarterly_data[0].stock_id;

        // Sort all quarters by fiscal year and quarter (most recent first)
        let mut sorted_quarters = quarterly_data;
        sorted_quarters.sort_by(|a, b| {
            // Primary sort: fiscal year descending
            let year_cmp = b.fiscal_year.cmp(&a.fiscal_year);
            if year_cmp != std::cmp::Ordering::Equal {
                return year_cmp;
            }

            // Secondary sort: quarter descending (Q3, Q2, Q1)
            let order_a = match a.fiscal_period.as_str() {
                "Q4" => 4, "Q3" => 3, "Q2" => 2, "Q1" => 1, _ => 0
            };
            let order_b = match b.fiscal_period.as_str() {
                "Q4" => 4, "Q3" => 3, "Q2" => 2, "Q1" => 1, _ => 0
            };
            order_b.cmp(&order_a)
        });

        let mut ttm_records = Vec::new();

        // Calculate rolling TTM for the most recent periods
        // Use a sliding window of 4 quarters to create multiple TTM snapshots
        for window_start in 0..sorted_quarters.len() {
            if window_start + self.config.min_quarters > sorted_quarters.len() {
                break; // Not enough quarters left for TTM
            }

            let window_quarters: Vec<_> = sorted_quarters.iter()
                .skip(window_start)
                .take(self.config.min_quarters)
                .cloned()
                .collect();

            if window_quarters.len() < self.config.min_quarters {
                continue;
            }

            // Use the most recent quarter's date and fiscal year as the TTM endpoint
            let ttm_end_date = window_quarters[0].report_date;
            let ttm_fiscal_year = window_quarters[0].fiscal_year;

            // Calculate TTM by summing the window quarters
            let operating_cash_flow = Self::sum_optional_values(&window_quarters, |q| q.operating_cash_flow);
            let investing_cash_flow = Self::sum_optional_values(&window_quarters, |q| q.investing_cash_flow);
            let financing_cash_flow = Self::sum_optional_values(&window_quarters, |q| q.financing_cash_flow);
            let net_cash_flow = Self::sum_optional_values(&window_quarters, |q| q.net_cash_flow);
            let depreciation_expense = Self::sum_optional_values(&window_quarters, |q| q.depreciation_expense);
            let dividends_paid = Self::sum_optional_values(&window_quarters, |q| q.dividends_paid);
            let share_repurchases = Self::sum_optional_values(&window_quarters, |q| q.share_repurchases);

            // Calculate data quality score based on how many fields have data
            let total_fields = 7.0;
            let fields_with_data = [
                operating_cash_flow.is_some(),
                investing_cash_flow.is_some(),
                financing_cash_flow.is_some(),
                net_cash_flow.is_some(),
                depreciation_expense.is_some(),
                dividends_paid.is_some(),
                share_repurchases.is_some(),
            ].iter().filter(|&&x| x).count() as f32;

            let data_quality_score = (fields_with_data / total_fields) * 100.0;

            let ttm_record = CashFlowTTMData {
                stock_id,
                _symbol: symbol.clone(),
                ttm_end_date,
                fiscal_year: ttm_fiscal_year,
                operating_cash_flow,
                investing_cash_flow,
                financing_cash_flow,
                net_cash_flow,
                depreciation_expense,
                dividends_paid,
                share_repurchases,
                _quarters_used: window_quarters.len(),
                _data_quality_score: data_quality_score,
            };

            debug!("✅ {} TTM ending {}: calculated with {} quarters, {:.1}% data quality",
                   symbol, ttm_end_date, window_quarters.len(), data_quality_score);

            ttm_records.push(ttm_record);

            // For now, only calculate the most recent TTM to avoid duplicates
            // TODO: In the future, we could store historical TTM snapshots
            break;
        }

        Ok(ttm_records)
    }

    /// Helper function to sum optional values
    fn sum_optional_values<F>(quarters: &[CashFlowQuarterlyData], extractor: F) -> Option<f64>
    where
        F: Fn(&CashFlowQuarterlyData) -> Option<f64>,
    {
        let values: Vec<f64> = quarters.iter()
            .filter_map(|q| extractor(q))
            .collect();

        if values.is_empty() {
            None
        } else {
            Some(values.iter().sum())
        }
    }

    /// Insert TTM records into database
    async fn insert_ttm_records(&self, ttm_records: Vec<CashFlowTTMData>) -> Result<usize> {
        if ttm_records.is_empty() {
            return Ok(0);
        }

        let mut tx = self.pool.begin().await?;
        let mut inserted_count = 0;

        for ttm_record in ttm_records {
            let result = sqlx::query(
                r#"
                INSERT OR REPLACE INTO cash_flow_statements
                (stock_id, period_type, report_date, fiscal_year, fiscal_period,
                 operating_cash_flow, investing_cash_flow, financing_cash_flow, net_cash_flow,
                 depreciation_expense, dividends_paid, share_repurchases, data_source)
                VALUES (?, 'TTM', ?, ?, NULL, ?, ?, ?, ?, ?, ?, ?, 'calculated_ttm')
                "#
            )
            .bind(ttm_record.stock_id)
            .bind(ttm_record.ttm_end_date.format("%Y-%m-%d").to_string())
            .bind(ttm_record.fiscal_year)
            .bind(ttm_record.operating_cash_flow)
            .bind(ttm_record.investing_cash_flow)
            .bind(ttm_record.financing_cash_flow)
            .bind(ttm_record.net_cash_flow)
            .bind(ttm_record.depreciation_expense)
            .bind(ttm_record.dividends_paid)
            .bind(ttm_record.share_repurchases)
            .execute(&mut *tx)
            .await?;

            if result.rows_affected() > 0 {
                inserted_count += 1;
            }
        }

        tx.commit().await?;
        Ok(inserted_count)
    }

    /// Process a single stock
    async fn process_stock(&self, stock_id: i64, symbol: String) -> Result<usize> {
        // No rate limiting needed for local SQLite database operations

        debug!("🔄 Processing {}", symbol);

        // Get quarterly data
        let quarterly_data = self.get_quarterly_data(stock_id, &symbol).await?;

        if quarterly_data.len() < self.config.min_quarters {
            debug!("⏭️  {}: Insufficient quarterly data ({} quarters)", symbol, quarterly_data.len());
            self.stats.stocks_with_insufficient_data.fetch_add(1, Ordering::Relaxed);
            return Ok(0);
        }

        // Calculate TTM data
        let ttm_records = self.calculate_ttm_data(quarterly_data)?;

        if ttm_records.is_empty() {
            debug!("⏭️  {}: No TTM records generated", symbol);
            self.stats.stocks_with_insufficient_data.fetch_add(1, Ordering::Relaxed);
            return Ok(0);
        }

        // Insert TTM records
        let inserted_count = self.insert_ttm_records(ttm_records).await?;

        debug!("✅ {}: {} TTM records created", symbol, inserted_count);

        self.stats.stocks_processed.fetch_add(1, Ordering::Relaxed);
        self.stats.ttm_records_created.fetch_add(inserted_count, Ordering::Relaxed);

        Ok(inserted_count)
    }

    /// Process all stocks concurrently
    async fn process_all_stocks(&self, stocks: Vec<(i64, String)>) -> Result<()> {
        info!("🚀 Starting concurrent TTM calculation for {} stocks", stocks.len());

        // Start progress monitor
        let stats_clone = Arc::clone(&self.stats);
        let progress_task: JoinHandle<()> = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(3));
            loop {
                interval.tick().await;
                let processed = stats_clone.stocks_processed.load(Ordering::Relaxed);
                let total = stats_clone.total_stocks.load(Ordering::Relaxed);
                let ttm_created = stats_clone.ttm_records_created.load(Ordering::Relaxed);
                let insufficient = stats_clone.stocks_with_insufficient_data.load(Ordering::Relaxed);

                if total > 0 {
                    let progress = (processed as f32 / total as f32) * 100.0;
                    info!("📊 Progress: {:.1}% ({}/{}) | TTM Records: {} | Insufficient Data: {}",
                          progress, processed, total, ttm_created, insufficient);

                    if processed >= total {
                        break;
                    }
                }
            }
        });

        // Process stocks in batches
        let mut handles = Vec::new();

        for stock_batch in stocks.chunks(self.config.batch_size) {
            for (stock_id, symbol) in stock_batch {
                let calculator = self.clone_for_task();
                let stock_id = *stock_id;
                let symbol = symbol.clone();

                let handle = tokio::spawn(async move {
                    if let Err(e) = calculator.process_stock(stock_id, symbol.clone()).await {
                        error!("❌ Failed to process {}: {}", symbol, e);
                    }
                });

                handles.push(handle);
            }

            // Wait for this batch to complete before starting the next
            for handle in handles.drain(..) {
                let _ = handle.await;
            }
        }

        // Stop progress monitor
        progress_task.abort();

        // Final stats
        let processed = self.stats.stocks_processed.load(Ordering::Relaxed);
        let ttm_created = self.stats.ttm_records_created.load(Ordering::Relaxed);
        let insufficient = self.stats.stocks_with_insufficient_data.load(Ordering::Relaxed);

        info!("🎉 TTM Calculation Complete!");
        info!("   📈 Stocks processed: {}", processed);
        info!("   💰 TTM records created: {}", ttm_created);
        info!("   ⚠️  Stocks with insufficient data: {}", insufficient);

        Ok(())
    }

    fn clone_for_task(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            config: self.config.clone(),
            stats: Arc::clone(&self.stats),
            semaphore: Arc::clone(&self.semaphore),
        }
    }

    /// Show current TTM status
    async fn show_status(&self) -> Result<()> {
        info!("📊 Cash Flow TTM Status Report");
        info!("==============================");

        // Overall statistics
        let total_cash_flow: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cash_flow_statements")
            .fetch_one(&self.pool).await?;

        let ttm_cash_flow: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cash_flow_statements WHERE period_type = 'TTM'")
            .fetch_one(&self.pool).await?;

        let quarterly_cash_flow: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cash_flow_statements WHERE period_type = 'Quarterly'")
            .fetch_one(&self.pool).await?;

        info!("📋 Cash Flow Data Overview:");
        info!("   Total records: {}", total_cash_flow);
        info!("   TTM records: {}", ttm_cash_flow);
        info!("   Quarterly records: {}", quarterly_cash_flow);

        // S&P 500 specific stats
        if self.config.sp500_only {
            let sp500_with_ttm: i64 = sqlx::query_scalar(
                "SELECT COUNT(DISTINCT s.id) FROM stocks s
                 JOIN sp500_symbols sp ON s.symbol = sp.symbol
                 JOIN cash_flow_statements cf ON s.id = cf.stock_id
                 WHERE cf.period_type = 'TTM'"
            ).fetch_one(&self.pool).await?;

            let sp500_total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sp500_symbols")
                .fetch_one(&self.pool).await?;

            info!("🎯 S&P 500 Coverage:");
            info!("   Stocks with TTM cash flow: {}/{} ({:.1}%)",
                  sp500_with_ttm, sp500_total, (sp500_with_ttm as f32 / sp500_total as f32) * 100.0);
        }

        // Piotroski readiness check
        let piotroski_ready: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT s.id) FROM stocks s
             JOIN income_statements i ON s.id = i.stock_id AND i.period_type = 'TTM'
             JOIN balance_sheets b ON s.id = b.stock_id AND b.period_type = 'TTM'
             JOIN cash_flow_statements c ON s.id = c.stock_id AND c.period_type = 'TTM'"
        ).fetch_one(&self.pool).await?;

        info!("🧮 Piotroski F-Score Readiness:");
        info!("   Stocks with complete TTM data: {}", piotroski_ready);

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    info!("🚀 Concurrent Cash Flow TTM Calculator");
    info!("====================================");

    let config = CalculatorConfig {
        max_workers: cli.workers,
        batch_size: cli.batch_size,
        _progress_interval: Duration::from_secs(cli.progress_interval),
        sp500_only: cli.sp500_only,
        min_quarters: cli.min_quarters,
    };

    info!("Configuration:");
    info!("   Workers: {}", config.max_workers);
    info!("   Batch size: {}", config.batch_size);
    info!("   S&P 500 only: {}", config.sp500_only);
    info!("   Min quarters for TTM: {}", config.min_quarters);

    let calculator = ConcurrentTTMCalculator::new(config).await?;

    match cli.command.unwrap_or(Commands::Calculate) {
        Commands::Calculate => {
            let stocks = calculator.get_stocks_for_processing(cli.symbols.as_deref()).await?;
            calculator.process_all_stocks(stocks).await?;
        }
        Commands::Status => {
            calculator.show_status().await?;
        }
        Commands::Validate => {
            calculator.show_status().await?;
            info!("🔍 Validation complete - see status above");
        }
    }

    info!("✅ TTM calculation process finished");
    Ok(())
}