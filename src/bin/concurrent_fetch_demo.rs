//! Demo binary for concurrent stock data fetching

use anyhow::Result;
use chrono::NaiveDate;
use std::sync::Arc;
use tracing::{info, error};
use clap::Parser;

use rust_stocks::{
    concurrent_fetcher::{ConcurrentFetchConfig, DateRange, fetch_stocks_concurrently},
    database::DatabaseManager,
    models::Config,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Start date in YYYYMMDD format
    #[arg(short, long)]
    start_date: Option<String>,

    /// End date in YYYYMMDD format
    #[arg(short, long)]
    end_date: Option<String>,

    /// Number of concurrent threads
    #[arg(short, long, default_value_t = 10)]
    threads: usize,

    /// Number of retry attempts per stock
    #[arg(short, long, default_value_t = 3)]
    retries: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize logging
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    info!("🚀 Concurrent Stock Data Fetcher Demo");
    info!("=====================================");

    // Load configuration
    let config = Config::from_env()?;
    let database = DatabaseManager::new(&config.database_path)?;
    let database = Arc::new(database);

    // Check if we have stocks in the database
    let stocks = database.get_active_stocks()?;
    if stocks.is_empty() {
        error!("❌ No stocks found in database. Please run 'cargo run --bin update_sp500' first.");
        return Ok(());
    }

    info!("📊 Found {} active stocks in database", stocks.len());

    // Parse date range from command line arguments
    let (start_date, end_date) = parse_date_range(&args)?;
    
    info!("📅 Date Range: {} to {}", start_date, end_date);
    info!("🧵 Threads: {}", args.threads);
    info!("🔄 Retries: {}", args.retries);

    // Run concurrent fetch with specified configuration
    let fetch_config = ConcurrentFetchConfig {
        date_range: DateRange {
            start_date,
            end_date,
        },
        num_threads: args.threads,
        retry_attempts: args.retries,
    };

    let result = fetch_stocks_concurrently(database, fetch_config).await?;
    
    info!("✅ Fetch Results:");
    info!("   - Total stocks: {}", result.total_stocks);
    info!("   - Processed: {}", result.processed_stocks);
    info!("   - Skipped: {}", result.skipped_stocks);
    info!("   - Failed: {}", result.failed_stocks);
    info!("   - Records fetched: {}", result.total_records_fetched);

    info!("\n🎉 Concurrent fetching completed successfully!");
    Ok(())
}

fn parse_date_range(args: &Args) -> Result<(NaiveDate, NaiveDate)> {
    let start_date = if let Some(start_str) = &args.start_date {
        NaiveDate::parse_from_str(start_str, "%Y%m%d")
            .map_err(|e| anyhow::anyhow!("Invalid start date format: {}. Expected YYYYMMDD", e))?
    } else {
        // Default to January 1, 2024
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
    };

    let end_date = if let Some(end_str) = &args.end_date {
        NaiveDate::parse_from_str(end_str, "%Y%m%d")
            .map_err(|e| anyhow::anyhow!("Invalid end date format: {}. Expected YYYYMMDD", e))?
    } else {
        // Default to today
        chrono::Utc::now().date_naive()
    };

    if start_date > end_date {
        return Err(anyhow::anyhow!("Start date must be before end date"));
    }

    Ok((start_date, end_date))
}
