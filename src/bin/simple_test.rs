use anyhow::Result;
use chrono::NaiveDate;

use rust_stocks::api::{SchwabClient, StockDataProvider};
use rust_stocks::models::Config;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 Testing if we can actually fetch stock data...");
    
    // Load config
    let config = Config::from_env()?;
    println!("✅ Config loaded");
    
    // Create client
    let client = SchwabClient::new(&config)?;
    println!("✅ API client created");
    
    // Test with AAPL for a small date range
    let start_date = NaiveDate::from_ymd_opt(2024, 8, 1).unwrap();
    let end_date = NaiveDate::from_ymd_opt(2024, 8, 30).unwrap();
    
    println!("📈 Fetching AAPL data from {} to {}...", start_date, end_date);
    
    match client.get_price_history("AAPL", start_date, end_date).await {
        Ok(price_bars) => {
            println!("✅ SUCCESS! Got {} price records for AAPL", price_bars.len());
            
            if !price_bars.is_empty() {
                println!("\n📊 Sample data:");
                for (i, bar) in price_bars.iter().take(5).enumerate() {
                    let date = chrono::DateTime::from_timestamp(bar.datetime / 1000, 0)
                        .unwrap()
                        .date_naive();
                    println!("  {}. {}: Open=${:.2}, High=${:.2}, Low=${:.2}, Close=${:.2}", 
                             i+1, date, bar.open, bar.high, bar.low, bar.close);
                }
                if price_bars.len() > 5 {
                    println!("  ... and {} more records", price_bars.len() - 5);
                }
            }
        }
        Err(e) => {
            println!("❌ FAILED to fetch data: {}", e);
            return Err(e);
        }
    }
    
    println!("\n🎉 Test completed successfully - the system CAN fetch stock data!");
    Ok(())
}