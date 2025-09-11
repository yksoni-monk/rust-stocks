use clap::{Arg, Command};
use sqlx::sqlite::SqlitePoolOptions;
use std::time::Instant;
use anyhow::Result;

// Import the simfin importer module
use rust_stocks_tauri_lib::tools::simfin_importer::update_shares_outstanding_from_income_statements;

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("Fix Shares Outstanding Data")
        .version("1.0")
        .author("Stock Analysis System")
        .about("Update shares outstanding data in daily_prices table using income statements data")
        .arg(Arg::new("database")
            .long("db")
            .value_name("FILE")
            .help("Path to SQLite database (PRODUCTION: db/stocks.db in src-tauri)")
            .default_value("db/stocks.db"))
        .get_matches();

    let db_path = matches.get_one::<String>("database").unwrap();

    println!("🔧 FIXING SHARES OUTSTANDING DATA");
    println!("💾 Database: {}", db_path);
    println!("{}", "=".repeat(60));

    let start_time = Instant::now();

    // Connect to production database
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&format!("sqlite:{}?mode=rwc", db_path))
        .await?;
        
    println!("✅ Connected to production database: {}", db_path);

    // Verify database structure
    let stock_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM stocks")
        .fetch_one(&pool)
        .await?;

    if stock_count == 0 {
        eprintln!("❌ No stocks found in database!");
        return Ok(());
    }

    // Check quarterly financials data availability
    let quarterly_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM quarterly_financials WHERE shares_diluted IS NOT NULL AND shares_diluted > 0"
    ).fetch_one(&pool).await.unwrap_or(0);

    if quarterly_count == 0 {
        eprintln!("❌ No shares outstanding data found in quarterly_financials!");
        eprintln!("💡 Please import quarterly financials first");
        return Ok(());
    }

    println!("📊 Database contains {} stocks and {} quarterly financial records with shares data", 
        stock_count, quarterly_count);

    // Update shares outstanding data
    println!("\n🔄 Updating shares outstanding from income statements...");
    match update_shares_outstanding_from_income_statements(&pool).await {
        Ok(updated_count) => {
            let duration = start_time.elapsed();
            println!("\n{}", "=".repeat(60));
            println!("🎉 SHARES OUTSTANDING UPDATE COMPLETE");
            println!("{}", "=".repeat(60));
            println!("⏱️  Total Duration: {:?}", duration);
            println!("📊 Daily Price Records Updated: {}", updated_count);
            println!("✅ Shares outstanding data fixed!");
            println!("💡 Next step: Run ratio calculator to recalculate P/S and EV/S ratios");
        }
        Err(e) => {
            eprintln!("❌ Update failed: {}", e);
            return Err(e);
        }
    }

    pool.close().await;
    Ok(())
}
