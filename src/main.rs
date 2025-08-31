mod api;
mod analysis;
mod data_collector;
mod database;
mod models;
mod ui;
mod utils;

use anyhow::Result;
use tracing::{info, error, Level};
use tracing_subscriber::{self, FmtSubscriber};

use crate::analysis::AnalysisEngine;
use crate::api::SchwabClient;
use crate::data_collector::DataCollector;
use crate::database::DatabaseManager;
use crate::models::Config;
// use crate::ui::StockApp;

fn main() -> Result<()> {
    // Initialize logging - suppress most logs for TUI
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::ERROR)
        .with_env_filter("rust_stocks=error")
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    // info!("🚀 Starting Rust Stocks Analysis System");

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

    // info!("📋 Configuration loaded successfully");

    // Initialize database
    let database = match DatabaseManager::new(&config.database_path) {
        Ok(db) => db,
        Err(e) => {
            error!("Failed to initialize database: {}", e);
            eprintln!("❌ Database Error: {}", e);
            std::process::exit(1);
        }
    };

    // info!("💾 Database initialized at: {}", config.database_path);

    // Skip Schwab client initialization for TUI testing
    /*
    // Initialize Schwab API client
    let schwab_client = SchwabClient::new(&config)?;
    
    // Initialize data collector
    let data_collector = DataCollector::new(schwab_client, database.clone(), config.clone());
    */

    // Skip heavy initialization for now to get TUI working fast
    /*
    // Initialize analysis engine
    let analysis_engine = AnalysisEngine::new(database);
    
    // Get database statistics
    let stats = analysis_engine.get_summary_stats().await?;
    */

    // Force TUI to run - ignore TTY detection
    println!("🚀 Starting Stock Analysis TUI...");
    
    match ui::app::run_app() {
        Ok(_) => {
            println!("Thanks for using Rust Stocks Analysis System!");
        }
        Err(e) => {
            eprintln!("❌ TUI Error: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

/// CLI interface fallback when TUI is not available
fn run_cli_interface() -> Result<()> {
    println!("");
    println!("📊 STOCK ANALYSIS SYSTEM - CLI MODE");
    println!("===================================");
    println!("");
    println!("Available commands:");
    println!("  📋 update_sp500    - Update S&P 500 company list");
    println!("  📈 collect_data    - Collect historical price data");
    println!("  🔍 check_status    - Check database status");
    println!("  📊 show_stats      - Show database statistics");
    println!("");
    println!("Usage examples:");
    println!("  cargo run --bin update_sp500");
    println!("  cargo run --bin collect_with_detailed_logs -- --start-date 20240101");
    println!("");
    println!("💡 For full TUI experience, run from a proper terminal (not IDE)");
    
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
    println!("\n📋 Step 1: Checking S&P 500 stock list...");
    let stocks = data_collector.get_active_stocks()?;
    if stocks.is_empty() {
        println!("❌ No stocks found in database!");
        println!("💡 Run 'cargo run --bin update_sp500' first to populate the S&P 500 list.");
        return Ok(());
    }
    println!("✅ Found {} stocks in database", stocks.len());
    
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
    println!("\n📋 Step 1: Checking S&P 500 stock list...");
    let stocks = data_collector.get_active_stocks()?;
    if stocks.is_empty() {
        println!("❌ No stocks found in database!");
        println!("💡 Run 'cargo run --bin update_sp500' first to populate the S&P 500 list.");
        return Ok(());
    }
    println!("✅ Found {} stocks in database", stocks.len());
    
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
