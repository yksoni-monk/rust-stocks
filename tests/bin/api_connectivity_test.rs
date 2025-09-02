use anyhow::Result;
use clap::{Parser, Subcommand};
use rust_stocks::api::{SchwabClient, StockDataProvider};
use rust_stocks::models::Config;
use tracing::{info, error, Level};
use tracing_subscriber::{self, FmtSubscriber};

/// Comprehensive API connectivity testing tool
#[derive(Parser)]
#[command(name = "api_connectivity_test")]
#[command(version = "1.0.0")]
#[command(about = "Test Schwab API connectivity and functionality")]
#[command(long_about = "
This tool provides comprehensive testing of the Schwab API integration:
- Authentication testing
- Quote fetching for multiple stocks
- Price history fetching
- Error handling validation

Examples:
  cargo run --bin api_connectivity_test auth
  cargo run --bin api_connectivity_test quotes
  cargo run --bin api_connectivity_test history AAPL 20240101 20240131
")]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Test API authentication
    Auth,
    
    /// Test quote fetching for major stocks
    Quotes,
    
    /// Test price history fetching
    History {
        /// Stock symbol
        symbol: String,
        /// Start date in YYYYMMDD format
        start_date: String,
        /// End date in YYYYMMDD format
        end_date: String,
    },
    
    /// Run all connectivity tests
    All,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    let args = Args::parse();

    match args.command {
        Commands::Auth => {
            test_authentication().await?;
        }
        Commands::Quotes => {
            test_quotes().await?;
        }
        Commands::History { symbol, start_date, end_date } => {
            test_price_history(&symbol, &start_date, &end_date).await?;
        }
        Commands::All => {
            test_all().await?;
        }
    }

    Ok(())
}

async fn test_authentication() -> Result<()> {
    info!("🔐 Testing API Authentication");
    info!("============================");

    // Load configuration
    let config = Config::from_env()?;
    info!("📋 Configuration loaded");

    // Initialize Schwab client
    match SchwabClient::new(&config) {
        Ok(_client) => {
            info!("✅ Schwab client initialized successfully");
            info!("✅ Authentication test passed!");
        }
        Err(e) => {
            error!("❌ Failed to initialize Schwab client: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

async fn test_quotes() -> Result<()> {
    info!("📊 Testing Quote Fetching");
    info!("========================");

    // Load configuration and create client
    let config = Config::from_env()?;
    let client = SchwabClient::new(&config)?;

    // Test API call - fetch quotes for major stocks
    let symbols = vec!["AAPL".to_string(), "MSFT".to_string(), "GOOGL".to_string()];
    
    info!("🔍 Testing get_quotes for: {:?}", symbols);
    
    match client.get_quotes(&symbols).await {
        Ok(quotes) => {
            info!("✅ Successfully fetched {} quotes", quotes.len());
            for quote in quotes {
                info!("📊 {}: ${:.2} (P/E: {:?})", 
                      quote.symbol, 
                      quote.last_price,
                      quote.pe_ratio);
            }
            info!("✅ Quote fetching test passed!");
        }
        Err(e) => {
            error!("❌ Failed to fetch quotes: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

async fn test_price_history(symbol: &str, start_str: &str, end_str: &str) -> Result<()> {
    info!("📈 Testing Price History Fetching");
    info!("=================================");

    // Parse dates
    let start_date = parse_date(start_str)?;
    let end_date = parse_date(end_str)?;

    info!("📅 Fetching {} data from {} to {}", symbol, start_date, end_date);

    // Load configuration and create client
    let config = Config::from_env()?;
    let client = SchwabClient::new(&config)?;

    match client.get_price_history(symbol, start_date, end_date).await {
        Ok(price_bars) => {
            info!("✅ Successfully fetched {} price records for {}", price_bars.len(), symbol);
            
            if !price_bars.is_empty() {
                info!("📊 Sample data:");
                for (i, bar) in price_bars.iter().take(3).enumerate() {
                    let date = chrono::DateTime::from_timestamp(bar.datetime / 1000, 0)
                        .unwrap()
                        .date_naive();
                    info!("  {}. {}: Open=${:.2}, High=${:.2}, Low=${:.2}, Close=${:.2}, Volume={}", 
                          i+1, date, bar.open, bar.high, bar.low, bar.close, bar.volume);
                }
                if price_bars.len() > 3 {
                    info!("  ... and {} more records", price_bars.len() - 3);
                }
            }
            info!("✅ Price history test passed!");
        }
        Err(e) => {
            error!("❌ Failed to fetch price history: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

async fn test_all() -> Result<()> {
    info!("🧪 Running All API Connectivity Tests");
    info!("=====================================");

    // Test 1: Authentication
    info!("\n📋 Test 1: Authentication");
    test_authentication().await?;

    // Test 2: Quotes
    info!("\n📋 Test 2: Quote Fetching");
    test_quotes().await?;

    // Test 3: Price History (AAPL for last month)
    info!("\n📋 Test 3: Price History Fetching");
    let end_date = chrono::Utc::now().date_naive();
    let start_date = end_date - chrono::Duration::days(30);
    
    test_price_history("AAPL", &start_date.format("%Y%m%d").to_string(), &end_date.format("%Y%m%d").to_string()).await?;

    info!("\n🎉 All API connectivity tests completed successfully!");
    Ok(())
}

fn parse_date(date_str: &str) -> Result<chrono::NaiveDate> {
    if date_str.len() != 8 {
        return Err(anyhow::anyhow!("Date must be YYYYMMDD format"));
    }
    
    let year: i32 = date_str[0..4].parse()?;
    let month: u32 = date_str[4..6].parse()?;
    let day: u32 = date_str[6..8].parse()?;
    
    chrono::NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| anyhow::anyhow!("Invalid date"))
}
