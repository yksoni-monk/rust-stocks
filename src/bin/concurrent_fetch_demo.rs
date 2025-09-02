//! Demo binary for concurrent stock data fetching

use anyhow::Result;
use chrono::NaiveDate;
use std::sync::Arc;
use tracing::{info, error};

use rust_stocks::{
    concurrent_fetcher::{ConcurrentFetchConfig, DateRange, fetch_stocks_concurrently},
    database::DatabaseManager,
    models::Config,
};

#[tokio::main]
async fn main() -> Result<()> {
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

    // Demo 1: Small date range with few threads
    info!("\n📅 Demo 1: Small date range (January 2024) with 5 threads");
    let demo1_config = ConcurrentFetchConfig {
        date_range: DateRange {
            start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2024, 1, 31).unwrap(),
        },
        num_threads: 5,
        retry_attempts: 2,
    };

    let result1 = fetch_stocks_concurrently(database.clone(), demo1_config).await?;
    info!("✅ Demo 1 Results:");
    info!("   - Total stocks: {}", result1.total_stocks);
    info!("   - Processed: {}", result1.processed_stocks);
    info!("   - Skipped: {}", result1.skipped_stocks);
    info!("   - Failed: {}", result1.failed_stocks);
    info!("   - Records fetched: {}", result1.total_records_fetched);

    // Demo 2: Larger date range with more threads
    info!("\n📅 Demo 2: Larger date range (2020-2025) with 10 threads");
    let demo2_config = ConcurrentFetchConfig {
        date_range: DateRange {
            start_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2025, 8, 31).unwrap(),
        },
        num_threads: 10,
        retry_attempts: 3,
    };

    let result2 = fetch_stocks_concurrently(database.clone(), demo2_config).await?;
    info!("✅ Demo 2 Results:");
    info!("   - Total stocks: {}", result2.total_stocks);
    info!("   - Processed: {}", result2.processed_stocks);
    info!("   - Skipped: {}", result2.skipped_stocks);
    info!("   - Failed: {}", result2.failed_stocks);
    info!("   - Records fetched: {}", result2.total_records_fetched);

    // Summary
    info!("\n📈 Summary:");
    info!("   - Total records fetched across both demos: {}", 
          result1.total_records_fetched + result2.total_records_fetched);
    info!("   - Total stocks processed: {}", 
          result1.processed_stocks + result2.processed_stocks);
    info!("   - Total stocks skipped: {}", 
          result1.skipped_stocks + result2.skipped_stocks);
    info!("   - Total stocks failed: {}", 
          result1.failed_stocks + result2.failed_stocks);

    info!("\n🎉 Concurrent fetching demo completed successfully!");
    Ok(())
}
