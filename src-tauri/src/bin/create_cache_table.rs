use sqlx::{SqlitePool, Row};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Creating performance cache table...");
    
    let database_url = "sqlite:db/stocks.db";
    let pool = SqlitePool::connect(database_url).await?;
    
    // Create cache table for S&P 500 stocks with P/E data
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
    println!("✅ Created cache table: {:?}", start.elapsed());
    
    // Create index on cache table
    sqlx::query("CREATE INDEX idx_sp500_pe_cache_id ON sp500_pe_cache(id)").execute(&pool).await?;
    sqlx::query("CREATE INDEX idx_sp500_pe_cache_symbol ON sp500_pe_cache(symbol)").execute(&pool).await?;
    
    // Test performance
    let start = Instant::now();
    let row = sqlx::query("SELECT COUNT(*) as count FROM sp500_pe_cache").fetch_one(&pool).await?;
    let count: i64 = row.get("count");
    println!("⏱️  Count from cache: {} stocks in {:?}", count, start.elapsed());
    
    let start = Instant::now();
    let _rows = sqlx::query("SELECT * FROM sp500_pe_cache LIMIT 10").fetch_all(&pool).await?;
    println!("⏱️  Get 10 stocks from cache: {:?}", start.elapsed());
    
    println!("\n🎉 Cache table created successfully!");
    println!("💡 Now update the recommendation engine to use sp500_pe_cache table");
    
    Ok(())
}