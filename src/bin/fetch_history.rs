use anyhow::Result;
use chrono::{NaiveDate, Utc};
use rust_stocks::api::{SchwabClient, StockDataProvider};
use rust_stocks::models::Config;
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    info!("📈 Fetching AAPL historical data from Jan 1, 2020 to today");

    // Load configuration
    let config = Config::from_env()?;
    
    // Initialize Schwab client
    let schwab_client = SchwabClient::new(&config)?;
    
    // Define date range
    let start_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    let end_date = Utc::now().date_naive();
    
    info!("📅 Fetching data from {} to {}", start_date, end_date);
    
    // Fetch historical price data for AAPL
    match schwab_client.get_price_history("AAPL", start_date, end_date).await {
        Ok(price_bars) => {
            info!("✅ Successfully fetched {} price bars for AAPL", price_bars.len());
            
            // Display first few and last few records
            println!("\n📊 AAPL Historical Closing Prices:");
            println!("Date           | Close Price");
            println!("---------------|------------");
            
            let display_count = std::cmp::min(5, price_bars.len());
            
            // Show first few records
            for (i, bar) in price_bars.iter().take(display_count).enumerate() {
                // Convert milliseconds to seconds for proper timestamp parsing
                let date = chrono::DateTime::from_timestamp(bar.datetime / 1000, 0)
                    .unwrap_or_else(|| Utc::now())
                    .date_naive();
                println!("{} | ${:.2}", date, bar.close);
                if i == display_count - 1 && price_bars.len() > display_count * 2 {
                    println!("...            | ...");
                }
            }
            
            // Show last few records if we have more than 10 total
            if price_bars.len() > 10 {
                let skip_count = price_bars.len() - 5;
                for bar in price_bars.iter().skip(skip_count) {
                    let date = chrono::DateTime::from_timestamp(bar.datetime / 1000, 0)
                        .unwrap_or_else(|| Utc::now())
                        .date_naive();
                    println!("{} | ${:.2}", date, bar.close);
                }
            }
            
            // Calculate some basic statistics
            if !price_bars.is_empty() {
                let first_price = price_bars.first().unwrap().close;
                let last_price = price_bars.last().unwrap().close;
                let total_return = ((last_price - first_price) / first_price) * 100.0;
                
                let max_price = price_bars.iter().map(|b| b.close).fold(f64::NEG_INFINITY, f64::max);
                let min_price = price_bars.iter().map(|b| b.close).fold(f64::INFINITY, f64::min);
                
                println!("\n📈 Statistics:");
                println!("First Price:    ${:.2}", first_price);
                println!("Last Price:     ${:.2}", last_price);
                println!("Total Return:   {:.1}%", total_return);
                println!("Highest Price:  ${:.2}", max_price);
                println!("Lowest Price:   ${:.2}", min_price);
                println!("Total Records:  {}", price_bars.len());
            }
            
        }
        Err(e) => {
            info!("❌ Failed to fetch historical data: {}", e);
            return Err(e);
        }
    }

    info!("🎉 Historical data fetch completed successfully!");
    Ok(())
}