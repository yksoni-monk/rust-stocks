use clap::{Arg, Command};
use rust_stocks_tauri_lib::database::{DatabaseManager, initialize_database_safely, run_manual_migration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("Database Administration Tool")
        .version("1.0")
        .author("Stock Analysis System")
        .about("Safe database backup and migration operations")
        .arg(Arg::new("database")
            .long("db")
            .value_name("FILE")
            .help("Path to SQLite database")
            .default_value("db/stocks.db"))
        .subcommand(
            Command::new("backup")
                .about("Create database backup")
        )
        .subcommand(
            Command::new("status")
                .about("Show database status and statistics")
        )
        .subcommand(
            Command::new("migrate")
                .about("Run database migrations (requires explicit confirmation)")
                .arg(Arg::new("confirm")
                    .long("confirm")
                    .help("Explicitly confirm migration on production database")
                    .action(clap::ArgAction::SetTrue))
        )
        .subcommand(
            Command::new("verify")
                .about("Verify database integrity")
        )
        .get_matches();

    let db_path = matches.get_one::<String>("database").unwrap();

    match matches.subcommand() {
        Some(("backup", _)) => {
            println!("📦 Creating database backup...");
            let backup_path = DatabaseManager::create_backup(db_path).await?;
            println!("✅ Backup created: {}", backup_path);
        }
        
        Some(("status", _)) => {
            let db_manager = DatabaseManager::new(db_path).await?;
            let stats = db_manager.get_database_stats().await?;
            
            println!("📊 Database Status: {}", db_path);
            println!("   📈 Stocks: {}", stats.total_stocks);
            println!("   💹 Price records: {}", stats.total_prices);
            println!("   🏢 Financial records: {}", stats.total_financials);
            println!("   💾 Size: {:.2} MB", stats.database_size_mb);
            
            if stats.total_stocks > 1000 {
                println!("   🚨 PRODUCTION DATABASE - Extra safeguards active");
            }
        }
        
        Some(("migrate", sub_matches)) => {
            let confirm = sub_matches.get_flag("confirm");
            
            if !confirm {
                println!("⚠️  Migration requires explicit confirmation for safety:");
                println!("   cargo run --bin db_admin -- --db {} migrate --confirm", db_path);
                return Ok(());
            }
            
            println!("🔧 Running database migration...");
            run_manual_migration(db_path, confirm).await?;
            println!("✅ Migration completed");
        }
        
        Some(("verify", _)) => {
            println!("🔍 Verifying database integrity...");
            let db_manager = DatabaseManager::new(db_path).await?;
            let stats = db_manager.verify_data_safety().await?;
            
            println!("✅ Database verification completed");
            println!("   Data integrity: OK");
            println!("   Total records: {} stocks, {} prices", stats.total_stocks, stats.total_prices);
        }
        
        _ => {
            println!("📋 Available commands:");
            println!("   backup   - Create database backup");
            println!("   status   - Show database statistics");
            println!("   migrate  - Run migrations (with --confirm)");
            println!("   verify   - Verify database integrity");
            println!("\nExamples:");
            println!("   cargo run --bin db_admin -- backup");
            println!("   cargo run --bin db_admin -- status");
            println!("   cargo run --bin db_admin -- migrate --confirm");
        }
    }

    Ok(())
}