use anyhow::{Result, anyhow};
use chrono::{NaiveDate, Utc};
use clap::Parser;
use std::sync::Arc;
use tracing::{info, error, Level};
use tracing_subscriber::{self, FmtSubscriber};

use rust_stocks::api::SchwabClient;
use rust_stocks::data_collector::DataCollector;
use rust_stocks::database::DatabaseManager;
use rust_stocks::models::Config;

/// Historical stock data collector with detailed batch logging
#[derive(Parser)]
#[command(name = "collect_with_detailed_logs")]
#[command(version = "1.0.0")]
#[command(author = "Rust Stocks Analysis System")]
#[command(about = "Collect historical stock data for S&P 500 companies with detailed progress logging")]
#[command(long_about = "
This tool fetches historical OHLC (Open, High, Low, Close) data for all S&P 500 companies 
from the Schwab API and stores it in the local SQLite database. It processes stocks in small
batches and provides detailed progress logging including success/failure rates, processing
times, and record counts.

The tool requires a start date in YYYYMMDD format and performs comprehensive validation to 
ensure data integrity and prevent invalid date ranges. The end date defaults to today if 
not provided.

Examples:
  cargo run --bin collect_with_detailed_logs -s 20230101 -e 20231231  # 2023 data
  cargo run --bin collect_with_detailed_logs --start-date 20240101    # 2024 to today
  cargo run --bin collect_with_detailed_logs -s 20220101 -b 10 -d 1   # Fast processing
")]
#[command(help_template = "
{name} {version}
{author}

{about}

{usage-heading} {usage}

{all-args}

{long-about}
")]
struct Args {
    /// Start date for historical data collection (YYYYMMDD format)
    #[arg(long, short = 's', help = "Start date in YYYYMMDD format (e.g., 20230101)")]
    start_date: String,

    /// End date for historical data collection (YYYYMMDD format, defaults to today)
    #[arg(long, short = 'e', help = "End date in YYYYMMDD format (e.g., 20231231). Defaults to today if not provided")]
    end_date: Option<String>,

    /// Batch size for processing stocks (default: 5)
    #[arg(long, short = 'b', default_value_t = 5, help = "Number of stocks to process in each batch (1-50)")]
    batch_size: usize,

    /// Delay between batches in seconds (default: 3)
    #[arg(long, short = 'd', default_value_t = 3, help = "Delay between batches in seconds (1-60)")]
    batch_delay: u64,
}

fn parse_date(date_str: &str, field_name: &str) -> Result<NaiveDate> {
    // Validate format
    if date_str.len() != 8 {
        return Err(anyhow!("{} must be exactly 8 digits in YYYYMMDD format, got: {}", field_name, date_str));
    }
    
    // Check if all characters are digits
    if !date_str.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("{} must contain only digits, got: {}", field_name, date_str));
    }
    
    // Parse components
    let year: i32 = date_str[0..4].parse()
        .map_err(|_| anyhow!("Invalid year in {}: {}", field_name, &date_str[0..4]))?;
    let month: u32 = date_str[4..6].parse()
        .map_err(|_| anyhow!("Invalid month in {}: {}", field_name, &date_str[4..6]))?;
    let day: u32 = date_str[6..8].parse()
        .map_err(|_| anyhow!("Invalid day in {}: {}", field_name, &date_str[6..8]))?;
    
    // Validate year range (reasonable bounds for stock data)
    if year < 1970 || year > 2050 {
        return Err(anyhow!("Year in {} must be between 1970 and 2050, got: {}", field_name, year));
    }
    
    // Validate month
    if month < 1 || month > 12 {
        return Err(anyhow!("Month in {} must be between 01 and 12, got: {:02}", field_name, month));
    }
    
    // Validate day (basic check, let NaiveDate handle detailed validation)
    if day < 1 || day > 31 {
        return Err(anyhow!("Day in {} must be between 01 and 31, got: {:02}", field_name, day));
    }
    
    // Create date and validate it's a real date
    NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| anyhow!("Invalid date in {}: {}-{:02}-{:02} (not a valid calendar date)", 
                               field_name, year, month, day))
}

