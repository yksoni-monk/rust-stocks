mod api;
mod analysis;
mod data_collector;
mod database;
mod models;
mod ui;

use anyhow::Result;
use tracing::{info, error, Level};
use tracing_subscriber::{self, FmtSubscriber};

use crate::analysis::AnalysisEngine;
use crate::api::SchwabClient;
use crate::data_collector::DataCollector;
use crate::database::DatabaseManager;
use crate::models::Config;
use crate::ui::StockApp;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_env_filter("rust_stocks=info")
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    info!("🚀 Starting Rust Stocks Analysis System");

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

    info!("📋 Configuration loaded successfully");

    // Initialize database
    let database = match DatabaseManager::new(&config.database_path) {
        Ok(db) => db,
        Err(e) => {
            error!("Failed to initialize database: {}", e);
            eprintln!("❌ Database Error: {}", e);
            std::process::exit(1);
        }
    };

    info!("💾 Database initialized at: {}", config.database_path);

    // Initialize Schwab API client
    let schwab_client = match SchwabClient::new(&config) {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to initialize Schwab client: {}", e);
            eprintln!("❌ API Client Error: {}", e);
            eprintln!("Make sure your Schwab API tokens are valid and accessible.");
            std::process::exit(1);
        }
    };

    info!("🔑 Schwab API client initialized");

    // Initialize data collector
    let data_collector = DataCollector::new(schwab_client, database.clone(), config.clone());
    
    info!("🔄 Data collector initialized");

    // Initialize analysis engine
    let analysis_engine = AnalysisEngine::new(database);
    
    info!("📊 Analysis engine initialized");

    // Get database statistics and determine if we need data collection
    let stats = analysis_engine.get_summary_stats().await?;
    info!("📈 Database contains {} stocks with {} price records", 
          stats.total_stocks, stats.total_price_records);
    
    if let Some(last_update) = stats.last_update_date {
        info!("📅 Last data update: {}", last_update);
    } else {
        info!("📅 No previous data updates found - will need initial data sync");
    }

    // Check if we need to collect data
    if stats.total_stocks == 0 || stats.total_price_records == 0 {
        println!("\n🚀 Welcome to Rust Stocks Analysis System!");
        println!("No stock data found in database. Running initial setup automatically...");
        
        // Run initial setup automatically for now
        match perform_initial_setup_auto(&data_collector).await {
            Ok(_) => println!("✅ Initial setup completed successfully!"),
            Err(e) => {
                println!("❌ Setup failed: {}", e);
                println!("⚠️  Cannot proceed without data. Exiting.");
                std::process::exit(1);
            }
        }
    } else {
        println!("✅ Database has {} stocks with {} price records", 
                stats.total_stocks, stats.total_price_records);
    }

    // Initialize and run the TUI application
    let mut app = StockApp::new(analysis_engine);
    
    info!("🖥️  Starting terminal user interface...");
    
    match app.run().await {
        Ok(_) => {
            info!("👋 Application closed successfully");
            println!("Thanks for using Rust Stocks Analysis System!");
        }
        Err(e) => {
            error!("Application error: {}", e);
            eprintln!("❌ Application Error: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Prompt user for yes/no input
fn prompt_user(message: &str) -> Result<bool> {
    use std::io::{self, Write};
    
    print!("{}", message);
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    Ok(input.trim().to_lowercase().starts_with('y'))
}

/// Perform automatic initial setup with S&P 500 data (no prompts)
async fn perform_initial_setup_auto(data_collector: &DataCollector) -> Result<()> {
    println!("\n📋 Step 1: Syncing S&P 500 stock list...");
    let stocks_added = data_collector.sync_sp500_list().await?;
    println!("✅ Added {} stocks to database", stocks_added);
    
    println!("\n📊 Step 2: Fetching current stock quotes...");
    match data_collector.fetch_current_quotes().await {
        Ok(quotes_updated) => println!("✅ Updated {} stock quotes", quotes_updated),
        Err(e) => {
            println!("❌ Failed to fetch quotes: {}", e);
            println!("⚠️ This might be due to API authentication issues");
            println!("   Continuing with stock list only...");
        }
    }
    
    println!("\n🔍 Step 3: Validating data...");
    let validation_report = data_collector.validate_data_integrity().await?;
    println!("✅ {} stocks have data, {} without data", 
             validation_report.stocks_with_data, 
             validation_report.stocks_without_data);
    
    println!("✅ Basic setup completed! You can run data collection later for historical data.");
    Ok(())
}

/// Perform initial setup with S&P 500 data
async fn perform_initial_setup(data_collector: &DataCollector) -> Result<()> {
    println!("\n📋 Step 1: Syncing S&P 500 stock list...");
    let stocks_added = data_collector.sync_sp500_list().await?;
    println!("✅ Added {} stocks to database", stocks_added);
    
    println!("\n📊 Step 2: Fetching current stock quotes...");
    let quotes_updated = data_collector.fetch_current_quotes().await?;
    println!("✅ Updated {} stock quotes", quotes_updated);
    
    if prompt_user("\nDo you want to fetch historical data from January 2020? This may take several minutes (y/n): ")? {
        println!("\n📈 Step 3: Fetching historical data (this may take 10-15 minutes)...");
        let start_date = chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let historical_records = data_collector.backfill_historical_data(start_date, None).await?;
        println!("✅ Added {} historical price records", historical_records);
    } else {
        println!("⏩ Skipping historical data collection");
    }
    
    println!("\n🔍 Validating data integrity...");
    let validation_report = data_collector.validate_data_integrity().await?;
    println!("✅ {} stocks have data, {} need attention", 
             validation_report.stocks_with_data, 
             validation_report.stocks_without_data);
    
    if !validation_report.issues.is_empty() {
        println!("⚠️  Issues found:");
        for issue in &validation_report.issues[..std::cmp::min(5, validation_report.issues.len())] {
            println!("   - {}", issue);
        }
        if validation_report.issues.len() > 5 {
            println!("   ... and {} more", validation_report.issues.len() - 5);
        }
    }
    
    println!("\n🎉 Initial setup completed successfully!");
    Ok(())
}

/// Perform data update
async fn perform_data_update(data_collector: &DataCollector) -> Result<()> {
    println!("\n🔄 Updating stock data...");
    
    let updated_records = data_collector.incremental_update().await?;
    
    if updated_records > 0 {
        println!("✅ Updated {} price records", updated_records);
    } else {
        println!("✅ Data is already up to date");
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_database_initialization() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let database = DatabaseManager::new(db_path.to_str().unwrap()).unwrap();
        let stats = database.get_stats().unwrap();
        
        assert_eq!(stats.0, 0); // No stocks initially
        assert_eq!(stats.1, 0); // No price records initially
        assert_eq!(stats.2, None); // No last update date
    }

    #[test]
    fn test_config_validation() {
        // Test that config validation works
        std::env::set_var("SCHWAB_API_KEY", "test_key");
        std::env::set_var("SCHWAB_APP_SECRET", "test_secret");
        
        let config = Config::from_env();
        assert!(config.is_ok());
        
        let config = config.unwrap();
        assert_eq!(config.schwab_api_key, "test_key");
        assert_eq!(config.schwab_app_secret, "test_secret");
        assert_eq!(config.rate_limit_per_minute, 120); // default value
    }
}
