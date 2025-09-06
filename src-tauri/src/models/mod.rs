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
    pub alpha_vantage_api_key: String,
    pub database_path: String,
    pub rate_limit_per_minute: u32,
    #[allow(dead_code)]
    pub batch_size: usize,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok(); // Load .env file if it exists
        
        // Debug: Print all environment variables
        println!("DEBUG: Current working directory: {:?}", std::env::current_dir());
        println!("DEBUG: SCHWAB_API_KEY: {:?}", std::env::var("SCHWAB_API_KEY"));
        println!("DEBUG: SCHWAB_APP_SECRET: {:?}", std::env::var("SCHWAB_APP_SECRET"));
        println!("DEBUG: SCHWAB_CALLBACK_URL: {:?}", std::env::var("SCHWAB_CALLBACK_URL"));
        println!("DEBUG: SCHWAB_TOKEN_PATH: {:?}", std::env::var("SCHWAB_TOKEN_PATH"));
        println!("DEBUG: ALPHA_VANTAGE_API_KEY: {:?}", std::env::var("ALPHA_VANTAGE_API_KEY"));
        println!("DEBUG: DATABASE_PATH: {:?}", std::env::var("DATABASE_PATH"));
        
        let schwab_token_path = std::env::var("SCHWAB_TOKEN_PATH")
            .unwrap_or_else(|_| "schwab_tokens.json".to_string());
        
        println!("DEBUG: Final token path: {}", schwab_token_path);
        println!("DEBUG: Token file exists: {}", std::path::Path::new(&schwab_token_path).exists());
        
        Ok(Config {
            schwab_api_key: std::env::var("SCHWAB_API_KEY")
                .map_err(|_| anyhow::anyhow!("SCHWAB_API_KEY environment variable required"))?,
            schwab_app_secret: std::env::var("SCHWAB_APP_SECRET")
                .map_err(|_| anyhow::anyhow!("SCHWAB_APP_SECRET environment variable required"))?,
            schwab_callback_url: std::env::var("SCHWAB_CALLBACK_URL")
                .unwrap_or_else(|_| "https://localhost:8080".to_string()),
            schwab_token_path,
            alpha_vantage_api_key: std::env::var("ALPHA_VANTAGE_API_KEY")
                .unwrap_or_else(|_| "demo".to_string()),
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

// ============================================================================
// Enhanced Data Models for Comprehensive Stock Analysis
// ============================================================================

// Enhanced stock information with comprehensive company data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockInfoEnhanced {
    pub id: i64,
    pub symbol: String,
    pub company_name: String,
    pub exchange: Option<String>,
    pub sector: Option<String>,
    pub industry: Option<String>,
    pub market_cap: Option<f64>,
    pub description: Option<String>,
    pub employees: Option<i32>,
    pub founded_year: Option<i32>,
    pub headquarters: Option<String>,
    pub website: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Enhanced daily price data with comprehensive fundamental metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedPriceData {
    pub id: i64,
    pub stock_id: i64,
    pub date: String,
    pub open_price: f64,
    pub high_price: f64,
    pub low_price: f64,
    pub close_price: f64,
    pub adjusted_close: Option<f64>,
    pub volume: Option<i64>,
    pub average_volume: Option<i64>,
    
    // Fundamental ratios
    pub pe_ratio: Option<f64>,
    pub pe_ratio_forward: Option<f64>,
    pub pb_ratio: Option<f64>,
    pub ps_ratio: Option<f64>,
    pub dividend_yield: Option<f64>,
    pub dividend_per_share: Option<f64>,
    pub eps: Option<f64>,
    pub eps_forward: Option<f64>,
    pub beta: Option<f64>,
    
    // 52-week data
    pub week_52_high: Option<f64>,
    pub week_52_low: Option<f64>,
    pub week_52_change_percent: Option<f64>,
    
    // Market metrics
    pub shares_outstanding: Option<f64>,
    pub float_shares: Option<f64>,
    pub revenue_ttm: Option<f64>,
    pub profit_margin: Option<f64>,
    pub operating_margin: Option<f64>,
    pub return_on_equity: Option<f64>,
    pub return_on_assets: Option<f64>,
    pub debt_to_equity: Option<f64>,
    
    pub created_at: DateTime<Utc>,
}

// Fundamental data structure for Schwab API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundamentalData {
    pub symbol: String,
    pub pe_ratio: Option<f64>,
    pub pe_ratio_forward: Option<f64>,
    pub market_cap: Option<f64>,
    pub dividend_yield: Option<f64>,
    pub dividend_per_share: Option<f64>,
    pub eps: Option<f64>,
    pub eps_forward: Option<f64>,
    pub beta: Option<f64>,
    pub week_52_high: Option<f64>,
    pub week_52_low: Option<f64>,
    pub pb_ratio: Option<f64>,
    pub ps_ratio: Option<f64>,
    pub shares_outstanding: Option<f64>,
    pub float_shares: Option<f64>,
    pub revenue_ttm: Option<f64>,
    pub profit_margin: Option<f64>,
    pub operating_margin: Option<f64>,
    pub return_on_equity: Option<f64>,
    pub return_on_assets: Option<f64>,
    pub debt_to_equity: Option<f64>,
}

// Real-time quote data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealTimeQuote {
    pub id: Option<i64>,
    pub stock_id: i64,
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub bid_price: Option<f64>,
    pub bid_size: Option<i32>,
    pub ask_price: Option<f64>,
    pub ask_size: Option<i32>,
    pub last_price: f64,
    pub last_size: Option<i32>,
    pub volume: Option<i64>,
    pub change_amount: Option<f64>,
    pub change_percent: Option<f64>,
    pub day_high: Option<f64>,
    pub day_low: Option<f64>,
}

// Intraday price data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntradayPrice {
    pub id: Option<i64>,
    pub stock_id: i64,
    pub datetime: DateTime<Utc>,
    pub interval_type: String, // '1min', '5min', '15min', '30min', '1hour'
    pub open_price: f64,
    pub high_price: f64,
    pub low_price: f64,
    pub close_price: f64,
    pub volume: Option<i64>,
}

// Option chain data with Greeks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionData {
    pub id: Option<i64>,
    pub stock_id: i64,
    pub symbol: String,
    pub expiration_date: NaiveDate,
    pub strike_price: f64,
    pub option_type: String, // 'CALL' or 'PUT'
    pub bid: Option<f64>,
    pub ask: Option<f64>,
    pub last_price: Option<f64>,
    pub volume: Option<i64>,
    pub open_interest: Option<i64>,
    pub implied_volatility: Option<f64>,
    pub delta: Option<f64>,
    pub gamma: Option<f64>,
    pub theta: Option<f64>,
    pub vega: Option<f64>,
    pub rho: Option<f64>,
}

// Comprehensive stock data combining all data types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensiveStockData {
    pub stock_info: StockInfoEnhanced,
    pub price_data: Vec<EnhancedPriceData>,
    pub fundamentals: Option<FundamentalData>,
    pub real_time_quote: Option<RealTimeQuote>,
    pub intraday_data: Vec<IntradayPrice>,
    pub options_data: Vec<OptionData>,
}

// API response structures for different endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
}

// Data fetch request configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchRequest {
    pub symbol: String,
    pub start_date: String,
    pub end_date: String,
    pub include_fundamentals: bool,
    pub include_real_time: bool,
    pub include_intraday: bool,
    pub include_options: bool,
    pub intraday_interval: Option<String>,
}

