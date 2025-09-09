use clap::{Arg, Command};
use sqlx::sqlite::SqlitePoolOptions;
use std::time::Instant;
use anyhow::Result;

// Import the simfin_importer module
use rust_stocks_tauri_lib::tools::simfin_importer::{
    import_stocks_from_daily_prices,
    import_daily_prices,
    import_quarterly_financials,
    calculate_and_store_eps,
    calculate_and_store_pe_ratios,
    add_performance_indexes,
    ImportStats,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("SimFin Data Importer")
        .version("1.0")
        .author("Stock Analysis System")
        .about("Import SimFin CSV data into SQLite database")
        .arg(Arg::new("prices")
            .long("prices")
            .value_name("FILE")
            .help("Path to us-shareprices-daily.csv")
            .required(true))
        .arg(Arg::new("income")
            .long("income") 
            .value_name("FILE")
            .help("Path to us-income-quarterly.csv")
            .required(true))
        .arg(Arg::new("database")
            .long("db")
            .value_name("FILE")
            .help("Path to SQLite database")
            .default_value("db/stocks.db"))
        .arg(Arg::new("batch_size")
            .long("batch-size")
            .value_name("SIZE")
            .help("Batch size for price imports")
            .default_value("10000"))
        .get_matches();

    let prices_path = matches.get_one::<String>("prices").unwrap();
    let income_path = matches.get_one::<String>("income").unwrap();
    let db_path = matches.get_one::<String>("database").unwrap();
    let batch_size: usize = matches.get_one::<String>("batch_size").unwrap().parse()?;

    println!("🚀 SimFin Data Import Started");
    println!("📊 Daily Prices: {}", prices_path);
    println!("🏢 Quarterly Income: {}", income_path);
    println!("💾 Database: {}", db_path);
    println!("📦 Batch Size: {}", batch_size);
    println!("{}", "=".repeat(60));

    let start_time = Instant::now();
    let mut stats = ImportStats::default();

    // Connect to database
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&format!("sqlite:{}?mode=rwc", db_path))
        .await?;

    println!("✅ Connected to database: {}", db_path);

    // Phase 1: Import stocks from daily prices CSV
    println!("\n📋 PHASE 1: Stock Import");
    match import_stocks_from_daily_prices(&pool, prices_path).await {
        Ok(count) => {
            stats.stocks_imported = count;
            println!("✅ Phase 1 Complete: {} stocks imported", count);
        }
        Err(e) => {
            eprintln!("❌ Phase 1 Failed: {}", e);
            stats.errors += 1;
        }
    }

    // Phase 2: Import daily prices
    println!("\n📈 PHASE 2: Daily Price Import");
    match import_daily_prices(&pool, prices_path, batch_size).await {
        Ok(count) => {
            stats.prices_imported = count;
            println!("✅ Phase 2 Complete: {} prices imported", count);
        }
        Err(e) => {
            eprintln!("❌ Phase 2 Failed: {}", e);
            stats.errors += 1;
        }
    }

    // Phase 3: Import quarterly financials
    println!("\n🏢 PHASE 3: Quarterly Financials Import");
    match import_quarterly_financials(&pool, income_path).await {
        Ok(count) => {
            stats.financials_imported = count;
            println!("✅ Phase 3 Complete: {} financial records imported", count);
        }
        Err(e) => {
            eprintln!("❌ Phase 3 Failed: {}", e);
            stats.errors += 1;
        }
    }

    // Phase 4: Calculate EPS
    println!("\n🧮 PHASE 4: EPS Calculation");
    match calculate_and_store_eps(&pool).await {
        Ok(count) => {
            stats.eps_calculated = count;
            println!("✅ Phase 4 Complete: {} EPS values calculated", count);
        }
        Err(e) => {
            eprintln!("❌ Phase 4 Failed: {}", e);
            stats.errors += 1;
        }
    }

    // Phase 5: Calculate P/E ratios
    println!("\n📊 PHASE 5: P/E Ratio Calculation");
    match calculate_and_store_pe_ratios(&pool).await {
        Ok(count) => {
            stats.pe_ratios_calculated = count;
            println!("✅ Phase 5 Complete: {} P/E ratios calculated", count);
        }
        Err(e) => {
            eprintln!("❌ Phase 5 Failed: {}", e);
            stats.errors += 1;
        }
    }

    // Phase 6: Add performance indexes
    println!("\n⚡ PHASE 6: Performance Indexes");
    match add_performance_indexes(&pool).await {
        Ok(_) => {
            println!("✅ Phase 6 Complete: Performance indexes created");
        }
        Err(e) => {
            eprintln!("❌ Phase 6 Failed: {}", e);
            stats.errors += 1;
        }
    }

    let total_duration = start_time.elapsed();

    // Final summary
    println!("\n{}", "=".repeat(60));
    println!("🎉 SIMFIN IMPORT COMPLETE");
    println!("{}", "=".repeat(60));
    println!("⏱️  Total Duration: {:?}", total_duration);
    println!("📊 Stocks Imported: {}", stats.stocks_imported);
    println!("📈 Prices Imported: {}", stats.prices_imported);
    println!("🏢 Financials Imported: {}", stats.financials_imported);
    println!("🧮 EPS Calculated: {}", stats.eps_calculated);
    println!("📊 P/E Ratios Calculated: {}", stats.pe_ratios_calculated);
    println!("❌ Errors: {}", stats.errors);
    println!("{}", "=".repeat(60));

    if stats.errors > 0 {
        println!("⚠️  Import completed with {} errors. Check logs above.", stats.errors);
    } else {
        println!("✅ All phases completed successfully!");
    }

    // Database size check
    match std::fs::metadata(db_path) {
        Ok(metadata) => {
            let size_mb = metadata.len() as f64 / 1024.0 / 1024.0;
            println!("💾 Database Size: {:.2} MB", size_mb);
        }
        Err(_) => {}
    }

    pool.close().await;
    Ok(())
}