fn validate_date_range(start_date: NaiveDate, end_date: NaiveDate) -> Result<()> {
    let today = Utc::now().date_naive();
    
    // Check if start date is before end date
    if start_date >= end_date {
        return Err(anyhow!(
            "Start date ({}) must be earlier than end date ({})", 
            start_date, end_date
        ));
    }
    
    // Check if end date is not in the future
    if end_date > today {
        return Err(anyhow!(
            "End date ({}) cannot be in the future. Today is {}", 
            end_date, today
        ));
    }
    
    // Check if start date is not too far in the past (before stock market data availability)
    let earliest_reasonable_date = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
    if start_date < earliest_reasonable_date {
        return Err(anyhow!(
            "Start date ({}) is too far in the past. Earliest recommended date is {}", 
            start_date, earliest_reasonable_date
        ));
    }
    
    // Warn about very large date ranges (more than 10 years)
    let days_span = (end_date - start_date).num_days();
    if days_span > 3650 { // ~10 years
        println!("⚠️  Warning: Large date range detected ({} days, ~{} years).", 
                 days_span, days_span / 365);
        println!("   This may take several hours to complete.");
        println!("   Consider using smaller date ranges for faster processing.");
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();
    
    // Parse and validate start date
    let start_date = parse_date(&args.start_date, "start-date")?;
    
    // Parse end date (use today if not provided)
    let end_date = match args.end_date {
        Some(end_date_str) => parse_date(&end_date_str, "end-date")?,
        None => Utc::now().date_naive(),
    };
    
    // Validate date range
    validate_date_range(start_date, end_date)?;
    
    // Validate batch size
    if args.batch_size < 1 || args.batch_size > 50 {
        return Err(anyhow!("Batch size must be between 1 and 50, got: {}", args.batch_size));
    }
    
    // Validate batch delay
    if args.batch_delay < 1 || args.batch_delay > 60 {
        return Err(anyhow!("Batch delay must be between 1 and 60 seconds, got: {}", args.batch_delay));
    }

    // Initialize logging with more detailed output
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_env_filter("rust_stocks=info")
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    info!("🚀 Starting Historical Data Collection with Detailed Logging");
    info!("📅 Date Range: {} to {}", start_date, end_date);
    let days_span = (end_date - start_date).num_days();
    info!("📊 Total days: {} (~{} trading days)", days_span, (days_span * 5 / 7));
    info!("================================================");

    // Load configuration
    info!("📋 Step 1: Loading configuration...");
    let config = Config::from_env()?;
    info!("✅ Configuration loaded successfully");

    // Initialize database
    info!("💾 Step 2: Initializing database...");
    let database = Arc::new(DatabaseManager::new(&config.database_path)?);
    info!("✅ Database initialized at: {}", config.database_path);

    // Initialize Schwab API client
    info!("🔑 Step 3: Initializing Schwab API client...");
    let schwab_client = Arc::new(SchwabClient::new(&config)?);
    info!("✅ Schwab API client initialized");

    // Initialize data collector
    info!("🔄 Step 4: Initializing data collector...");
    let data_collector = DataCollector::new(
        SchwabClient::new(&config)?, 
        DatabaseManager::new(&config.database_path)?, 
        config.clone()
    );
    info!("✅ Data collector initialized");

    // Get all stocks
    info!("📊 Step 5: Loading stocks from database...");
    let all_stocks = data_collector.get_active_stocks()?;
    info!("✅ Found {} stocks to process", all_stocks.len());

    // Check current database state
    info!("📈 Step 6: Checking current database state...");
    let initial_stats = match database.get_stats() {
        Ok((stock_count, price_count, last_update)) => {
            info!("✅ Database stats:");
            info!("   - Stocks: {}", stock_count);
            info!("   - Price records: {}", price_count);
            info!("   - Last update: {:?}", last_update);
            price_count as i64
        },
        Err(e) => {
            error!("❌ Failed to get database stats: {}", e);
            0
        }
    };

    info!("📈 Step 7: Starting historical data collection");
    info!("   Date range: {} to {}", start_date, end_date);
    info!("   Processing approach: Sequential batches with detailed logging");
    info!("================================================");
    
    // Process stocks in configurable batches
    let batch_size = args.batch_size;
    let mut total_records = 0;
    let mut processed_stocks = 0;
    let mut successful_stocks = 0;
    let mut failed_stocks = 0;

    let total_batches = (all_stocks.len() + batch_size - 1) / batch_size;

    for (batch_num, batch) in all_stocks.chunks(batch_size).enumerate() {
        let batch_start = std::time::Instant::now();
        info!("📦 BATCH {}/{} - Processing {} stocks:", 
              batch_num + 1, total_batches, batch.len());
        
        for stock in batch {
            info!("   - {} ({})", stock.symbol, stock.company_name);
        }

        let mut batch_records = 0;
        let mut batch_successful = 0;
        let mut batch_failed = 0;

        for (i, stock) in batch.iter().enumerate() {
            let stock_start = std::time::Instant::now();
            info!("🔄 [{}/{}] Starting {}: {}", 
                  processed_stocks + i + 1, 
                  all_stocks.len(), 
                  stock.symbol, 
                  stock.company_name);

            match DataCollector::fetch_stock_history(
                schwab_client.clone(),
                database.clone(),
                stock.clone(),
                start_date,
                end_date
            ).await {
                Ok(records) => {
                    batch_records += records;
                    batch_successful += 1;
                    let elapsed = stock_start.elapsed();
                    info!("✅ [{}/{}] {} completed: {} records in {:.1}s", 
                          processed_stocks + i + 1, 
                          all_stocks.len(), 
                          stock.symbol, 
                          records, 
                          elapsed.as_secs_f64());
                }
                Err(e) => {
                    batch_failed += 1;
                    let elapsed = stock_start.elapsed();
                    error!("❌ [{}/{}] {} failed after {:.1}s: {}", 
                           processed_stocks + i + 1, 
                           all_stocks.len(), 
                           stock.symbol, 
                           elapsed.as_secs_f64(), 
                           e);
                }
            }

            // Small delay between stocks
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        processed_stocks += batch.len();
        successful_stocks += batch_successful;
        failed_stocks += batch_failed;
        total_records += batch_records;
        let batch_elapsed = batch_start.elapsed();
        
        info!("📊 BATCH {}/{} SUMMARY:", batch_num + 1, total_batches);
        info!("   ✅ Successful: {}/{} stocks", batch_successful, batch.len());
        info!("   ❌ Failed: {}/{} stocks", batch_failed, batch.len());
        info!("   📈 Records added: {}", batch_records);
        info!("   ⏱️  Time taken: {:.1}s", batch_elapsed.as_secs_f64());
        info!("   📊 OVERALL PROGRESS: {}/{} stocks, {} total records", 
              processed_stocks, all_stocks.len(), total_records);
        info!("================================================");

        // Pause between batches
        if batch_num < total_batches - 1 {
            info!("⏸️  Pausing {} seconds before next batch...", args.batch_delay);
            tokio::time::sleep(tokio::time::Duration::from_secs(args.batch_delay)).await;
        }
    }

    // Final comprehensive summary
    let final_stats = match database.get_stats() {
        Ok((_, price_count, _)) => price_count as i64,
        Err(_) => 0
    };
    
    info!("🎉 HISTORICAL DATA COLLECTION COMPLETED!");
    info!("================================================");
    info!("📊 FINAL STATISTICS:");
    info!("   Stocks processed: {}/{}", processed_stocks, all_stocks.len());
    info!("   ✅ Successful: {} stocks", successful_stocks);
    info!("   ❌ Failed: {} stocks", failed_stocks);
    info!("   📈 Records added this session: {}", total_records);
    info!("   📊 Database before: {} records", initial_stats);
    info!("   📊 Database after: {} records", final_stats);
    info!("   📊 Net increase: {} records", final_stats - initial_stats);
    info!("================================================");

    Ok(())
}