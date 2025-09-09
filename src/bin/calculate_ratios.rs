use clap::{Arg, Command};
use sqlx::sqlite::SqlitePoolOptions;
use std::time::Instant;
use anyhow::Result;

// Import the ratio calculator module
use rust_stocks::tools::ratio_calculator::{
    calculate_ps_and_evs_ratios,
    calculate_ratios_for_negative_earnings_stocks,
    generate_ratio_summary_report,
};

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("P/S and EV/S Ratio Calculator")
        .version("1.0")
        .author("Multi-Period Valuation System")
        .about("Calculate P/S and EV/S ratios using TTM financial data")
        .arg(Arg::new("database")
            .long("db")
            .value_name("FILE")
            .help("Path to SQLite database (PRODUCTION: ./stocks.db in ROOT)")
            .default_value("./stocks.db"))
        .arg(Arg::new("negative_earnings_only")
            .long("negative-earnings")
            .help("Focus only on stocks with negative earnings where P/E is invalid")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("report_only")
            .long("report")
            .help("Generate summary report of existing ratios without recalculating")
            .action(clap::ArgAction::SetTrue))
        .get_matches();

    let db_path = matches.get_one::<String>("database").unwrap();
    let negative_earnings_only = matches.get_flag("negative_earnings_only");
    let report_only = matches.get_flag("report_only");

    println!("🧮 P/S AND EV/S RATIO CALCULATOR");
    println!("💾 Database: {}", db_path);
    
    if negative_earnings_only {
        println!("🎯 Focus: Stocks with negative earnings (P/E invalid)");
    } else if report_only {
        println!("📊 Mode: Report only (no calculations)");
    } else {
        println!("📊 Mode: Calculate all available P/S and EV/S ratios");
    }
    
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

    // Check TTM data availability
    let ttm_income_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM income_statements WHERE period_type = 'TTM'"
    ).fetch_one(&pool).await.unwrap_or(0);

    if ttm_income_count == 0 {
        eprintln!("❌ No TTM financial data found!");
        eprintln!("💡 Please run TTM import first: cargo run --bin import-ttm");
        return Ok(());
    }

    println!("📊 Database contains {} stocks and {} TTM financial records", 
        stock_count, ttm_income_count);

    if report_only {
        // Generate report only
        println!("\n📋 Generating ratio summary report...");
        generate_ratio_summary_report(&pool).await?;
    } else if negative_earnings_only {
        // Calculate ratios for negative earnings stocks only
        println!("\n🔍 Calculating ratios for stocks with negative earnings...");
        match calculate_ratios_for_negative_earnings_stocks(&pool).await {
            Ok(stats) => {
                let duration = start_time.elapsed();
                print_calculation_summary(&stats, duration);
                
                // Generate report
                println!("\n📋 Generating summary report...");
                generate_ratio_summary_report(&pool).await?;
            }
            Err(e) => {
                eprintln!("❌ Calculation failed: {}", e);
                return Err(e);
            }
        }
    } else {
        // Calculate all P/S and EV/S ratios
        println!("\n🧮 Calculating P/S and EV/S ratios for all stocks...");
        match calculate_ps_and_evs_ratios(&pool).await {
            Ok(stats) => {
                let duration = start_time.elapsed();
                print_calculation_summary(&stats, duration);
                
                // Generate report
                println!("\n📋 Generating summary report...");
                generate_ratio_summary_report(&pool).await?;
            }
            Err(e) => {
                eprintln!("❌ Calculation failed: {}", e);
                return Err(e);
            }
        }
    }

    pool.close().await;
    Ok(())
}

/// Print calculation summary
fn print_calculation_summary(stats: &rust_stocks::tools::ratio_calculator::RatioCalculationStats, duration: std::time::Duration) {
    println!("\n{}", "=".repeat(60));
    println!("🎉 RATIO CALCULATIONS COMPLETE");
    println!("{}", "=".repeat(60));
    println!("⏱️  Total Duration: {:?}", duration);
    println!("📊 Stocks Processed: {}", stats.stocks_processed);
    println!("💰 P/S Ratios Calculated: {}", stats.ps_ratios_calculated);
    println!("🏢 EV/S Ratios Calculated: {}", stats.evs_ratios_calculated);
    println!("📈 Market Caps Calculated: {}", stats.market_caps_calculated);
    println!("🏦 Enterprise Values Calculated: {}", stats.enterprise_values_calculated);
    println!("❌ Errors: {}", stats.errors);
    println!("{}", "=".repeat(60));
    
    if stats.errors > 0 {
        println!("⚠️  Calculations completed with {} errors", stats.errors);
    } else {
        println!("✅ All calculations completed successfully!");
        println!("🚀 P/S and EV/S ratios ready for stock analysis");
    }
}