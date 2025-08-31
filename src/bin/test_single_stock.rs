use anyhow::Result;
use chrono::NaiveDate;
use std::env;

use rust_stocks::api::{SchwabClient, StockDataProvider};
use rust_stocks::database::DatabaseManager;
use rust_stocks::models::Config;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 4 {
        println!("Usage: cargo run --bin test_single_stock -- <symbol> <start_date> <end_date>");
        println!("Example: cargo run --bin test_single_stock -- AAPL 20241201 20241231");
        println!("Date format: YYYYMMDD");
        return Ok(());
    }

    let symbol = &args[1];
    let start_str = &args[2];
    let end_str = &args[3];

    println!("🧪 Testing Single Stock Data Fetch");
    println!("Symbol: {}", symbol);
    println!("Date range: {} to {}", start_str, end_str);

    // Parse dates
    let start_date = parse_date(start_str)?;
    let end_date = parse_date(end_str)?;

    println!("Parsed dates: {} to {}", start_date, end_date);

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

    // Test 3: Check database stats
    println!("\n📊 Test 3: Database statistics...");
    let (total_stocks, total_prices, last_update) = database.get_stats()?;
    println!("Total stocks: {}", total_stocks);
    println!("Total price records: {}", total_prices);
    println!("Last update: {:?}", last_update);

    // Test 4: Try a simple API call (just to test connectivity)
    println!("\n📈 Test 4: Testing API connectivity...");
    println!("Attempting to fetch price history for {} from {} to {}", symbol, start_date, end_date);
    
    match client.get_price_history(symbol, start_date, end_date).await {
        Ok(bars) => {
            println!("✅ Successfully fetched {} price bars", bars.len());
            
            if !bars.is_empty() {
                println!("Sample data:");
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
            println!("❌ Failed to fetch price history: {}", e);
            println!("This might be due to API authentication or network issues");
            return Err(e.into());
        }
    }

    println!("\n🎉 Test completed successfully!");
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
