use anyhow::Result;
use chrono::NaiveDate;
use std::env;

use rust_stocks::api::{SchwabClient, StockDataProvider};
use rust_stocks::database::DatabaseManager;
use rust_stocks::data_collector::DataCollector;
use rust_stocks::models::Config;
use rust_stocks::utils::MarketCalendar;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: cargo run --bin smart_collect -- <start_date> [end_date]");
        println!("Example: cargo run --bin smart_collect -- 20250810");
        println!("This tool automatically handles weekends and holidays!");
        return Ok(())
    }

    let start_str = &args[1];
    let end_str = if args.len() > 2 { &args[2] } else { start_str };

    println!("🗓️  Smart Stock Data Collection");
    println!("📅 Requested date range: {} to {}", start_str, end_str);

    // Parse dates
    let start_date = parse_date(start_str)?;
    let end_date = parse_date(end_str)?;

    // Weekend adjustment using shared utility
    let adjusted_start = MarketCalendar::adjust_for_weekend(start_date);
    let adjusted_end = MarketCalendar::adjust_for_weekend(end_date);

    if adjusted_start != start_date || adjusted_end != end_date {
        println!("📅 Date range adjusted for weekends:");
        println!("   Original: {} to {}", start_date, end_date);
        println!("   Adjusted: {} to {}", adjusted_start, adjusted_end);
    } else {
        println!("✅ Requested dates are valid (not weekends)");
    }

    // Setup
    let config = Config::from_env()?;
    let database = DatabaseManager::new(&config.database_path)?;
    let client = SchwabClient::new(&config)?;

    println!("✅ Setup complete");

    // Get first 10 stocks to test with
    let data_collector = DataCollector::new(client, database, config);
    let stocks = data_collector.get_active_stocks()?;
    let test_stocks: Vec<_> = stocks.into_iter().take(10).collect();

    println!("📊 Testing with {} stocks", test_stocks.len());

    // Process each stock with adjusted dates
    let mut total_records = 0;
    for (i, stock) in test_stocks.iter().enumerate() {
        print!("[{}/{}] Processing {}... ", i+1, test_stocks.len(), stock.symbol);
        
        let schwab_client = SchwabClient::new(&Config::from_env()?)?;
        match schwab_client.get_price_history(&stock.symbol, adjusted_start, adjusted_end).await {
            Ok(bars) => {
                println!("✅ {} records", bars.len());
                total_records += bars.len();
                
                // Show sample data for first stock
                if i == 0 && !bars.is_empty() {
                    println!("   Sample: Open=${:.2}, High=${:.2}, Low=${:.2}, Close=${:.2}", 
                             bars[0].open, bars[0].high, bars[0].low, bars[0].close);
                }
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }

    println!("\n🎉 Completed! Total records: {}", total_records);
    println!("💡 Weekend adjustment handled: {}", 
             if adjusted_start != start_date || adjusted_end != end_date { "yes" } else { "no" });
    
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

