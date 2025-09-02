use anyhow::{Result, anyhow};
use chrono::NaiveDate;
use clap::{Parser, Subcommand};
use std::sync::Arc;
use tracing::{info, error, Level};
use tracing_subscriber::{self, FmtSubscriber};

use rust_stocks::{
    api::{SchwabClient, StockDataProvider},
    concurrent_fetcher::{ConcurrentFetchConfig, DateRange, fetch_stocks_concurrently},
    data_collector::DataCollector,
    database::DatabaseManager,
    models::Config,
    utils::MarketCalendar,
};

/// Comprehensive data collection testing tool
#[derive(Parser)]
#[command(name = "data_collection_test")]
#[command(version = "1.0.0")]
#[command(about = "Test different data collection methods")]
#[command(long_about = "
This tool provides multiple ways to test stock data collection:
- Quick test: Simple collection for 10 stocks
- Detailed collection: Full production-like collection with logging
- Concurrent collection: Multi-threaded collection demo

Examples:
  cargo run --bin data_collection_test quick 20240101
  cargo run --bin data_collection_test detailed -s 20240101 -e 20240131
  cargo run --bin data_collection_test concurrent -s 20240101 --threads 5
")]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Quick test with 10 stocks
    Quick {
        /// Start date in YYYYMMDD format
        start_date: String,
        /// End date in YYYYMMDD format (optional, defaults to start_date)
        end_date: Option<String>,
    },
    
    /// Detailed collection with full logging
    Detailed {
        /// Start date in YYYYMMDD format
        #[arg(long, short = 's')]
        start_date: String,
        /// End date in YYYYMMDD format (optional, defaults to today)
        #[arg(long, short = 'e')]
        end_date: Option<String>,
        /// Batch size for processing stocks
        #[arg(long, short = 'b', default_value_t = 5)]
        batch_size: usize,
        /// Delay between batches in seconds
        #[arg(long, short = 'd', default_value_t = 3)]
        batch_delay: u64,
    },
    
    /// Concurrent collection demo
    Concurrent {
        /// Start date in YYYYMMDD format
        #[arg(long, short = 's')]
        start_date: Option<String>,
        /// End date in YYYYMMDD format
        #[arg(long, short = 'e')]
        end_date: Option<String>,
        /// Number of concurrent threads
        #[arg(long, short = 't', default_value_t = 10)]
        threads: usize,
        /// Number of retry attempts per stock
        #[arg(long, short = 'r', default_value_t = 3)]
        retries: u32,
    },
    
    /// Single stock testing
    Single {
        /// Stock symbol
        symbol: String,
        /// Start date in YYYYMMDD format
        start_date: String,
        /// End date in YYYYMMDD format
        end_date: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    let args = Args::parse();

    match args.command {
        Commands::Quick { start_date, end_date } => {
            quick_test(&start_date, end_date.as_deref()).await?;
        }
        Commands::Detailed { start_date, end_date, batch_size, batch_delay } => {
            detailed_collection(&start_date, end_date, batch_size, batch_delay).await?;
        }
        Commands::Concurrent { start_date, end_date, threads, retries } => {
            concurrent_collection(start_date.as_deref(), end_date.as_deref(), threads, retries).await?;
        }
        Commands::Single { symbol, start_date, end_date } => {
            single_stock_test(&symbol, &start_date, &end_date).await?;
        }
    }

    Ok(())
}

async fn quick_test(start_str: &str, end_str: Option<&str>) -> Result<()> {
    let end_str = end_str.unwrap_or(start_str);
    
    info!("🗓️  Quick Data Collection Test");
    info!("📅 Date range: {} to {}", start_str, end_str);

    // Parse dates
    let start_date = parse_date_simple(start_str)?;
    let end_date = parse_date_simple(end_str)?;

    // Weekend adjustment
    let adjusted_start = MarketCalendar::adjust_for_weekend(start_date);
    let adjusted_end = MarketCalendar::adjust_for_weekend(end_date);

    if adjusted_start != start_date || adjusted_end != end_date {
        info!("📅 Date range adjusted for weekends:");
        info!("   Original: {} to {}", start_date, end_date);
        info!("   Adjusted: {} to {}", adjusted_start, adjusted_end);
    }

    // Setup
    let config = Config::from_env()?;
    let database = DatabaseManager::new(&config.database_path)?;
    let client = SchwabClient::new(&config)?;

    // Get first 10 stocks
    let data_collector = DataCollector::new(client, database, config);
    let stocks = data_collector.get_active_stocks()?;
    let test_stocks: Vec<_> = stocks.into_iter().take(10).collect();

    info!("📊 Testing with {} stocks", test_stocks.len());

    // Process each stock
    let mut total_records = 0;
    for (i, stock) in test_stocks.iter().enumerate() {
        info!("[{}/{}] Processing {}...", i+1, test_stocks.len(), stock.symbol);
        
        let schwab_client = SchwabClient::new(&Config::from_env()?)?;
        match schwab_client.get_price_history(&stock.symbol, adjusted_start, adjusted_end).await {
            Ok(bars) => {
                info!("✅ {} records", bars.len());
                total_records += bars.len();
                
                if i == 0 && !bars.is_empty() {
                    info!("   Sample: Open=${:.2}, High=${:.2}, Low=${:.2}, Close=${:.2}", 
                          bars[0].open, bars[0].high, bars[0].low, bars[0].close);
                }
            }
            Err(e) => {
                error!("❌ Error: {}", e);
            }
        }
    }

    info!("🎉 Quick test completed! Total records: {}", total_records);
    Ok(())
}

