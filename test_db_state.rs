// Simple test to check database state and API connection
use anyhow::Result;
use tracing_subscriber::{FmtSubscriber, EnvFilter};

mod api;
mod analysis;
mod data_collector;
mod database;
mod models;

use crate::analysis::AnalysisEngine;
use crate::api::SchwabClient;
use crate::database::DatabaseManager;
use crate::models::Config;

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
    let database = DatabaseManager::new(&config.database_path)?;
    println!("✅ Database initialized at: {}", config.database_path);

    // Initialize analysis engine
    let analysis_engine = AnalysisEngine::new(database);
    
    // Get database statistics
    let stats = analysis_engine.get_summary_stats().await?;
    
    println!("\n📊 Current Database State:");
    println!("   Total Stocks: {}", stats.total_stocks);
    println!("   Total Price Records: {}", stats.total_price_records);
    
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