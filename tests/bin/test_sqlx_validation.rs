use anyhow::Result;
use chrono::{NaiveDate, DateTime, Utc};
use sqlx::{sqlite::SqlitePoolOptions, Row};

// Simple test structs for validation
#[derive(Debug)]
struct TestStock {
    id: Option<i64>,
    symbol: String,
    company_name: String,
    sector: Option<String>,
    industry: Option<String>,
    market_cap: Option<f64>,
    status: String,
    first_trading_date: Option<NaiveDate>,
    last_updated: Option<DateTime<Utc>>,
}

#[derive(Debug)]
struct TestDailyPrice {
    id: Option<i64>,
    stock_id: i64,
    date: NaiveDate,
    open_price: f64,
    high_price: f64,
    low_price: f64,
    close_price: f64,
    volume: Option<i64>,
}

/// Test SQLX implementation with existing database
async fn test_sqlx_with_existing_db() -> Result<()> {
    println!("🚀 Testing SQLX implementation with existing database...");
    
    // Connect to existing database
    let database_url = "sqlite:stocks.db";
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;
    
    println!("✅ Connected to existing database");
    
    // Test 1: Check if tables exist
    let tables = sqlx::query("SELECT name FROM sqlite_master WHERE type='table'")
        .fetch_all(&pool)
        .await?;
    
    println!("📋 Found tables: {:?}", tables.iter().map(|r| r.get::<String, _>("name")).collect::<Vec<_>>());
    
    // Test 2: Count existing stocks
    let stock_count = sqlx::query("SELECT COUNT(*) as count FROM stocks")
        .fetch_one(&pool)
        .await?;
    let count = stock_count.get::<i64, _>("count");
    println!("📊 Found {} stocks in database", count);
    
    // Test 3: Get a sample stock
    if count > 0 {
        let sample_stock = sqlx::query(
            "SELECT id, symbol, company_name, sector, industry, market_cap, status, first_trading_date, last_updated FROM stocks LIMIT 1"
        )
        .fetch_optional(&pool)
        .await?;
        
        if let Some(row) = sample_stock {
            let stock = TestStock {
                id: Some(row.get::<i64, _>("id")),
                symbol: row.get::<String, _>("symbol"),
                company_name: row.get::<String, _>("company_name"),
                sector: row.get::<Option<String>, _>("sector"),
                industry: row.get::<Option<String>, _>("industry"),
                market_cap: row.get::<Option<f64>, _>("market_cap"),
                status: row.get::<Option<String>, _>("status").unwrap_or_else(|| "active".to_string()),
                first_trading_date: row.get::<Option<NaiveDate>, _>("first_trading_date"),
                last_updated: row.get::<Option<DateTime<Utc>>, _>("last_updated"),
            };
            println!("📈 Sample stock: {:?}", stock);
        }
    }
    
    // Test 4: Count existing prices
    let price_count = sqlx::query("SELECT COUNT(*) as count FROM daily_prices")
        .fetch_one(&pool)
        .await?;
    let price_count = price_count.get::<i64, _>("count");
    println!("📊 Found {} price records in database", price_count);
    
    // Test 5: Test metadata
    let metadata_count = sqlx::query("SELECT COUNT(*) as count FROM metadata")
        .fetch_one(&pool)
        .await?;
    let metadata_count = metadata_count.get::<i64, _>("count");
    println!("📊 Found {} metadata records in database", metadata_count);
    
    // Test 6: Test inserting a test stock
    let test_symbol = "TEST_SQLX";
    let result = sqlx::query(
        r#"
        INSERT INTO stocks (symbol, company_name, sector, industry, market_cap, status, first_trading_date, last_updated)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(symbol) DO UPDATE SET
            company_name = excluded.company_name,
            last_updated = excluded.last_updated
        RETURNING id
        "#
    )
    .bind(test_symbol)
    .bind("SQLX Test Company")
    .bind("Technology")
    .bind("Software")
    .bind(1000000.0)
    .bind("active")
    .bind(NaiveDate::from_ymd_opt(2024, 1, 1))
    .bind(Utc::now().naive_utc())
    .fetch_one(&pool)
    .await?;
    
    let test_stock_id = result.get::<i64, _>("id");
    println!("✅ Inserted test stock with ID: {}", test_stock_id);
    
    // Test 7: Test inserting a test price
    let test_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let price_result = sqlx::query(
        r#"
        INSERT INTO daily_prices (stock_id, date, open_price, high_price, low_price, close_price, volume)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(stock_id, date) DO UPDATE SET
            open_price = excluded.open_price,
            close_price = excluded.close_price
        RETURNING id
        "#
    )
    .bind(test_stock_id)
    .bind(test_date)
    .bind(100.0)
    .bind(105.0)
    .bind(95.0)
    .bind(102.0)
    .bind(1000000)
    .fetch_one(&pool)
    .await?;
    
    let test_price_id = price_result.get::<i64, _>("id");
    println!("✅ Inserted test price with ID: {}", test_price_id);
    
    // Test 8: Clean up test data
    sqlx::query("DELETE FROM daily_prices WHERE stock_id = ?").bind(test_stock_id).execute(&pool).await?;
    sqlx::query("DELETE FROM stocks WHERE id = ?").bind(test_stock_id).execute(&pool).await?;
    println!("🧹 Cleaned up test data");
    
    pool.close().await;
    println!("✅ SQLX implementation test completed successfully!");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    test_sqlx_with_existing_db().await?;
    Ok(())
}
