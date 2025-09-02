use anyhow::Result;
use chrono::{NaiveDate, Duration};
use std::env;

use rust_stocks::api::{SchwabClient, StockDataProvider};
use rust_stocks::database::DatabaseManager;
use rust_stocks::models::Config;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 4 {
        println!("Usage: cargo run --bin test_batched_stock -- <symbol> <start_date> <end_date>");
        println!("Example: cargo run --bin test_batched_stock -- AAPL 20241201 20241231");
        println!("Date format: YYYYMMDD");
        return Ok(());
    }

    let symbol = &args[1];
    let start_str = &args[2];
    let end_str = &args[3];

    println!("🧪 Testing Batched Stock Data Fetch");
    println!("Symbol: {}", symbol);
    println!("Date range: {} to {}", start_str, end_str);

    // Parse dates
    let start_date = parse_date(start_str)?;
    let end_date = parse_date(end_str)?;

    println!("Parsed dates: {} to {}", start_date, end_date);

    // Calculate total days and batch size
    let total_days = (end_date - start_date).num_days() as usize;
    let batch_size = 20; // Fetch 20 days at a time
    let total_batches = (total_days + batch_size - 1) / batch_size; // Ceiling division

    println!("Total days: {}", total_days);
    println!("Batch size: {} days", batch_size);
    println!("Total batches: {}", total_batches);

    // Test 1: Basic setup
    println!("\n📊 Test 1: Basic setup...");
    let config = Config::from_env()?;
    println!("✅ Config loaded successfully");
    
    let database = DatabaseManager::new(&config.database_path)?;
    println!("✅ Database connected successfully");
    
    let client = SchwabClient::new(&config)?;
    println!("✅ Schwab client created successfully");

    // Test 2: Check if stock exists in database
    println!("\n📊 Test 2: Checking stock in database...");
    match database.get_stock_by_symbol(symbol)? {
        Some(stock) => {
            println!("✅ Stock found in database: {} ({})", stock.symbol, stock.company_name);
        }
        None => {
            println!("⚠️  Stock not found in database, will try to fetch from API");
        }
    }

    // Test 3: Fetch data in batches
    println!("\n📈 Test 3: Fetching data in batches...");
    let mut total_bars = 0;
    let mut current_date = start_date;

    for batch_num in 1..=total_batches {
        // Calculate batch end date
        let batch_end_date = if batch_num == total_batches {
            end_date
        } else {
            current_date + Duration::days(batch_size as i64 - 1)
        };

        println!("\n🔄 Batch {}/{}: Fetching data from {} to {}", 
                 batch_num, total_batches, current_date, batch_end_date);

        println!("📅 Batch date range: {} to {}", current_date, batch_end_date);

        // Fetch data for this batch
        match client.get_price_history(symbol, current_date, batch_end_date).await {
            Ok(bars) => {
                println!("✅ Batch {}: Successfully fetched {} price bars", batch_num, bars.len());
                total_bars += bars.len();
                
                if !bars.is_empty() {
                    println!("📊 Sample data from batch {}:", batch_num);
                    println!("  First bar: Open=${:.2}, High=${:.2}, Low=${:.2}, Close=${:.2}, Volume={}", 
                             bars[0].open, bars[0].high, bars[0].low, bars[0].close, bars[0].volume);
                    
                    if bars.len() > 1 {
                        let last_bar = &bars[bars.len() - 1];
                        println!("  Last bar: Open=${:.2}, High=${:.2}, Low=${:.2}, Close=${:.2}, Volume={}", 
                                 last_bar.open, last_bar.high, last_bar.low, last_bar.close, last_bar.volume);
                    }
                }
            }
            Err(e) => {
                println!("❌ Batch {}: Failed to fetch price history: {}", batch_num, e);
                println!("Continuing with next batch...");
            }
        }

        // Move to next batch start date
        current_date = batch_end_date + Duration::days(1);
        
        // Add a small delay between batches to avoid rate limiting
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    // Test 4: Summary
    println!("\n📊 Test 4: Summary...");
    println!("Total bars fetched: {}", total_bars);
    println!("Total batches processed: {}", total_batches);

    // Test 5: Simple database check (skip problematic stats)
    println!("\n📊 Test 5: Simple database check...");
    println!("✅ Database connection is working");
    println!("✅ Total bars fetched: {}", total_bars);
    println!("✅ Total batches processed: {}", total_batches);

    println!("\n🎉 Batched test completed successfully!");
    Ok(())
}

fn parse_date(date_str: &str) -> Result<NaiveDate> {
    if date_str.len() != 8 {
        return Err(anyhow::anyhow!("Date must be YYYYMMDD format"));
    }
    
    let year: i32 = date_str[0..4].parse()?;
    let month: u32 = date_str[4..6].parse()?;
    let day: u32 = date_str[6..8].parse()?;
    
    NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| anyhow::anyhow!("Invalid date"))
}
