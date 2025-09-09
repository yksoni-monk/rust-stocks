use sqlx::{SqlitePool, Row};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Analyzing why S&P 500 stocks are missing P/E data...");
    
    let database_url = "sqlite:db/stocks.db";
    let pool = SqlitePool::connect(database_url).await?;
    
    // 1. Find S&P 500 stocks missing from stocks table
    println!("\n❌ S&P 500 symbols missing from stocks table:");
    let query = "
        SELECT sp.symbol
        FROM sp500_symbols sp
        LEFT JOIN stocks s ON sp.symbol = s.symbol
        WHERE s.symbol IS NULL
        ORDER BY sp.symbol
    ";
    let rows = sqlx::query(query).fetch_all(&pool).await?;
    println!("   Count: {}", rows.len());
    for row in rows.iter().take(10) {
        let symbol: String = row.get("symbol");
        println!("   - {}", symbol);
    }
    if rows.len() > 10 {
        println!("   ... and {} more", rows.len() - 10);
    }
    
    // 2. S&P 500 stocks with no price data
    println!("\n💸 S&P 500 stocks with no price data:");
    let query = "
        SELECT s.symbol, s.company_name
        FROM stocks s
        INNER JOIN sp500_symbols sp ON s.symbol = sp.symbol
        LEFT JOIN daily_prices dp ON s.id = dp.stock_id
        WHERE dp.stock_id IS NULL
        ORDER BY s.symbol
    ";
    let rows = sqlx::query(query).fetch_all(&pool).await?;
    println!("   Count: {}", rows.len());
    for row in rows.iter().take(10) {
        let symbol: String = row.get("symbol");
        let company_name: String = row.get("company_name");
        println!("   - {} ({})", symbol, company_name);
    }
    if rows.len() > 10 {
        println!("   ... and {} more", rows.len() - 10);
    }
    
    // 3. S&P 500 stocks with no quarterly financials
    println!("\n🏢 S&P 500 stocks with no quarterly financials:");
    let query = "
        SELECT s.symbol, s.company_name, COUNT(dp.id) as price_records
        FROM stocks s
        INNER JOIN sp500_symbols sp ON s.symbol = sp.symbol
        INNER JOIN daily_prices dp ON s.id = dp.stock_id
        LEFT JOIN quarterly_financials qf ON s.id = qf.stock_id
        WHERE qf.stock_id IS NULL
        GROUP BY s.id, s.symbol, s.company_name
        ORDER BY s.symbol
    ";
    let rows = sqlx::query(query).fetch_all(&pool).await?;
    println!("   Count: {}", rows.len());
    for row in rows.iter().take(10) {
        let symbol: String = row.get("symbol");
        let company_name: String = row.get("company_name");
        let price_records: i64 = row.get("price_records");
        println!("   - {} ({}) - {} price records", symbol, company_name, price_records);
    }
    if rows.len() > 10 {
        println!("   ... and {} more", rows.len() - 10);
    }
    
    // 4. S&P 500 stocks with financials but no calculated EPS
    println!("\n🧮 S&P 500 stocks with financials but no calculated EPS:");
    let query = "
        SELECT s.symbol, s.company_name, COUNT(qf.id) as financial_records,
               COUNT(CASE WHEN qf.net_income IS NOT NULL THEN 1 END) as with_net_income,
               COUNT(CASE WHEN qf.shares_diluted IS NOT NULL AND qf.shares_diluted > 0 THEN 1 END) as with_shares
        FROM stocks s
        INNER JOIN sp500_symbols sp ON s.symbol = sp.symbol
        INNER JOIN quarterly_financials qf ON s.id = qf.stock_id
        WHERE s.id NOT IN (
            SELECT DISTINCT stock_id 
            FROM quarterly_financials 
            WHERE eps_calculated IS NOT NULL
        )
        GROUP BY s.id, s.symbol, s.company_name
        ORDER BY s.symbol
        LIMIT 20
    ";
    let rows = sqlx::query(query).fetch_all(&pool).await?;
    println!("   Count (showing first 20): {}", rows.len());
    for row in rows {
        let symbol: String = row.get("symbol");
        let company_name: String = row.get("company_name");
        let financial_records: i64 = row.get("financial_records");
        let with_net_income: i64 = row.get("with_net_income");
        let with_shares: i64 = row.get("with_shares");
        println!("   - {} ({}) - {} financials, {} with net_income, {} with shares", 
                symbol, company_name, financial_records, with_net_income, with_shares);
    }
    
    // 5. S&P 500 stocks with EPS but no P/E ratios
    println!("\n📊 S&P 500 stocks with EPS but no P/E ratios in daily_prices:");
    let query = "
        SELECT s.symbol, s.company_name, 
               COUNT(dp.id) as price_records,
               COUNT(CASE WHEN dp.pe_ratio IS NOT NULL THEN 1 END) as pe_records,
               COUNT(qf.id) as eps_records
        FROM stocks s
        INNER JOIN sp500_symbols sp ON s.symbol = sp.symbol
        INNER JOIN daily_prices dp ON s.id = dp.stock_id
        INNER JOIN quarterly_financials qf ON s.id = qf.stock_id AND qf.eps_calculated IS NOT NULL
        WHERE s.id NOT IN (
            SELECT DISTINCT stock_id 
            FROM daily_prices 
            WHERE pe_ratio IS NOT NULL
        )
        GROUP BY s.id, s.symbol, s.company_name
        ORDER BY s.symbol
        LIMIT 20
    ";
    let rows = sqlx::query(query).fetch_all(&pool).await?;
    println!("   Count (showing first 20): {}", rows.len());
    for row in rows {
        let symbol: String = row.get("symbol");
        let company_name: String = row.get("company_name");
        let price_records: i64 = row.get("price_records");
        let pe_records: i64 = row.get("pe_records");
        let eps_records: i64 = row.get("eps_records");
        println!("   - {} ({}) - {} prices, {} P/E, {} EPS", 
                symbol, company_name, price_records, pe_records, eps_records);
    }
    
    println!("\n🔍 Analysis complete - real reasons identified!");
    Ok(())
}