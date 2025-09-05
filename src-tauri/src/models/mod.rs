use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// Core stock information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stock {
    pub id: Option<i64>,
    pub symbol: String,
    pub company_name: String,
    pub sector: Option<String>,
    pub industry: Option<String>,
    pub market_cap: Option<f64>,
    pub status: StockStatus,
    pub first_trading_date: Option<NaiveDate>,
    pub last_updated: Option<DateTime<Utc>>,
}

/// Stock status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StockStatus {
    Active,
    Delisted,
    Suspended,
}

impl Default for StockStatus {
    fn default() -> Self {
        StockStatus::Active
    }
}

/// Daily price and fundamental data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyPrice {
    pub id: Option<i64>,
    pub stock_id: i64,
    pub date: NaiveDate,
    pub open_price: f64,
    pub high_price: f64,
    pub low_price: f64,
    pub close_price: f64,
    pub volume: Option<i64>,
    pub pe_ratio: Option<f64>,
    pub market_cap: Option<f64>,
    pub dividend_yield: Option<f64>,
}

/// Stock analysis result for P/E decline ranking
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StockAnalysis {
    pub stock: Stock,
    pub current_price: f64,
    pub current_pe: Option<f64>,
    pub year_ago_pe: Option<f64>,
    pub pe_decline_percent: f64,
    pub price_change_percent: f64,
}

/// Detailed stock information for UI display
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StockDetail {
    pub stock: Stock,
    pub current_price: DailyPrice,
    pub price_history: Vec<DailyPrice>,
    pub pe_trend: Vec<(NaiveDate, f64)>,
    pub volume_trend: Vec<(NaiveDate, i64)>,
}

/// Schwab API quote response structure
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct SchwabQuote {
    pub symbol: String,
    #[serde(rename = "lastPrice")]
    pub last_price: f64,
    #[serde(rename = "openPrice")]
    pub open_price: Option<f64>,
    #[serde(rename = "highPrice")]
    pub high_price: Option<f64>,
    #[serde(rename = "lowPrice")]
    pub low_price: Option<f64>,
    #[serde(rename = "closePrice")]
    pub close_price: Option<f64>,
    pub volume: Option<i64>,
    #[serde(rename = "peRatio")]
    pub pe_ratio: Option<f64>,
    #[serde(rename = "marketCap")]
    pub market_cap: Option<f64>,
    #[serde(rename = "divYield")]
    pub dividend_yield: Option<f64>,
}

/// Schwab API price history bar
#[derive(Debug, Deserialize)]
pub struct SchwabPriceBar {
    #[serde(rename = "datetime")]
    pub datetime: i64, // Unix timestamp
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: i64,
}

/// System metadata for tracking state
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SystemMetadata {
    pub key: String,
    pub value: String,
    pub updated_at: DateTime<Utc>,
}

/// Configuration for the application
#[derive(Debug, Clone)]
pub struct Config {
    pub schwab_api_key: String,
    pub schwab_app_secret: String,
    pub schwab_callback_url: String,
    pub schwab_token_path: String,
    pub database_path: String,
    pub rate_limit_per_minute: u32,
    #[allow(dead_code)]
    pub batch_size: usize,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok(); // Load .env file if it exists
        
        Ok(Config {
            schwab_api_key: std::env::var("SCHWAB_API_KEY")
                .map_err(|_| anyhow::anyhow!("SCHWAB_API_KEY environment variable required"))?,
            schwab_app_secret: std::env::var("SCHWAB_APP_SECRET")
                .map_err(|_| anyhow::anyhow!("SCHWAB_APP_SECRET environment variable required"))?,
            schwab_callback_url: std::env::var("SCHWAB_CALLBACK_URL")
                .unwrap_or_else(|_| "https://localhost:8080".to_string()),
            schwab_token_path: std::env::var("SCHWAB_TOKEN_PATH")
                .unwrap_or_else(|_| "schwab_tokens.json".to_string()),
            database_path: std::env::var("DATABASE_PATH")
                .unwrap_or_else(|_| "stocks.db".to_string()),
            rate_limit_per_minute: std::env::var("RATE_LIMIT_PER_MINUTE")
                .unwrap_or_else(|_| "120".to_string())
                .parse()
                .unwrap_or(120),
            batch_size: std::env::var("BATCH_SIZE")
                .unwrap_or_else(|_| "50".to_string())
                .parse()
                .unwrap_or(50),
        })
    }
}

// ============================================================================
// Enhanced TUI Application Models
// ============================================================================

// DateRange moved to concurrent_fetcher.rs to avoid duplication

// DatabaseStats moved to ui/dashboard.rs to avoid duplication

// Collection progress and status structs removed - unused in current implementation

// Complex analysis metrics removed - unused in current implementation

// Progress tracking structs removed - unused in current implementation

/// Stock data statistics for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockDataStats {
    pub data_points: usize,
    pub earliest_date: Option<NaiveDate>,
    pub latest_date: Option<NaiveDate>,
}

/// Database statistics for dashboard and analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub total_stocks: usize,
    pub total_price_records: usize,
    pub data_coverage_percentage: f64,
    pub oldest_data_date: Option<NaiveDate>,
    pub last_update_date: Option<NaiveDate>,
    pub top_pe_decliner: Option<(String, f64)>, // (symbol, decline_percent) - for analysis views
}

