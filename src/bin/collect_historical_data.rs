use anyhow::Result;
use chrono::NaiveDate;
use tracing::{info, error, Level};
use tracing_subscriber::{self, FmtSubscriber};

use rust_stocks::api::SchwabClient;
use rust_stocks::data_collector::DataCollector;
use rust_stocks::database::DatabaseManager;
use rust_stocks::models::Config;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_env_filter("rust_stocks=info")
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    info!("🚀 Starting Historical Data Collection");

    // Load configuration
    let config = Config::from_env()?;
    info!("📋 Configuration loaded successfully");

    // Initialize database
    let database = DatabaseManager::new(&config.database_path)?;
    info!("💾 Database initialized at: {}", config.database_path);

    // Initialize Schwab API client
    let schwab_client = SchwabClient::new(&config)?;
    info!("🔑 Schwab API client initialized");

    // Initialize data collector
    let data_collector = DataCollector::new(schwab_client, database, config.clone());
    info!("🔄 Data collector initialized");

    // Check current database state
    let stats = data_collector.get_collection_stats().await?;
    info!("📊 Current database state:");
    info!("   - Stocks: {}", stats.total_stocks);
    info!("   - Price records: {}", stats.total_price_records);
    
    if let Some(last_update) = stats.last_update_date {
        info!("   - Last update: {}", last_update);
    } else {
        info!("   - No previous updates found");
    }

    // Define date range for historical data
    let start_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    let end_date = chrono::Utc::now().date_naive();
    
    info!("📈 Starting historical data collection from {} to {}", start_date, end_date);
    info!("   Expected: ~1.5M price records for 503 stocks over ~5 years");
    info!("   This may take 20-30 minutes to complete...");
    
    // Start with a progress check every minute
    let _progress_start = std::time::Instant::now();
    
    // Start the historical data backfill
    match data_collector.backfill_historical_data(start_date, Some(end_date)).await {
        Ok(total_records) => {
            info!("✅ Historical data collection completed!");
            info!("   Total records inserted: {}", total_records);
            
            // Get final stats
            let final_stats = data_collector.get_collection_stats().await?;
            info!("📊 Final database state:");
            info!("   - Stocks: {}", final_stats.total_stocks);
            info!("   - Price records: {}", final_stats.total_price_records);
        }
        Err(e) => {
            error!("❌ Historical data collection failed: {}", e);
            return Err(e);
        }
    }

    info!("🎉 Historical data collection process completed successfully!");
    Ok(())
}