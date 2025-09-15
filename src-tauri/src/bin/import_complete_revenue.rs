use clap::{Arg, Command};
use rust_stocks_tauri_lib::tools::ttm_importer::import_complete_revenue_dataset;
use sqlx::SqlitePool;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("Complete Revenue Data Import")
        .version("1.0")
        .author("Stock Analysis System")
        .about("Import all Annual, Quarterly, and TTM revenue and balance sheet data from SimFin")
        .arg(Arg::new("database")
            .long("db")
            .value_name("FILE")
            .help("Path to SQLite database")
            .default_value("db/stocks.db"))
        .arg(Arg::new("simfin-data-dir")
            .long("simfin-dir")
            .value_name("DIRECTORY")
            .help("Path to SimFin data directory")
            .default_value("../simfin_data"))
        .get_matches();

    let db_path = matches.get_one::<String>("database").unwrap();
    let simfin_dir = matches.get_one::<String>("simfin-data-dir").unwrap();

    println!("🚀 Complete Revenue Data Import Tool");
    println!("  📁 Database: {}", db_path);
    println!("  📁 SimFin Data: {}", simfin_dir);

    // Verify database exists
    if !Path::new(db_path).exists() {
        return Err(format!("Database file not found: {}", db_path).into());
    }

    // Verify SimFin data directory exists
    if !Path::new(simfin_dir).exists() {
        return Err(format!("SimFin data directory not found: {}", simfin_dir).into());
    }

    // Connect to database
    let pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path)).await?;
    println!("✅ Connected to database");

    // Define file paths
    let annual_income_path = format!("{}/us-income-annual.csv", simfin_dir);
    let quarterly_income_path = format!("{}/us-income-quarterly.csv", simfin_dir);
    let annual_balance_path = format!("{}/us-balance-annual.csv", simfin_dir);
    let quarterly_balance_path = format!("{}/us-balance-quarterly.csv", simfin_dir);
    let ttm_balance_path = format!("{}/us-balance-ttm.csv", simfin_dir);

    // Verify all required files exist
    let required_files = vec![
        &annual_income_path,
        &quarterly_income_path,
        &annual_balance_path,
        &quarterly_balance_path,
        &ttm_balance_path,
    ];

    for file_path in required_files {
        if !Path::new(file_path).exists() {
            return Err(format!("Required file not found: {}", file_path).into());
        }
    }

    println!("✅ All required SimFin files found");

    // Run the complete import
    let stats = import_complete_revenue_dataset(
        &pool,
        &annual_income_path,
        &quarterly_income_path,
        &annual_balance_path,
        &quarterly_balance_path,
        &ttm_balance_path,
    ).await?;

    // Display final statistics
    println!("\n📊 Import Summary:");
    println!("  📈 Annual Revenue Records: {}", stats.annual_revenue_imported);
    println!("  📈 Quarterly Revenue Records: {}", stats.quarterly_revenue_imported);
    println!("  🏦 Annual Balance Records: {}", stats.annual_balance_imported);
    println!("  🏦 Quarterly Balance Records: {}", stats.quarterly_balance_imported);
    println!("  🏦 TTM Balance Records: {}", stats.ttm_balance_imported);
    println!("  ❌ Errors: {}", stats.errors);

    if stats.errors == 0 {
        println!("\n🎉 Complete revenue dataset import successful!");
        println!("  📈 Ready for enhanced P/S screening with full revenue data");
        println!("  🔍 Run database validation to verify data integrity");
    } else {
        println!("\n⚠️  Import completed with {} errors", stats.errors);
        println!("  🔍 Check logs above for specific error details");
    }

    pool.close().await;
    Ok(())
}
