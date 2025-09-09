use sqlx::SqlitePool;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Creating database indexes for performance...");
    
    let database_url = "sqlite:../stocks.db";
    let pool = SqlitePool::connect(database_url).await?;
    
    let indexes = vec![
        ("idx_daily_prices_stock_id", "CREATE INDEX IF NOT EXISTS idx_daily_prices_stock_id ON daily_prices(stock_id)"),
        ("idx_daily_prices_pe_ratio", "CREATE INDEX IF NOT EXISTS idx_daily_prices_pe_ratio ON daily_prices(pe_ratio)"),
        ("idx_daily_prices_stock_pe", "CREATE INDEX IF NOT EXISTS idx_daily_prices_stock_pe ON daily_prices(stock_id, pe_ratio)"),
        ("idx_daily_prices_date", "CREATE INDEX IF NOT EXISTS idx_daily_prices_date ON daily_prices(date)"),
        ("idx_stocks_symbol", "CREATE INDEX IF NOT EXISTS idx_stocks_symbol ON stocks(symbol)"),
        ("idx_sp500_symbol", "CREATE INDEX IF NOT EXISTS idx_sp500_symbol ON sp500_symbols(symbol)"),
    ];
    
    for (name, sql) in indexes {
        let start = Instant::now();
        sqlx::query(sql).execute(&pool).await?;
        println!("✅ Created index {}: {:?}", name, start.elapsed());
    }
    
    // Test the performance improvement
    println!("\n🚀 Testing performance after indexing...");
    
    let start = Instant::now();
    let query = "
        SELECT COUNT(DISTINCT s.id) as count
        FROM stocks s
        INNER JOIN sp500_symbols sp ON s.symbol = sp.symbol
        INNER JOIN daily_prices dp ON s.id = dp.stock_id
        WHERE dp.pe_ratio IS NOT NULL AND dp.pe_ratio > 0
    ";
    let row = sqlx::query(query).fetch_one(&pool).await?;
    let count: i64 = row.get("count");
    println!("⏱️  Count S&P 500 stocks with P/E data: {} stocks in {:?}", count, start.elapsed());
    
    // Test sample stock query
    let start = Instant::now();
    let stocks_query = "
        SELECT DISTINCT s.id, s.symbol, s.company_name
        FROM stocks s
        INNER JOIN sp500_symbols sp ON s.symbol = sp.symbol
        INNER JOIN daily_prices dp ON s.id = dp.stock_id
        WHERE dp.pe_ratio IS NOT NULL AND dp.pe_ratio > 0
        ORDER BY s.symbol
        LIMIT 10
    ";
    let _rows = sqlx::query(stocks_query).fetch_all(&pool).await?;
    println!("⏱️  Get S&P 500 stock list (10 stocks): {:?}", start.elapsed());
    
    println!("\n🎉 Database optimization complete!");
    Ok(())
}

use sqlx::Row;