mod api;
mod analysis;
mod concurrent_fetcher;
mod data_collector;
mod models;
mod database_sqlx;
mod ui;
mod utils;

use anyhow::Result;
use tracing::{error, Level};
use tracing_subscriber::{self, FmtSubscriber};

use crate::database_sqlx::DatabaseManagerSqlx;
use crate::models::Config;
use crate::ui::app::run_app_async;

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

    println!("🚀 Stock Analysis System - Database initialized successfully!");
    println!("📊 Starting TUI application...");

    // Run the TUI application
    match run_app_async(config, database).await {
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
