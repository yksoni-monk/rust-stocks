use anyhow::Result;
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

    info!("🧪 Testing Schwab API connectivity");

    // Load configuration
    let config = Config::from_env()?;
    info!("📋 Configuration loaded");

    // Initialize Schwab client
    let schwab_client = SchwabClient::new(&config)?;
    info!("🔑 Schwab client initialized");

    // Test API call - fetch quotes for a few major stocks
    let symbols = vec!["AAPL".to_string(), "MSFT".to_string(), "GOOGL".to_string()];
    
    info!("🔍 Testing get_quotes for: {:?}", symbols);
    
    match schwab_client.get_quotes(&symbols).await {
        Ok(quotes) => {
            info!("✅ Successfully fetched {} quotes", quotes.len());
            for quote in quotes {
                info!("📊 {}: ${:.2} (P/E: {:?})", 
                      quote.symbol, 
                      quote.last_price,
                      quote.pe_ratio);
            }
        }
        Err(e) => {
            info!("❌ Failed to fetch quotes: {}", e);
            return Err(e);
        }
    }

    info!("🎉 API connectivity test completed successfully!");
    Ok(())
}