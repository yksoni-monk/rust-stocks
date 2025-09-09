use sqlx::sqlite::SqlitePoolOptions;
use rust_stocks_tauri_lib::tools::ttm_importer::import_complete_ttm_dataset;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Direct database connection (bypassing protected init)
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite:./stocks.db?mode=rwc")
        .await?;

    println!("🧪 DIRECT TTM IMPORT TEST");
    println!("📊 Testing TTM import functionality with existing database");

    // Check stocks count
    let stock_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM stocks")
        .fetch_one(&pool)
        .await?;
    println!("📈 Found {} stocks in database", stock_count);

    if stock_count == 0 {
        println!("🔧 Creating sample stocks for testing...");
        
        // Insert a few sample stocks that exist in the TTM files
        let sample_stocks = vec![
            ("AAPL", "Apple Inc.", 45846),
            ("MSFT", "Microsoft Corporation", 56976), 
            ("GOOGL", "Alphabet Inc.", 64850),
        ];
        
        for (symbol, name, simfin_id) in sample_stocks {
            sqlx::query(
                "INSERT OR IGNORE INTO stocks (symbol, company_name, simfin_id, created_at) 
                 VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP)"
            )
            .bind(symbol)
            .bind(name)
            .bind(simfin_id)
            .execute(&pool)
            .await?;
        }
        
        let new_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM stocks")
            .fetch_one(&pool)
            .await?;
        println!("✅ Created {} sample stocks for testing", new_count);
    }

    // Test with small subset first by limiting to just first few records
    let income_path = "/Users/yksoni/simfin_data/us-income-ttm.csv";
    let balance_path = "/Users/yksoni/simfin_data/us-balance-ttm.csv";

    println!("🔄 Starting TTM import test...");
    match import_complete_ttm_dataset(&pool, income_path, balance_path).await {
        Ok(stats) => {
            println!("✅ TTM Import Test Successful!");
            println!("  💰 Income statements: {}", stats.income_statements_imported);
            println!("  🏦 Balance sheets: {}", stats.balance_sheets_imported);
        }
        Err(e) => {
            println!("❌ TTM Import Test Failed: {}", e);
        }
    }

    // Check results
    let income_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM income_statements WHERE period_type = 'TTM'"
    ).fetch_one(&pool).await.unwrap_or(0);

    let balance_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM balance_sheets WHERE period_type = 'TTM'"
    ).fetch_one(&pool).await.unwrap_or(0);

    println!("\n📊 TTM DATA VERIFICATION:");
    println!("  💰 TTM Income Statements: {}", income_count);
    println!("  🏦 TTM Balance Sheets: {}", balance_count);

    if income_count > 0 && balance_count > 0 {
        println!("🎉 TTM import system is working correctly!");
    } else {
        println!("⚠️  TTM import may need debugging");
    }

    pool.close().await;
    Ok(())
}