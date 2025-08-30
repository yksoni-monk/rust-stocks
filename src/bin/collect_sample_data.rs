use anyhow::Result;
use chrono::NaiveDate;
use std::sync::Arc;
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

    info!("🧪 Starting Sample Historical Data Collection (First 10 stocks)");

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

    // Get first 10 stocks only
    let all_stocks = data_collector.get_active_stocks()?;
    let sample_stocks: Vec<_> = all_stocks.into_iter().take(10).collect();
    
    info!("📊 Testing with first {} stocks:", sample_stocks.len());
    for stock in &sample_stocks {
        info!("   - {} ({})", stock.symbol, stock.company_name);
    }

    // Define date range for historical data (shorter range for testing)
    let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(); // Just 2024 data for testing
    let end_date = chrono::Utc::now().date_naive();
    
    info!("📈 Fetching historical data from {} to {}", start_date, end_date);
    info!("   Expected: ~2,500 price records for 10 stocks over ~1 year");
    
    // Process sample stocks directly (bypassing the full backfill function)
    let schwab_client = Arc::new(SchwabClient::new(&config)?);
    let database = Arc::new(DatabaseManager::new(&config.database_path)?);
    
    let mut total_records = 0;
    for (i, stock) in sample_stocks.iter().enumerate() {
        info!("📈 Processing {}/{}: {} ({})", i + 1, sample_stocks.len(), stock.symbol, stock.company_name);
        
        match rust_stocks::data_collector::DataCollector::fetch_stock_history(
            schwab_client.clone(),
            database.clone(),
            stock.clone(),
            start_date,
            end_date
        ).await {
            Ok(records) => {
                total_records += records;
                info!("✅ {}: {} records added", stock.symbol, records);
            }
            Err(e) => {
                error!("❌ {}: failed - {}", stock.symbol, e);
            }
        }
    }

    info!("✅ Sample historical data collection completed!");
    info!("   Total records inserted: {}", total_records);
    
    // Get final stats
    let final_stats = data_collector.get_collection_stats().await?;
    info!("📊 Final database state:");
    info!("   - Stocks: {}", final_stats.total_stocks);
    info!("   - Price records: {}", final_stats.total_price_records);

    info!("🎉 Sample data collection process completed successfully!");
    Ok(())
}