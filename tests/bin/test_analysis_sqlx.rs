use anyhow::Result;
use tracing::{error, Level};
use tracing_subscriber::{self, FmtSubscriber};

use rust_stocks::database_sqlx::DatabaseManagerSqlx;
use rust_stocks::analysis::AnalysisEngine;
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

    println!("🚀 Analysis Module Test - Database initialized successfully!");
    println!("📊 Testing analysis functionality with SQLX...");

    // Test analysis functionality
    match test_analysis_functionality(database).await {
        Ok(_) => {
            println!("✅ Analysis module test completed successfully!");
            println!("🎉 Phase 2 analysis module is working!");
        }
        Err(e) => {
            eprintln!("❌ Analysis test failed: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

async fn test_analysis_functionality(database: DatabaseManagerSqlx) -> Result<()> {
    let analysis_engine = AnalysisEngine::new(database);

    // Test 1: Search stocks
    println!("🔍 Testing stock search...");
    let search_results = analysis_engine.search_stocks("AAPL").await?;
    println!("📈 Search results for 'AAPL': {} stocks found", search_results.len());
    
    if !search_results.is_empty() {
        println!("📊 First result: {} - {}", search_results[0].symbol, search_results[0].company_name);
    }

    // Test 2: Get summary stats
    println!("📊 Testing summary statistics...");
    let stats = analysis_engine.get_database_stats().await?;
    println!("📈 Summary stats: {} stocks, {} price records", stats.total_stocks, stats.total_price_records);

    // Test 3: Get top P/E decliners (limit to 5 for performance)
    println!("📉 Testing P/E decliners analysis...");
    let decliners = analysis_engine.get_top_pe_decliners(5, 0).await?;
    println!("📊 Found {} stocks with P/E decline", decliners.len());
    
    for (i, analysis) in decliners.iter().enumerate().take(3) {
        println!("  {}. {}: {:.2}% P/E decline", i + 1, analysis.stock.symbol, analysis.pe_decline_percent);
    }

    // Test 4: Get stock details for a specific stock
    if !search_results.is_empty() {
        println!("📋 Testing stock details...");
        let symbol = &search_results[0].symbol;
        let details = analysis_engine.get_stock_details(symbol).await?;
        
        match details {
            Some(detail) => {
                println!("📊 Stock details for {}: {} price records", symbol, detail.price_history.len());
                println!("📈 P/E trend: {} data points", detail.pe_trend.len());
                println!("📊 Volume trend: {} data points", detail.volume_trend.len());
            }
            None => {
                println!("⚠️ No details found for {}", symbol);
            }
        }
    }

    Ok(())
}
