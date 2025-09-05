// Simple test to check database state and API connection
use anyhow::Result;
use tracing_subscriber::{FmtSubscriber, EnvFilter};

use rust_stocks::analysis::AnalysisEngine;
use rust_stocks::api::SchwabClient;
use rust_stocks::database_sqlx::DatabaseManagerSqlx;
use rust_stocks::models::Config;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)?;

    println!("🔍 Checking Database State...");

    // Load configuration
    let config = Config::from_env()?;
    println!("✅ Configuration loaded");

    // Initialize database
    let database = DatabaseManagerSqlx::new(&config.database_path).await?;
    println!("✅ Database initialized at: {}", config.database_path);

    // Initialize analysis engine
    let analysis_engine = AnalysisEngine::new(database.clone());
    
    // Get database statistics
    let stats = analysis_engine.get_database_stats().await?;
    
    println!("\n📊 Current Database State:");
    println!("   Total Stocks: {}", stats.total_stocks);
    println!("   Total Price Records: {}", stats.total_price_records);
    
    // Test individual stock stats
    println!("\n🔍 Testing individual stock data stats:");
    let stocks = database.get_active_stocks().await?;
    for stock in stocks.iter().take(3) {
        if let Some(stock_id) = stock.id {
            match database.get_stock_data_stats(stock_id).await {
                Ok(stock_stats) => {
                    println!("   {} (ID: {}): {} data points, {:?} to {:?}", 
                            stock.symbol, stock_id, stock_stats.data_points, 
                            stock_stats.earliest_date, stock_stats.latest_date);
                }
                Err(e) => {
                    println!("   {} (ID: {}): ERROR - {}", stock.symbol, stock_id, e);
                }
            }
        } else {
            println!("   {}: No ID", stock.symbol);
        }
    }
    
    if let Some(last_update) = stats.last_update_date {
        println!("   Last Update: {}", last_update);
    } else {
        println!("   Last Update: Never");
    }

    // Test API connection (without making actual calls)
    match SchwabClient::new(&config) {
        Ok(_) => println!("✅ Schwab API client initialized successfully"),
        Err(e) => println!("❌ Schwab API client error: {}", e),
    }

    if stats.total_stocks == 0 {
        println!("\n🚀 Database is empty - ready for initial setup!");
    } else {
        println!("\n✅ Database has data - ready for analysis!");
        
        if let Some(top_decliner) = stats.top_pe_decliner {
            println!("   Top P/E Decliner: {} ({:.1}% decline)", 
                     top_decliner.0, top_decliner.1);
        }
    }

    Ok(())
}