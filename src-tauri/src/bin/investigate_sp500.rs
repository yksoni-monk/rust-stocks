use sqlx::{SqlitePool, Row};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Investigating S&P 500 data completeness...");
    
    let database_url = "sqlite:../stocks.db";
    let pool = SqlitePool::connect(database_url).await?;
    
    // Check S&P 500 symbols table
    let row = sqlx::query("SELECT COUNT(*) as count FROM sp500_symbols").fetch_one(&pool).await?;
    let sp500_count: i64 = row.get("count");
    println!("📊 S&P 500 symbols in database: {}", sp500_count);
    
    // Check how many S&P 500 companies we have in stocks table
    let query = "
        SELECT COUNT(*) as count
        FROM stocks s
        INNER JOIN sp500_symbols sp ON s.symbol = sp.symbol
    ";
    let row = sqlx::query(query).fetch_one(&pool).await?;
    let stocks_with_sp500: i64 = row.get("count");
    println!("📊 S&P 500 companies in stocks table: {}", stocks_with_sp500);
    
    // Check how many have ANY price data
    let query = "
        SELECT COUNT(DISTINCT s.id) as count
        FROM stocks s
        INNER JOIN sp500_symbols sp ON s.symbol = sp.symbol
        INNER JOIN daily_prices dp ON s.id = dp.stock_id
    ";
    let row = sqlx::query(query).fetch_one(&pool).await?;
    let with_price_data: i64 = row.get("count");
    println!("📊 S&P 500 companies with ANY price data: {}", with_price_data);
    
    // Check how many have P/E data
    let query = "
        SELECT COUNT(DISTINCT s.id) as count
        FROM stocks s
        INNER JOIN sp500_symbols sp ON s.symbol = sp.symbol
        INNER JOIN daily_prices dp ON s.id = dp.stock_id
        WHERE dp.pe_ratio IS NOT NULL AND dp.pe_ratio > 0
    ";
    let row = sqlx::query(query).fetch_one(&pool).await?;
    let with_pe_data: i64 = row.get("count");
    println!("📊 S&P 500 companies with P/E data: {}", with_pe_data);
    
    // Find missing S&P 500 companies (in sp500_symbols but not in stocks)
    let query = "
        SELECT sp.symbol
        FROM sp500_symbols sp
        LEFT JOIN stocks s ON sp.symbol = s.symbol
        WHERE s.symbol IS NULL
        ORDER BY sp.symbol
    ";
    let rows = sqlx::query(query).fetch_all(&pool).await?;
    if !rows.is_empty() {
        println!("\n❌ S&P 500 symbols missing from stocks table ({}):", rows.len());
        for row in rows.iter().take(10) {
            let symbol: String = row.get("symbol");
            println!("   - {}", symbol);
        }
        if rows.len() > 10 {
            println!("   ... and {} more", rows.len() - 10);
        }
    }
    
    // Find S&P 500 companies with no price data
    let query = "
        SELECT s.symbol, s.company_name
        FROM stocks s
        INNER JOIN sp500_symbols sp ON s.symbol = sp.symbol
        LEFT JOIN daily_prices dp ON s.id = dp.stock_id
        WHERE dp.stock_id IS NULL
        ORDER BY s.symbol
    ";
    let rows = sqlx::query(query).fetch_all(&pool).await?;
    if !rows.is_empty() {
        println!("\n❌ S&P 500 companies with NO price data ({}):", rows.len());
        for row in rows.iter().take(10) {
            let symbol: String = row.get("symbol");
            let company_name: String = row.get("company_name");
            println!("   - {} ({})", symbol, company_name);
        }
        if rows.len() > 10 {
            println!("   ... and {} more", rows.len() - 10);
        }
    }
    
    // Find S&P 500 companies with price data but no P/E data
    let query = "
        SELECT s.symbol, s.company_name, COUNT(dp.id) as price_records
        FROM stocks s
        INNER JOIN sp500_symbols sp ON s.symbol = sp.symbol
        INNER JOIN daily_prices dp ON s.id = dp.stock_id
        WHERE s.id NOT IN (
            SELECT DISTINCT stock_id 
            FROM daily_prices 
            WHERE pe_ratio IS NOT NULL AND pe_ratio > 0
        )
        GROUP BY s.id, s.symbol, s.company_name
        ORDER BY s.symbol
    ";
    let rows = sqlx::query(query).fetch_all(&pool).await?;
    if !rows.is_empty() {
        println!("\n⚠️  S&P 500 companies with price data but NO P/E data ({}):", rows.len());
        for row in rows.iter().take(10) {
            let symbol: String = row.get("symbol");
            let company_name: String = row.get("company_name");
            let price_records: i64 = row.get("price_records");
            println!("   - {} ({}) - {} price records", symbol, company_name, price_records);
        }
        if rows.len() > 10 {
            println!("   ... and {} more", rows.len() - 10);
        }
    }
    
    println!("\n🔍 Data completeness analysis complete!");
    Ok(())
}