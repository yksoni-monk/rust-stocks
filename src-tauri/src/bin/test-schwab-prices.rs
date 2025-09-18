/// Test Schwab API historical price data retrieval
/// This tool tests the Schwab API's ability to fetch historical OHLCV data
/// for S&P 500 stocks covering the timeframe of our EDGAR financial data.

use anyhow::Result;
use chrono::NaiveDate;
use rust_stocks_tauri_lib::api::schwab_client::SchwabClient;
use rust_stocks_tauri_lib::api::StockDataProvider;
use rust_stocks_tauri_lib::models::Config;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("🧪 Schwab API Historical Price Data Test");
    println!("========================================");
    
    // Load configuration
    println!("📋 Loading configuration...");
    let config = match Config::from_env() {
        Ok(config) => {
            println!("✅ Configuration loaded successfully");
            config
        }
        Err(e) => {
            println!("❌ Failed to load configuration: {}", e);
            println!("💡 Make sure these environment variables are set:");
            println!("   - SCHWAB_API_KEY");
            println!("   - SCHWAB_APP_SECRET");
            println!("   - SCHWAB_TOKEN_PATH (optional, defaults to schwab_tokens.json)");
            return Err(e);
        }
    };
    
    // Initialize Schwab client
    println!("🔌 Initializing Schwab API client...");
    let schwab_client = SchwabClient::new(&config)?;
    println!("✅ Schwab client initialized");
    
    // Test with Apple (AAPL) - representative S&P 500 stock
    let test_symbol = "AAPL";
    let start_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
    let end_date = NaiveDate::from_ymd_opt(2023, 12, 31).unwrap();
    
    println!("📈 Testing price history for {} from {} to {}", test_symbol, start_date, end_date);
    
    match schwab_client.get_price_history(test_symbol, start_date, end_date).await {
        Ok(price_bars) => {
            println!("✅ Successfully retrieved {} price bars for {}", price_bars.len(), test_symbol);
            
            if !price_bars.is_empty() {
                let first_bar = &price_bars[0];
                let last_bar = &price_bars[price_bars.len() - 1];
                
                println!("📊 Sample data:");
                println!("   First bar: Date: {}, Open: ${:.2}, High: ${:.2}, Low: ${:.2}, Close: ${:.2}, Volume: {}",
                         chrono::DateTime::from_timestamp_millis(first_bar.datetime)
                             .map(|dt| dt.format("%Y-%m-%d").to_string())
                             .unwrap_or_else(|| "Unknown".to_string()),
                         first_bar.open, first_bar.high, first_bar.low, first_bar.close, first_bar.volume);
                         
                println!("   Last bar:  Date: {}, Open: ${:.2}, High: ${:.2}, Low: ${:.2}, Close: ${:.2}, Volume: {}",
                         chrono::DateTime::from_timestamp_millis(last_bar.datetime)
                             .map(|dt| dt.format("%Y-%m-%d").to_string())
                             .unwrap_or_else(|| "Unknown".to_string()),
                         last_bar.open, last_bar.high, last_bar.low, last_bar.close, last_bar.volume);
                
                // Calculate expected trading days for 2023 (approximately 252 trading days)
                let expected_days = 252;
                let coverage_percentage = (price_bars.len() as f64 / expected_days as f64) * 100.0;
                
                println!("📈 Data Quality Assessment:");
                println!("   Expected trading days in 2023: ~{}", expected_days);
                println!("   Actual price bars received: {}", price_bars.len());
                println!("   Coverage: {:.1}%", coverage_percentage);
                
                if coverage_percentage >= 95.0 {
                    println!("✅ Excellent data coverage!");
                } else if coverage_percentage >= 80.0 {
                    println!("⚠️ Good data coverage, some gaps may exist");
                } else {
                    println!("❌ Poor data coverage, significant gaps detected");
                }
            }
            
            println!("🎯 S&P 500 Projection:");
            let sp500_count = 503;
            let estimated_total_bars = price_bars.len() * sp500_count;
            let estimated_time_minutes = sp500_count / 120; // 120 requests per minute rate limit
            
            println!("   S&P 500 stocks: {}", sp500_count);
            println!("   Estimated total price bars: {}", estimated_total_bars);
            println!("   Estimated download time: ~{} minutes", estimated_time_minutes);
            
            println!("✅ Schwab API test completed successfully!");
            println!("💡 Ready to proceed with bulk S&P 500 price data download");
        }
        Err(e) => {
            println!("❌ Failed to retrieve price history: {}", e);
            println!("🔍 Troubleshooting steps:");
            println!("   1. Check if Schwab API tokens are valid (run refresh_token.py)");
            println!("   2. Verify SCHWAB_TOKEN_PATH points to valid token file");
            println!("   3. Confirm API credentials have market data permissions");
            println!("   4. Check internet connectivity");
            return Err(e);
        }
    }
    
    Ok(())
}