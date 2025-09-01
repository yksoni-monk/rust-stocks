mod api;
mod analysis;
mod data_collector;
mod database;
mod models;
mod ui;

use anyhow::Result;
use tracing::{error, Level};
use tracing_subscriber::{self, FmtSubscriber};

use crate::database::DatabaseManager;
use crate::models::Config;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging - suppress most logs for TUI
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::ERROR)
        .with_env_filter("rust_stocks=error")
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

    // Initialize database
    let database = match DatabaseManager::new(&config.database_path) {
        Ok(db) => db,
        Err(e) => {
            error!("Failed to initialize database: {}", e);
            eprintln!("❌ Database Error: {}", e);
            std::process::exit(1);
        }
    };

    // Start TUI with async support
    println!("🚀 Starting Stock Analysis TUI...");
    
    match ui::app::run_app_async(config, database).await {
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
