use sqlx::{SqlitePool, Row, Column};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Debugging SimFin import status...");
    
    let database_url = "sqlite:db/stocks.db";
    let pool = SqlitePool::connect(database_url).await?;
    
    // 1. Check table structure
    println!("📊 Database table structure:");
    
    let tables = vec!["stocks", "daily_prices", "quarterly_financials"];
    for table in tables {
        println!("\n📋 Table: {}", table);
        let query = format!("PRAGMA table_info({})", table);
        match sqlx::query(&query).fetch_all(&pool).await {
            Ok(rows) => {
                for row in rows {
                    let name: String = row.get("name");
                    let col_type: String = row.get("type");
                    println!("   - {} ({})", name, col_type);
                }
            }
            Err(e) => println!("   ❌ Error: {}", e)
        }
    }
    
    // 2. Check data counts
    println!("\n📊 Data counts:");
    
    let queries = vec![
        ("Total stocks", "SELECT COUNT(*) FROM stocks"),
        ("Daily prices", "SELECT COUNT(*) FROM daily_prices"),
        ("Quarterly financials", "SELECT COUNT(*) FROM quarterly_financials"),
        ("SP500 symbols", "SELECT COUNT(*) FROM sp500_symbols"),
    ];
    
    for (label, query) in queries {
        match sqlx::query(query).fetch_one(&pool).await {
            Ok(row) => {
                let count: i64 = row.get(0);
                println!("   {}: {}", label, count);
            }
            Err(e) => println!("   ❌ {}: Error - {}", label, e)
        }
    }
    
    // 3. Check quarterly_financials columns specifically for EPS/Net Income
    println!("\n🔍 Quarterly financials sample data:");
    let query = "SELECT * FROM quarterly_financials LIMIT 3";
    match sqlx::query(query).fetch_all(&pool).await {
        Ok(rows) => {
            for (i, row) in rows.iter().enumerate() {
                println!("   Record {}:", i + 1);
                // Get column names
                let columns = row.columns();
                for column in columns {
                    let col_name = column.name();
                    let value: Option<String> = row.try_get(col_name).ok();
                    println!("     {}: {:?}", col_name, value);
                }
                println!();
            }
        }
        Err(e) => println!("   ❌ Error: {}", e)
    }
    
    // 4. Check what SimFin ticker symbols look like vs S&P 500
    println!("🔍 Sample ticker matching:");
    let query = "
        SELECT s.symbol, s.company_name, COUNT(qf.id) as financial_records 
        FROM stocks s 
        LEFT JOIN quarterly_financials qf ON s.id = qf.stock_id
        WHERE s.symbol IN ('AAPL', 'MSFT', 'GOOGL', 'AMZN', 'TSLA')
        GROUP BY s.id, s.symbol, s.company_name
    ";
    match sqlx::query(query).fetch_all(&pool).await {
        Ok(rows) => {
            for row in rows {
                let symbol: String = row.get("symbol");
                let company_name: String = row.get("company_name");
                let financial_records: i64 = row.get("financial_records");
                println!("   {} ({}): {} financial records", symbol, company_name, financial_records);
            }
        }
        Err(e) => println!("   ❌ Error: {}", e)
    }
    
    // 5. Check if any stocks have pe_ratio calculated
    println!("\n📊 P/E ratio status:");
    let query = "
        SELECT 
            COUNT(*) as total_prices,
            COUNT(CASE WHEN pe_ratio IS NOT NULL AND pe_ratio > 0 THEN 1 END) as with_pe,
            MIN(pe_ratio) as min_pe,
            MAX(pe_ratio) as max_pe,
            AVG(pe_ratio) as avg_pe
        FROM daily_prices
    ";
    match sqlx::query(query).fetch_one(&pool).await {
        Ok(row) => {
            let total: i64 = row.get("total_prices");
            let with_pe: i64 = row.get("with_pe");
            let min_pe: Option<f64> = row.try_get("min_pe").ok().flatten();
            let max_pe: Option<f64> = row.try_get("max_pe").ok().flatten();
            let avg_pe: Option<f64> = row.try_get("avg_pe").ok().flatten();
            
            println!("   Total price records: {}", total);
            println!("   With P/E ratio: {} ({:.1}%)", with_pe, (with_pe as f64 / total as f64) * 100.0);
            println!("   P/E range: {:?} - {:?} (avg: {:?})", min_pe, max_pe, avg_pe);
        }
        Err(e) => println!("   ❌ Error: {}", e)
    }
    
    println!("\n🔍 Diagnosis complete!");
    Ok(())
}