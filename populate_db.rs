// Simple program to populate database with stock data
use anyhow::Result;
use tracing_subscriber::{FmtSubscriber, EnvFilter};

mod api;
mod analysis;
mod data_collector;
mod database;
mod models;

use crate::api::SchwabClient;
use crate::data_collector::DataCollector;
use crate::database::DatabaseManager;
use crate::models::Config;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)?;

    println!("🚀 Starting Database Population...");

    // Load configuration
    let config = Config::from_env()?;
    println!("✅ Configuration loaded");

    // Initialize database
    let database = DatabaseManager::new(&config.database_path)?;
    println!("✅ Database initialized");

    // Initialize Schwab API client
    let schwab_client = SchwabClient::new(&config)?;
    println!("✅ Schwab API client initialized");

    // Initialize data collector
    let data_collector = DataCollector::new(schwab_client, database, config);
    println!("✅ Data collector initialized");

    // Step 1: Sync S&P 500 stock list
    println!("\n📋 Step 1: Syncing S&P 500 stock list...");
    match data_collector.sync_sp500_list().await {
        Ok(stocks_added) => println!("✅ Added {} stocks to database", stocks_added),
        Err(e) => {
            println!("❌ Failed to sync stock list: {}", e);
            return Err(e);
        }
    }

    // Step 2: Fetch current quotes
    println!("\n📊 Step 2: Fetching current stock quotes...");
    match data_collector.fetch_current_quotes().await {
        Ok(quotes_updated) => println!("✅ Updated {} stock quotes", quotes_updated),
        Err(e) => {
            println!("❌ Failed to fetch current quotes: {}", e);
            // Don't return error here - we can continue without current quotes
            println!("⚠️ Continuing without current quotes...");
        }
    }

    // Step 3: Validate what we have
    println!("\n🔍 Step 3: Validating data...");
    match data_collector.validate_data_integrity().await {
        Ok(report) => {
            println!("✅ {} stocks have data, {} need attention", 
                     report.stocks_with_data, 
                     report.stocks_without_data);
            
            if !report.issues.is_empty() && report.issues.len() <= 5 {
                println!("⚠️ Issues found:");
                for issue in &report.issues {
                    println!("   - {}", issue);
                }
            }
        }
        Err(e) => println!("⚠️ Validation failed: {}", e),
    }

    println!("\n🎉 Database population completed!");
    println!("You can now run the main application to see the analysis interface.");

    Ok(())
}