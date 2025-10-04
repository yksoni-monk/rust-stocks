use clap::{Arg, Command};
use sqlx::sqlite::SqlitePoolOptions;
use std::time::Instant;
use anyhow::Result;

// Import the SEC EDGAR client
use rust_stocks_tauri_lib::tools::sec_edgar_client::{SecEdgarClient, test_sec_edgar_client};

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("SEC EDGAR Balance Sheet Downloader")
        .version("1.0")
        .author("SEC EDGAR Integration")
        .about("Download balance sheet data from SEC EDGAR 10-K filings")
        .arg(Arg::new("database")
            .long("db")
            .value_name("FILE")
            .help("Path to SQLite database")
            .default_value("db/stocks.db"))
        .arg(Arg::new("test")
            .long("test")
            .help("Test the SEC EDGAR client with a few companies")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("limit")
            .long("limit")
            .value_name("NUMBER")
            .help("Limit to first N companies (for testing)")
            .value_parser(clap::value_parser!(usize)))
        .get_matches();

    let db_path = matches.get_one::<String>("database").unwrap();
    let test_mode = matches.get_flag("test");
    let limit = matches.get_one::<usize>("limit").copied();

    println!("🏛️ SEC EDGAR BALANCE SHEET DOWNLOADER");
    println!("💾 Database: {}", db_path);
    
    if test_mode {
        println!("🧪 Mode: Test mode (few companies only)");
    } else if let Some(limit) = limit {
        println!("📊 Mode: Limited to first {} companies", limit);
    } else {
        println!("📊 Mode: Full S&P 500 download");
    }
    
    println!("{}", "=".repeat(60));

    let start_time = Instant::now();

    // Connect to database
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&format!("sqlite:{}?mode=rwc", db_path))
        .await?;
        
    println!("✅ Connected to database: {}", db_path);

    // Verify database structure
    let stock_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM stocks")
        .fetch_one(&pool)
        .await?;

    if stock_count == 0 {
        eprintln!("❌ No stocks found in database!");
        return Ok(());
    }

    // Check CIK mappings
    let cik_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM stocks WHERE is_sp500 = 1 AND cik IS NOT NULL")
        .fetch_one(&pool)
        .await?;

    if cik_count == 0 {
        eprintln!("❌ No CIK mappings found!");
        eprintln!("💡 Please run CIK mapping import first");
        return Ok(());
    }

    println!("📊 Database contains {} stocks and {} CIK mappings", stock_count, cik_count);

    if test_mode {
        // Test mode - just test the client
        test_sec_edgar_client(&pool).await?;
    } else {
        // Full download mode
        let mut client = SecEdgarClient::new(pool.clone());
        
        if let Some(limit) = limit {
            // Limited download for testing
            println!("\n🔍 Downloading balance sheet data for first {} companies...", limit);
            // TODO: Implement limited download
            client.download_all_sp500_balance_sheets().await?;
        } else {
            // Full S&P 500 download
            println!("\n🚀 Downloading balance sheet data for all S&P 500 companies...");
            client.download_all_sp500_balance_sheets().await?;
        }
    }

    let duration = start_time.elapsed();
    println!("\n{}", "=".repeat(60));
    println!("🎉 SEC EDGAR DOWNLOAD COMPLETE");
    println!("{}", "=".repeat(60));
    println!("⏱️  Total Duration: {:?}", duration);
    println!("✅ Balance sheet data download completed successfully!");
    println!("💡 Next: Run P/B ratio calculation to see improved coverage");

    pool.close().await;
    Ok(())
}