async fn detailed_collection(start_str: &str, end_str: Option<String>, batch_size: usize, batch_delay: u64) -> Result<()> {
    let end_str = end_str.unwrap_or_else(|| {
        chrono::Utc::now().date_naive().format("%Y%m%d").to_string()
    });

    info!("🔍 Detailed Data Collection");
    info!("📅 Date range: {} to {}", start_str, end_str);
    info!("📦 Batch size: {}", batch_size);
    info!("⏱️  Batch delay: {}s", batch_delay);

    // Parse dates with validation
    let start_date = parse_date_detailed(start_str, "start_date")?;
    let end_date = parse_date_detailed(&end_str, "end_date")?;

    if start_date > end_date {
        return Err(anyhow!("Start date cannot be after end date"));
    }

    // Setup
    let config = Config::from_env()?;
    let database = DatabaseManager::new(&config.database_path)?;

    // Get all active stocks
    let stocks = database.get_active_stocks()?;
    info!("📊 Found {} active stocks", stocks.len());

    // Process in batches
    let mut total_records = 0;
    let mut successful = 0;
    let mut failed = 0;

    for (batch_num, chunk) in stocks.chunks(batch_size).enumerate() {
        info!("📦 Processing batch {}/{} ({} stocks)", 
              batch_num + 1, (stocks.len() + batch_size - 1) / batch_size, chunk.len());

        for stock in chunk {
            info!("  📈 Processing {}", stock.symbol);
            
            let client = SchwabClient::new(&Config::from_env()?)?;
            match DataCollector::fetch_stock_history(
                Arc::new(client),
                Arc::new(database.clone()),
                stock.clone(),
                start_date,
                end_date
            ).await {
                Ok(records) => {
                    info!("    ✅ {} records", records);
                    total_records += records;
                    successful += 1;
                }
                Err(e) => {
                    error!("    ❌ Error: {}", e);
                    failed += 1;
                }
            }
        }

        if batch_num < stocks.chunks(batch_size).count() - 1 {
            info!("⏱️  Waiting {} seconds before next batch...", batch_delay);
            tokio::time::sleep(tokio::time::Duration::from_secs(batch_delay)).await;
        }
    }

    info!("🎉 Detailed collection completed!");
    info!("   - Successful: {}", successful);
    info!("   - Failed: {}", failed);
    info!("   - Total records: {}", total_records);
    Ok(())
}

async fn concurrent_collection(start_str: Option<&str>, end_str: Option<&str>, threads: usize, retries: u32) -> Result<()> {
    info!("🚀 Concurrent Data Collection Demo");
    info!("🧵 Threads: {}", threads);
    info!("🔄 Retries: {}", retries);

    // Setup
    let config = Config::from_env()?;
    let database = DatabaseManager::new(&config.database_path)?;
    let database = Arc::new(database);

    // Check if we have stocks
    let stocks = database.get_active_stocks()?;
    if stocks.is_empty() {
        error!("❌ No stocks found in database. Please run 'cargo run --bin update_sp500' first.");
        return Ok(());
    }

    info!("📊 Found {} active stocks in database", stocks.len());

    // Parse date range
    let (start_date, end_date) = parse_date_range_concurrent(start_str, end_str)?;
    info!("📅 Date Range: {} to {}", start_date, end_date);

    // Run concurrent fetch
    let fetch_config = ConcurrentFetchConfig {
        date_range: DateRange {
            start_date,
            end_date,
        },
        num_threads: threads,
        retry_attempts: retries,
        max_stocks: None, // No limit for production use
    };

    let result = fetch_stocks_concurrently(database, fetch_config).await?;
    
    info!("✅ Concurrent fetch results:");
    info!("   - Total stocks: {}", result.total_stocks);
    info!("   - Processed: {}", result.processed_stocks);
    info!("   - Skipped: {}", result.skipped_stocks);
    info!("   - Failed: {}", result.failed_stocks);
    info!("   - Records fetched: {}", result.total_records_fetched);

    info!("🎉 Concurrent fetching completed!");
    Ok(())
}

