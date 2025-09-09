use clap::{Arg, Command};
use sqlx::sqlite::SqlitePoolOptions;
use std::time::Instant;
use anyhow::Result;

// Import the TTM importer module
use rust_stocks_tauri_lib::tools::ttm_importer::{
    import_complete_ttm_dataset,
    TTMImportStats,
};

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("TTM Financial Data Importer")
        .version("2.0")
        .author("Multi-Period Valuation System")
        .about("Import SimFin TTM (Trailing Twelve Months) financial data for P/S and EV/S ratio calculations")
        .arg(Arg::new("income")
            .long("income")
            .value_name("FILE")
            .help("Path to us-income-ttm.csv")
            .required(false))
        .arg(Arg::new("balance")
            .long("balance") 
            .value_name("FILE")
            .help("Path to us-balance-ttm.csv")
            .required(false))
        .arg(Arg::new("database")
            .long("db")
            .value_name("FILE")
            .help("Path to SQLite database (PRODUCTION: db/stocks.db in src-tauri)")
            .default_value("db/stocks.db"))
        .arg(Arg::new("data_dir")
            .long("data-dir")
            .value_name("DIR")
            .help("Directory containing SimFin CSV files")
            .default_value("/Users/yksoni/simfin_data"))
        .get_matches();

    let db_path = matches.get_one::<String>("database").unwrap();
    let data_dir = matches.get_one::<String>("data_dir").unwrap();

    // Use data directory by default
    let income_path = if let Some(income) = matches.get_one::<String>("income") {
        income.clone()
    } else {
        format!("{}/us-income-ttm.csv", data_dir)
    };
    
    let balance_path = if let Some(balance) = matches.get_one::<String>("balance") {
        balance.clone() 
    } else {
        format!("{}/us-balance-ttm.csv", data_dir)
    };

    println!("🚀 TTM Financial Data Import Started");
    println!("💰 TTM Income: {}", income_path);
    println!("🏦 TTM Balance: {}", balance_path);
    println!("💾 Database: {}", db_path);
    println!("📊 Purpose: Multi-period P/S and EV/S ratio calculations");
    println!("{}", "=".repeat(70));

    let start_time = Instant::now();

    // Direct database connection to production database
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&format!("sqlite:{}?mode=rwc", db_path))
        .await?;
        
    println!("✅ Connected to production database: {}", db_path);

    // Check if stocks exist in the database
    let stock_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM stocks")
        .fetch_one(&pool)
        .await?;

    if stock_count == 0 {
        eprintln!("❌ No stocks found in database!");
        eprintln!("💡 Please run the basic SimFin importer first to populate stocks table");
        return Ok(());
    }

    println!("📊 Found {} stocks in database - proceeding with TTM import", stock_count);

    // Verify TTM files exist
    if !std::path::Path::new(&income_path).exists() {
        return Err(anyhow::anyhow!("TTM income file not found: {}", income_path));
    }
    if !std::path::Path::new(&balance_path).exists() {
        return Err(anyhow::anyhow!("TTM balance file not found: {}", balance_path));
    }

    // Import complete TTM dataset
    println!("\n🔄 STARTING TTM FINANCIAL DATA IMPORT");
    match import_complete_ttm_dataset(&pool, &income_path, &balance_path).await {
        Ok(stats) => {
            let duration = start_time.elapsed();
            print_success_summary(&stats, duration);
        }
        Err(e) => {
            eprintln!("❌ TTM Import Failed: {}", e);
            return Err(e);
        }
    }

    // Database statistics
    let final_stats = get_database_statistics(&pool).await?;
    print_database_statistics(&final_stats, db_path);

    pool.close().await;
    Ok(())
}

/// Print success summary
fn print_success_summary(stats: &TTMImportStats, duration: std::time::Duration) {
    println!("\n{}", "=".repeat(70));
    println!("🎉 TTM FINANCIAL DATA IMPORT COMPLETE");
    println!("{}", "=".repeat(70));
    println!("⏱️  Total Duration: {:?}", duration);
    println!("💰 Income Statements Imported: {}", stats.income_statements_imported);
    println!("🏦 Balance Sheets Imported: {}", stats.balance_sheets_imported);
    println!("📊 Total Financial Records: {}", stats.income_statements_imported + stats.balance_sheets_imported);
    println!("❌ Errors: {}", stats.errors);
    println!("{}", "=".repeat(70));
    
    if stats.errors > 0 {
        println!("⚠️  Import completed with {} errors. Check logs above.", stats.errors);
    } else {
        println!("✅ All TTM data imported successfully!");
        println!("🚀 Ready for P/S and EV/S ratio calculations");
    }
}

/// Database statistics structure
struct DatabaseStats {
    total_stocks: i64,
    total_income_statements: i64,
    total_balance_sheets: i64,
    ttm_income_statements: i64,
    ttm_balance_sheets: i64,
}

/// Get comprehensive database statistics
async fn get_database_statistics(pool: &sqlx::SqlitePool) -> Result<DatabaseStats> {
    let total_stocks = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM stocks")
        .fetch_one(pool).await.unwrap_or(0);
        
    let total_income_statements = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM income_statements")
        .fetch_one(pool).await.unwrap_or(0);
        
    let total_balance_sheets = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM balance_sheets")
        .fetch_one(pool).await.unwrap_or(0);
        
    let ttm_income_statements = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM income_statements WHERE period_type = 'TTM'"
    ).fetch_one(pool).await.unwrap_or(0);
    
    let ttm_balance_sheets = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM balance_sheets WHERE period_type = 'TTM'"
    ).fetch_one(pool).await.unwrap_or(0);

    Ok(DatabaseStats {
        total_stocks,
        total_income_statements,
        total_balance_sheets,
        ttm_income_statements,
        ttm_balance_sheets,
    })
}

/// Print comprehensive database statistics
fn print_database_statistics(stats: &DatabaseStats, db_path: &str) {
    println!("\n📈 DATABASE STATISTICS");
    println!("{}", "-".repeat(40));
    println!("🏢 Total Stocks: {}", stats.total_stocks);
    println!("💰 Total Income Statements: {} (TTM: {})", stats.total_income_statements, stats.ttm_income_statements);
    println!("🏦 Total Balance Sheets: {} (TTM: {})", stats.total_balance_sheets, stats.ttm_balance_sheets);
    
    // Database size
    match std::fs::metadata(db_path) {
        Ok(metadata) => {
            let size_mb = metadata.len() as f64 / 1024.0 / 1024.0;
            println!("💾 Database Size: {:.2} MB", size_mb);
        }
        Err(_) => {
            println!("💾 Database Size: Unable to determine");
        }
    }

    println!("{}", "-".repeat(40));
    
    if stats.ttm_income_statements > 0 && stats.ttm_balance_sheets > 0 {
        println!("🎯 TTM Data Coverage: Ready for multi-period ratio calculations");
        println!("📊 Next Step: Calculate P/S and EV/S ratios using TTM financial data");
        println!("💡 Ratios will be calculated for stocks with complete TTM financial data");
    } else {
        println!("⚠️  Incomplete TTM data - ratio calculations may be limited");
    }
}