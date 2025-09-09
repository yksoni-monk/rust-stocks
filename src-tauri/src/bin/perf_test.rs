use sqlx::{SqlitePool, Row};
use std::time::Instant;
use rust_stocks_tauri_lib::analysis::recommendation_engine::RecommendationEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Performance Analysis Tool");
    
    // Connect to database
    let start = Instant::now();
    let database_url = "sqlite:../stocks.db";
    let pool = SqlitePool::connect(database_url).await?;
    println!("⏱️  Database connection: {:?}", start.elapsed());
    
    // Test 1: Count S&P 500 stocks with P/E data
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
    
    // Test 2: Get S&P 500 stocks list
    let start = Instant::now();
    let engine = RecommendationEngine::new(pool.clone());
    let stocks_query = "
        SELECT DISTINCT s.id, s.symbol, s.company_name
        FROM stocks s
        INNER JOIN sp500_symbols sp ON s.symbol = sp.symbol
        INNER JOIN daily_prices dp ON s.id = dp.stock_id
        WHERE dp.pe_ratio IS NOT NULL AND dp.pe_ratio > 0
        ORDER BY s.symbol
        LIMIT 10
    ";
    let rows = sqlx::query(stocks_query).fetch_all(&pool).await?;
    println!("⏱️  Get S&P 500 stock list (10 stocks): {:?}", start.elapsed());
    
    // Test 3: Single stock P/E analysis
    if let Some(row) = rows.first() {
        let stock_id: i64 = row.get("id");
        let symbol: String = row.get("symbol");
        
        let start = Instant::now();
        let pe_query = "
            SELECT COUNT(*) as count
            FROM daily_prices
            WHERE stock_id = ? AND pe_ratio IS NOT NULL AND pe_ratio > 0
        ";
        let pe_row = sqlx::query(pe_query).bind(stock_id).fetch_one(&pool).await?;
        let pe_count: i64 = pe_row.get("count");
        println!("⏱️  Count P/E records for {}: {} records in {:?}", symbol, pe_count, start.elapsed());
        
        // Test actual P/E data retrieval
        let start = Instant::now();
        let pe_data_query = "
            SELECT pe_ratio
            FROM daily_prices
            WHERE stock_id = ? AND pe_ratio IS NOT NULL AND pe_ratio > 0
            ORDER BY date
        ";
        let pe_rows = sqlx::query(pe_data_query).bind(stock_id).fetch_all(&pool).await?;
        println!("⏱️  Fetch P/E data for {}: {} records in {:?}", symbol, pe_rows.len(), start.elapsed());
    }
    
    // Test 4: Full recommendation system (small sample)
    let start = Instant::now();
    let recommendations = engine.get_value_recommendations(Some(5)).await?;
    println!("⏱️  Generate top 5 recommendations: {} results in {:?}", recommendations.len(), start.elapsed());
    
    // Test 5: Database size check
    let start = Instant::now();
    let size_query = "
        SELECT 
            (SELECT COUNT(*) FROM stocks) as stock_count,
            (SELECT COUNT(*) FROM daily_prices) as price_count,
            (SELECT COUNT(*) FROM sp500_symbols) as sp500_count
    ";
    let size_row = sqlx::query(size_query).fetch_one(&pool).await?;
    let stock_count: i64 = size_row.get("stock_count");
    let price_count: i64 = size_row.get("price_count");
    let sp500_count: i64 = size_row.get("sp500_count");
    println!("⏱️  Database size check: {} stocks, {} prices, {} S&P500 in {:?}", 
             stock_count, price_count, sp500_count, start.elapsed());
    
    println!("\n📊 Database Analysis Complete");
    Ok(())
}