async fn single_stock_test(symbol: &str, start_str: &str, end_str: &str) -> Result<()> {
    info!("📈 Single Stock Data Collection Test");
    info!("====================================");
    info!("Symbol: {}", symbol);
    info!("Date range: {} to {}", start_str, end_str);

    // Parse dates
    let start_date = parse_date_simple(start_str)?;
    let end_date = parse_date_simple(end_str)?;

    info!("Parsed dates: {} to {}", start_date, end_date);

    // Setup
    let config = Config::from_env()?;
    let database = DatabaseManager::new(&config.database_path)?;
    let client = SchwabClient::new(&config)?;

    info!("✅ Setup complete");

    // Check if stock exists in database
    info!("📊 Checking stock in database...");
    match database.get_stock_by_symbol(symbol)? {
        Some(stock) => {
            info!("✅ Stock found in database: {} ({})", stock.symbol, stock.company_name);
        }
        None => {
            info!("⚠️  Stock not found in database, will try to fetch from API");
        }
    }

    // Check database stats
    info!("📊 Database statistics...");
    let (total_stocks, total_prices, last_update) = database.get_stats()?;
    info!("Total stocks: {}", total_stocks);
    info!("Total price records: {}", total_prices);
    info!("Last update: {:?}", last_update);

    // Fetch price history
    info!("📈 Fetching price history for {} from {} to {}", symbol, start_date, end_date);
    
    match client.get_price_history(symbol, start_date, end_date).await {
        Ok(bars) => {
            info!("✅ Successfully fetched {} price bars", bars.len());
            
            if !bars.is_empty() {
                info!("📊 Sample data:");
                info!("  First bar: Open=${:.2}, High=${:.2}, Low=${:.2}, Close=${:.2}, Volume={}", 
                     bars[0].open, bars[0].high, bars[0].low, bars[0].close, bars[0].volume);
                
                if bars.len() > 1 {
                    let last_bar = &bars[bars.len() - 1];
                    info!("  Last bar: Open=${:.2}, High=${:.2}, Low=${:.2}, Close=${:.2}, Volume={}", 
                         last_bar.open, last_bar.high, last_bar.low, last_bar.close, last_bar.volume);
                }
            }
        }
        Err(e) => {
            error!("❌ Failed to fetch price history: {}", e);
            return Err(e);
        }
    }

    info!("🎉 Single stock test completed successfully!");
    Ok(())
}

// Helper functions
fn parse_date_simple(date_str: &str) -> Result<NaiveDate> {
    if date_str.len() != 8 {
        return Err(anyhow!("Date must be YYYYMMDD format"));
    }
    
    let year: i32 = date_str[0..4].parse()?;
    let month: u32 = date_str[4..6].parse()?;
    let day: u32 = date_str[6..8].parse()?;
    
    NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| anyhow!("Invalid date"))
}

fn parse_date_detailed(date_str: &str, field_name: &str) -> Result<NaiveDate> {
    if date_str.len() != 8 {
        return Err(anyhow!("{} must be exactly 8 digits in YYYYMMDD format, got: {}", field_name, date_str));
    }
    
    if !date_str.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("{} must contain only digits, got: {}", field_name, date_str));
    }
    
    let year: i32 = date_str[0..4].parse()
        .map_err(|_| anyhow!("Invalid year in {}: {}", field_name, &date_str[0..4]))?;
    let month: u32 = date_str[4..6].parse()
        .map_err(|_| anyhow!("Invalid month in {}: {}", field_name, &date_str[4..6]))?;
    let day: u32 = date_str[6..8].parse()
        .map_err(|_| anyhow!("Invalid day in {}: {}", field_name, &date_str[6..8]))?;
    
    if year < 1970 || year > 2050 {
        return Err(anyhow!("Year in {} must be between 1970 and 2050, got: {}", field_name, year));
    }
    
    if month < 1 || month > 12 {
        return Err(anyhow!("Month in {} must be between 01 and 12, got: {:02}", field_name, month));
    }
    
    if day < 1 || day > 31 {
        return Err(anyhow!("Day in {} must be between 01 and 31, got: {:02}", field_name, day));
    }
    
    NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| anyhow!("Invalid date in {}: {}-{:02}-{:02}", field_name, year, month, day))
}

fn parse_date_range_concurrent(start_str: Option<&str>, end_str: Option<&str>) -> Result<(NaiveDate, NaiveDate)> {
    let start_date = if let Some(start_str) = start_str {
        NaiveDate::parse_from_str(start_str, "%Y%m%d")
            .map_err(|e| anyhow!("Invalid start date format: {}. Expected YYYYMMDD", e))?
    } else {
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
    };

    let end_date = if let Some(end_str) = end_str {
        NaiveDate::parse_from_str(end_str, "%Y%m%d")
            .map_err(|e| anyhow!("Invalid end date format: {}. Expected YYYYMMDD", e))?
    } else {
        chrono::Utc::now().date_naive()
    };

    Ok((start_date, end_date))
}
