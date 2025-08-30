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

    info!("🚀 Starting Robust Historical Data Collection");

    // Load configuration
    let config = Config::from_env()?;
    info!("📋 Configuration loaded successfully");

    // Initialize database
    let database = Arc::new(DatabaseManager::new(&config.database_path)?);
    info!("💾 Database initialized at: {}", config.database_path);

    // Initialize Schwab API client
    let schwab_client = Arc::new(SchwabClient::new(&config)?);
    info!("🔑 Schwab API client initialized");

    // Initialize data collector  
    let data_collector = DataCollector::new(
        SchwabClient::new(&config)?, 
        DatabaseManager::new(&config.database_path)?, 
        config.clone()
    );
    
    // Get all stocks
    let all_stocks = data_collector.get_active_stocks()?;
    info!("📊 Found {} stocks to process", all_stocks.len());

    // Check current database state
    let initial_count = match database.get_stats() {
        Ok((_, price_count, _)) => price_count as i64,
        Err(_) => 0
    };
    info!("📈 Starting with {} existing price records", initial_count);

    // Define date range for historical data
    let start_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    let end_date = chrono::Utc::now().date_naive();
    
    info!("📈 Processing historical data from {} to {}", start_date, end_date);
    info!("   Processing in batches of 25 stocks with progress tracking");
    
    // Process stocks in smaller batches to avoid hangs
    let batch_size = 25;
    let mut total_records = 0;
    let mut processed_stocks = 0;

    for (batch_num, batch) in all_stocks.chunks(batch_size).enumerate() {
        let batch_start = std::time::Instant::now();
        info!("📦 Processing batch {} of {} ({} stocks)", 
              batch_num + 1, 
              (all_stocks.len() + batch_size - 1) / batch_size,
              batch.len());

        let mut batch_records = 0;
        for (i, stock) in batch.iter().enumerate() {
            let stock_start = std::time::Instant::now();
            info!("📈 {}/{}: Processing {} ({})", 
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
                    let elapsed = stock_start.elapsed();
                    info!("✅ {}: {} records added in {:.1}s", 
                          stock.symbol, records, elapsed.as_secs_f64());
                }
                Err(e) => {
                    error!("❌ {}: failed - {}", stock.symbol, e);
                }
            }

            // Add small delay between stocks to be nice to the API
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        processed_stocks += batch.len();
        total_records += batch_records;
        let batch_elapsed = batch_start.elapsed();
        
        info!("✅ Batch {} completed: {} records in {:.1}s ({:.1} records/sec)", 
              batch_num + 1, 
              batch_records, 
              batch_elapsed.as_secs_f64(),
              batch_records as f64 / batch_elapsed.as_secs_f64());
        
        info!("📊 Overall progress: {}/{} stocks processed, {} total records", 
              processed_stocks, all_stocks.len(), total_records);

        // Add delay between batches to avoid overwhelming the API
        if batch_num < (all_stocks.len() + batch_size - 1) / batch_size - 1 {
            info!("⏸️  Pausing 5 seconds between batches...");
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }

    // Final stats
    let final_count = match database.get_stats() {
        Ok((_, price_count, _)) => price_count as i64,
        Err(_) => 0
    };
    
    info!("🎉 Historical data collection completed!");
    info!("   Processed {} stocks", processed_stocks);
    info!("   Records added this session: {}", total_records);
    info!("   Total records in database: {}", final_count);
    info!("   Net new records: {}", final_count - initial_count);

    Ok(())
}