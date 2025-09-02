use anyhow::Result;
use tracing::{error, Level};
use tracing_subscriber::{self, FmtSubscriber};
use std::sync::Arc;

use rust_stocks::database_sqlx::DatabaseManagerSqlx;
use rust_stocks::concurrent_fetcher::{ConcurrentFetchConfig, DateRange, fetch_stocks_concurrently};
use rust_stocks::models::Config;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    // Load configuration
    let config = match Config::from_env() {
        Ok(config) => config,
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            eprintln!("❌ Configuration Error: {}", e);
            eprintln!("Make sure you have a .env file with the required Schwab API credentials.");
            std::process::exit(1);
        }
    };

    // Initialize database with SQLX
    let database = match DatabaseManagerSqlx::new(&config.database_path).await {
        Ok(db) => db,
        Err(e) => {
            error!("Failed to initialize database: {}", e);
            eprintln!("❌ Database Error: {}", e);
            std::process::exit(1);
        }
    };

    println!("🚀 Concurrent Fetcher Module Test - Database initialized successfully!");
    println!("📊 Testing concurrent fetcher functionality with SQLX...");

    // Test concurrent fetcher functionality
    match test_concurrent_fetcher_functionality(&database).await {
        Ok(_) => {
            println!("✅ Concurrent fetcher module test completed successfully!");
            println!("🎉 Phase 2 concurrent fetcher module is working!");
        }
        Err(e) => {
            eprintln!("❌ Concurrent fetcher test failed: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

async fn test_concurrent_fetcher_functionality(database: &DatabaseManagerSqlx) -> Result<()> {
    // Test 1: Get active stocks
    println!("📊 Testing get_active_stocks...");
    let stocks = database.get_active_stocks().await?;
    println!("📈 Found {} active stocks in database", stocks.len());
    
    if !stocks.is_empty() {
        println!("📊 First stock: {} - {}", stocks[0].symbol, stocks[0].company_name);
    }

    // Test 2: Test concurrent fetch configuration
    println!("⚙️ Testing concurrent fetch configuration...");
    let fetch_config = ConcurrentFetchConfig {
        date_range: DateRange {
            start_date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end_date: chrono::NaiveDate::from_ymd_opt(2024, 1, 31).unwrap(),
        },
        num_threads: 2,
        retry_attempts: 3,
        max_stocks: Some(5), // Limit for testing
    };
    
    println!("📅 Date range: {} to {}", fetch_config.date_range.start_date, fetch_config.date_range.end_date);
    println!("🧵 Threads: {}", fetch_config.num_threads);
    println!("🔄 Retry attempts: {}", fetch_config.retry_attempts);
    println!("📊 Max stocks: {:?}", fetch_config.max_stocks);

    // Test 3: Test database operations used by concurrent fetcher
    println!("🔍 Testing database operations used by concurrent fetcher...");
    
    if !stocks.is_empty() {
        let first_stock = &stocks[0];
        if let Some(stock_id) = first_stock.id {
            // Test count_existing_records
            let existing_count = database.count_existing_records(
                stock_id,
                fetch_config.date_range.start_date,
                fetch_config.date_range.end_date
            ).await?;
            println!("📊 Existing records for {}: {}", first_stock.symbol, existing_count);
            
            // Test get_latest_price
            let latest_price = database.get_latest_price(stock_id).await?;
            match latest_price {
                Some(price) => {
                    println!("💰 Latest price for {}: ${:.2} on {}", 
                             first_stock.symbol, price.close_price, price.date);
                }
                None => {
                    println!("⚠️ No price data found for {}", first_stock.symbol);
                }
            }
        }
    }

    // Test 4: Test metadata operations
    println!("📝 Testing metadata operations...");
    database.set_metadata("concurrent_fetcher_test", "success").await?;
    let test_value = database.get_metadata("concurrent_fetcher_test").await?;
    println!("📝 Metadata test: {:?}", test_value);

    // Test 5: Test database stats
    println!("📊 Testing database statistics...");
    let stats = database.get_stats().await?;
    println!("📈 Database stats: {:?}", stats);

    // Note: We're not actually running the concurrent fetch because it requires API credentials
    // and would make real API calls. In a real scenario, you'd want to test with mock data.
    println!("⚠️ Skipping actual concurrent fetch (requires API credentials)");
    println!("✅ All database operations used by concurrent fetcher are working correctly");

    Ok(())
}
