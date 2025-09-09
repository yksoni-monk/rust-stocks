use sqlx::sqlite::SqlitePoolOptions;
use rust_stocks_tauri_lib::tools::{
    ttm_importer::import_complete_ttm_dataset,
    ratio_calculator::{calculate_ps_and_evs_ratios, generate_ratio_summary_report}
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to production database
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite:../stocks.db?mode=rwc")
        .await?;

    println!("🚀 P/S AND EV/S RATIO CALCULATION TEST");
    println!("📊 Using production database with 2.2GB of data");

    // Check current data
    let stock_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM stocks")
        .fetch_one(&pool)
        .await?;
    let price_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM daily_prices")
        .fetch_one(&pool)
        .await?;

    println!("📈 Production Database Stats:");
    println!("  🏢 Stocks: {}", stock_count);
    println!("  📊 Daily Prices: {}", price_count);

    // Check if TTM tables exist
    let table_check = sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name IN ('income_statements', 'balance_sheets', 'daily_valuation_ratios')")
        .fetch_all(&pool)
        .await?;
    
    if table_check.len() < 3 {
        println!("⚠️  TTM tables missing - need to run migration first");
        println!("💡 Run: sqlx migrate run --database-url sqlite:../stocks.db");
        return Ok(());
    }

    // Check TTM data
    let ttm_income_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM income_statements WHERE period_type = 'TTM'"
    ).fetch_one(&pool).await.unwrap_or(0);

    let ttm_balance_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM balance_sheets WHERE period_type = 'TTM'"  
    ).fetch_one(&pool).await.unwrap_or(0);

    println!("💰 TTM Financial Data:");
    println!("  📊 Income Statements: {}", ttm_income_count);
    println!("  🏦 Balance Sheets: {}", ttm_balance_count);

    if ttm_income_count == 0 {
        println!("\n🔄 Importing TTM financial data...");
        let income_path = "/Users/yksoni/simfin_data/us-income-ttm.csv";
        let balance_path = "/Users/yksoni/simfin_data/us-balance-ttm.csv";
        
        match import_complete_ttm_dataset(&pool, income_path, balance_path).await {
            Ok(stats) => {
                println!("✅ TTM import completed:");
                println!("  💰 Income statements: {}", stats.income_statements_imported);
                println!("  🏦 Balance sheets: {}", stats.balance_sheets_imported);
            }
            Err(e) => {
                println!("❌ TTM import failed: {}", e);
                return Ok(());
            }
        }
    }

    // Calculate P/S and EV/S ratios
    println!("\n🧮 Calculating P/S and EV/S ratios...");
    match calculate_ps_and_evs_ratios(&pool).await {
        Ok(stats) => {
            println!("✅ Ratio calculation completed:");
            println!("  📊 Stocks processed: {}", stats.stocks_processed);
            println!("  💰 P/S ratios calculated: {}", stats.ps_ratios_calculated);
            println!("  🏢 EV/S ratios calculated: {}", stats.evs_ratios_calculated);
            println!("  📈 Market caps calculated: {}", stats.market_caps_calculated);
            println!("  🏦 Enterprise values calculated: {}", stats.enterprise_values_calculated);
            println!("  ❌ Errors: {}", stats.errors);
        }
        Err(e) => {
            println!("❌ Ratio calculation failed: {}", e);
            return Ok(());
        }
    }

    // Generate summary report
    println!("\n📋 Generating ratio summary report...");
    generate_ratio_summary_report(&pool).await?;

    pool.close().await;
    Ok(())
}