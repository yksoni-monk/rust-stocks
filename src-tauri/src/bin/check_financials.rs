use sqlx::{SqlitePool, Row};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Checking S&P 500 quarterly financials and P/E calculation status...");
    
    let database_url = "sqlite:db/stocks.db";
    let pool = SqlitePool::connect(database_url).await?;
    
    // 1. Check if quarterly_financials table exists and has data
    let tables_query = "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE '%financial%'";
    let rows = sqlx::query(tables_query).fetch_all(&pool).await?;
    println!("📊 Financial tables found:");
    for row in rows {
        let table_name: String = row.get("name");
        println!("   - {}", table_name);
    }
    
    // 2. Check total quarterly financials records
    let query = "SELECT COUNT(*) as count FROM quarterly_financials";
    match sqlx::query(query).fetch_one(&pool).await {
        Ok(row) => {
            let count: i64 = row.get("count");
            println!("📊 Total quarterly financials records: {}", count);
        }
        Err(e) => println!("❌ Error checking quarterly_financials: {}", e)
    }
    
    // 3. Check S&P 500 companies with quarterly financials
    let query = "
        SELECT COUNT(DISTINCT s.id) as count
        FROM stocks s
        INNER JOIN sp500_symbols sp ON s.symbol = sp.symbol
        INNER JOIN quarterly_financials qf ON s.id = qf.stock_id
    ";
    match sqlx::query(query).fetch_one(&pool).await {
        Ok(row) => {
            let count: i64 = row.get("count");
            println!("📊 S&P 500 companies with quarterly financials: {}", count);
        }
        Err(e) => println!("❌ Error checking S&P 500 financials: {}", e)
    }
    
    // 4. Check if EPS has been calculated
    let query = "
        SELECT COUNT(*) as count
        FROM quarterly_financials
        WHERE eps IS NOT NULL
    ";
    match sqlx::query(query).fetch_one(&pool).await {
        Ok(row) => {
            let count: i64 = row.get("count");
            println!("📊 Quarterly records with calculated EPS: {}", count);
        }
        Err(e) => println!("❌ Error checking EPS: {}", e)
    }
    
    // 5. Check P/E ratios in daily_prices
    let query = "
        SELECT COUNT(*) as total_prices,
               COUNT(CASE WHEN pe_ratio IS NOT NULL THEN 1 END) as with_pe
        FROM daily_prices
    ";
    match sqlx::query(query).fetch_one(&pool).await {
        Ok(row) => {
            let total_prices: i64 = row.get("total_prices");
            let with_pe: i64 = row.get("with_pe");
            let percentage = if total_prices > 0 { (with_pe as f64 / total_prices as f64) * 100.0 } else { 0.0 };
            println!("📊 Daily prices: {} total, {} with P/E ({:.1}%)", total_prices, with_pe, percentage);
        }
        Err(e) => println!("❌ Error checking P/E ratios: {}", e)
    }
    
    // 6. Sample some S&P 500 companies without P/E ratios to see if they have financials
    let query = "
        SELECT s.symbol, s.company_name, 
               COUNT(DISTINCT qf.id) as financial_records,
               COUNT(DISTINCT dp.id) as price_records,
               COUNT(CASE WHEN dp.pe_ratio IS NOT NULL THEN 1 END) as pe_records
        FROM stocks s
        INNER JOIN sp500_symbols sp ON s.symbol = sp.symbol
        LEFT JOIN quarterly_financials qf ON s.id = qf.stock_id
        LEFT JOIN daily_prices dp ON s.id = dp.stock_id
        WHERE s.id NOT IN (
            SELECT DISTINCT stock_id 
            FROM daily_prices 
            WHERE pe_ratio IS NOT NULL AND pe_ratio > 0
        )
        GROUP BY s.id, s.symbol, s.company_name
        HAVING price_records > 0
        ORDER BY financial_records DESC
        LIMIT 10
    ";
    match sqlx::query(query).fetch_all(&pool).await {
        Ok(rows) => {
            if !rows.is_empty() {
                println!("\n🔍 S&P 500 companies without P/E but with data:");
                for row in rows {
                    let symbol: String = row.get("symbol");
                    let company_name: String = row.get("company_name");
                    let financial_records: i64 = row.get("financial_records");
                    let price_records: i64 = row.get("price_records");
                    let pe_records: i64 = row.get("pe_records");
                    println!("   {} ({}) - {} financials, {} prices, {} P/E", 
                            symbol, company_name, financial_records, price_records, pe_records);
                }
            }
        }
        Err(e) => println!("❌ Error checking sample companies: {}", e)
    }
    
    // 7. Check the P/E calculation status
    println!("\n💡 Next steps based on findings:");
    
    let query = "SELECT COUNT(DISTINCT stock_id) as count FROM quarterly_financials WHERE eps IS NOT NULL";
    match sqlx::query(query).fetch_one(&pool).await {
        Ok(row) => {
            let eps_companies: i64 = row.get("count");
            if eps_companies == 0 {
                println!("❌ No EPS calculated - run: calculate_and_store_eps()");
            } else {
                println!("✅ EPS calculated for {} companies", eps_companies);
            }
        }
        Err(_) => {}
    }
    
    let query = "SELECT COUNT(*) as count FROM daily_prices WHERE pe_ratio IS NOT NULL";
    match sqlx::query(query).fetch_one(&pool).await {
        Ok(row) => {
            let pe_count: i64 = row.get("count");
            if pe_count == 0 {
                println!("❌ No P/E ratios calculated - run: calculate_and_store_pe_ratios()");
            } else {
                println!("✅ P/E ratios exist in database");
            }
        }
        Err(_) => {}
    }
    
    println!("\n🔍 Analysis complete!");
    Ok(())
}