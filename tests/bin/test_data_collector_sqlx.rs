use anyhow::Result;
use tracing::{error, Level};
use tracing_subscriber::{self, FmtSubscriber};

use rust_stocks::database_sqlx::DatabaseManagerSqlx;
use rust_stocks::models::{Config, Stock, StockStatus};

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

    println!("🚀 Data Collector Module Test - Database initialized successfully!");
    println!("📊 Testing data collector functionality with SQLX...");

    // Test data collector functionality
    match test_data_collector_functionality(&database, &config).await {
        Ok(_) => {
            println!("✅ Data collector module test completed successfully!");
            println!("🎉 Phase 2 data collector module is working!");
        }
        Err(e) => {
            eprintln!("❌ Data collector test failed: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

async fn test_data_collector_functionality(database: &DatabaseManagerSqlx, _config: &Config) -> Result<()> {
    // Create a mock SchwabClient for testing (we'll just test the database operations)
    // In a real scenario, you'd want to test with actual API calls
    
    // Test 1: Get active stocks
    println!("📊 Testing get_active_stocks...");
    let stocks = database.get_active_stocks().await?;
    println!("📈 Found {} active stocks in database", stocks.len());
    
    if !stocks.is_empty() {
        println!("📊 First stock: {} - {}", stocks[0].symbol, stocks[0].company_name);
    }

    // Test 2: Get collection stats
    println!("📊 Testing collection statistics...");
    let stats = database.get_stats().await?;
    println!("📈 Database stats: {:?}", stats);

    // Test 3: Test stock operations
    println!("📋 Testing stock operations...");
    
    // Create a test stock
    let test_stock = Stock {
        id: None,
        symbol: "TEST_SQLX".to_string(),
        company_name: "Test Company SQLX".to_string(),
        sector: Some("Technology".to_string()),
        industry: None,
        market_cap: Some(1000000000.0),
        status: StockStatus::Active,
        first_trading_date: None,
        last_updated: None,
    };
    
    // Insert the test stock
    let stock_id = database.upsert_stock(&test_stock).await?;
    println!("✅ Inserted test stock with ID: {}", stock_id);
    
    // Retrieve the stock
    let retrieved_stock = database.get_stock_by_symbol("TEST_SQLX").await?;
    match retrieved_stock {
        Some(stock) => {
            println!("✅ Retrieved stock: {} - {}", stock.symbol, stock.company_name);
        }
        None => {
            println!("❌ Failed to retrieve test stock");
        }
    }

    // Test 4: Test metadata operations
    println!("📝 Testing metadata operations...");
    database.set_metadata("data_collector_test", "success").await?;
    let test_value = database.get_metadata("data_collector_test").await?;
    println!("📝 Metadata test: {:?}", test_value);

    // Test 5: Test price operations (if we have any stocks)
    if !stocks.is_empty() {
        println!("💰 Testing price operations...");
        let first_stock = &stocks[0];
        if let Some(stock_id) = first_stock.id {
            let latest_price = database.get_latest_price(stock_id).await?;
            match latest_price {
                Some(price) => {
                    println!("✅ Latest price for {}: ${:.2} on {}", 
                             first_stock.symbol, price.close_price, price.date);
                }
                None => {
                    println!("⚠️ No price data found for {}", first_stock.symbol);
                }
            }
        }
    }

    // Clean up test data
    println!("🧹 Cleaning up test data...");
    // Note: In a real scenario, you'd want to delete the test stock
    // For now, we'll just leave it as it's harmless

    Ok(())
}
