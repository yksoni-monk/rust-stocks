use sqlx::sqlite::SqlitePoolOptions;
use std::time::Instant;
use rust_stocks_tauri_lib::tools::simfin_importer::{
    calculate_and_store_eps,
    calculate_and_store_pe_ratios,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧮 Running P/E calculation for missing data...");
    
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite:../stocks.db?mode=rwc")
        .await?;
    
    // Phase 1: Calculate EPS (if missing)
    println!("\n🧮 PHASE 1: EPS Calculation");
    let start = Instant::now();
    match calculate_and_store_eps(&pool).await {
        Ok(count) => {
            println!("✅ Phase 1 Complete: {} EPS values calculated in {:?}", count, start.elapsed());
        }
        Err(e) => {
            println!("❌ Phase 1 Failed: {}", e);
            // Don't return - maybe EPS already exists
        }
    }
    
    // Phase 2: Calculate P/E ratios
    println!("\n📊 PHASE 2: P/E Ratio Calculation");  
    let start = Instant::now();
    match calculate_and_store_pe_ratios(&pool).await {
        Ok(count) => {
            println!("✅ Phase 2 Complete: {} P/E ratios calculated in {:?}", count, start.elapsed());
        }
        Err(e) => {
            println!("❌ Phase 2 Failed: {}", e);
        }
    }
    
    // Update the cache table
    println!("\n🔄 PHASE 3: Refresh cache table");
    let start = Instant::now();
    
    sqlx::query("DROP TABLE IF EXISTS sp500_pe_cache").execute(&pool).await?;
    
    let create_cache = "
        CREATE TABLE sp500_pe_cache AS
        SELECT DISTINCT s.id, s.symbol, s.company_name
        FROM stocks s
        INNER JOIN sp500_symbols sp ON s.symbol = sp.symbol
        INNER JOIN daily_prices dp ON s.id = dp.stock_id
        WHERE dp.pe_ratio IS NOT NULL AND dp.pe_ratio > 0
        ORDER BY s.symbol
    ";
    sqlx::query(create_cache).execute(&pool).await?;
    
    // Create indexes
    sqlx::query("CREATE INDEX idx_sp500_pe_cache_id ON sp500_pe_cache(id)").execute(&pool).await?;
    sqlx::query("CREATE INDEX idx_sp500_pe_cache_symbol ON sp500_pe_cache(symbol)").execute(&pool).await?;
    
    // Check results
    let row = sqlx::query("SELECT COUNT(*) as count FROM sp500_pe_cache")
        .fetch_one(&pool).await?;
    let count: i64 = row.get("count");
    
    println!("✅ Phase 3 Complete: Cache updated with {} S&P 500 stocks in {:?}", count, start.elapsed());
    
    println!("\n🎉 P/E calculation complete!");
    println!("📊 S&P 500 stocks with P/E data: {}/503", count);
    
    pool.close().await;
    Ok(())
}

use sqlx::Row;