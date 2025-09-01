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
pub struct StockDetail {
    pub stock: Stock,
    pub current_price: DailyPrice,
    pub price_history: Vec<DailyPrice>,
    pub pe_trend: Vec<(NaiveDate, f64)>,
    pub volume_trend: Vec<(NaiveDate, i64)>,
}

/// Schwab API quote response structure
#[derive(Debug, Deserialize)]
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

/// Date range for data analysis
#[derive(Debug, Clone, PartialEq)]
pub struct DateRange {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

impl DateRange {
    pub fn new(start: NaiveDate, end: NaiveDate) -> Self {
        Self { start, end }
    }
    
    pub fn days_count(&self) -> i64 {
        (self.end - self.start).num_days() + 1
    }
}

/// Database statistics for dashboard
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    pub total_stocks: usize,
    pub total_price_records: usize,
    pub data_coverage_percentage: f64,
    pub last_update_date: Option<NaiveDate>,
    pub oldest_data_date: Option<NaiveDate>,
}

/// Collection progress metrics
#[derive(Debug, Clone)]
pub struct CollectionProgress {
    pub stocks_with_data: usize,
    pub stocks_missing_data: usize,
    pub target_start_date: NaiveDate, // Jan 1, 2020
    pub completion_percentage: f64,
    pub estimated_records_remaining: usize,
}

/// Stock collection status for data collection view
#[derive(Debug, Clone)]
pub struct StockCollectionStatus {
    pub symbol: String,
    pub company_name: String,
    pub status: CollectionStatus,
    pub date_range: Option<(NaiveDate, NaiveDate)>,
    pub record_count: usize,
    pub progress_percentage: f64,
}

/// Collection status enumeration
#[derive(Debug, Clone)]
pub enum CollectionStatus {
    NotStarted,
    InProgress { current_date: NaiveDate },
    Completed,
    Failed { error: String },
    PartialData { gaps: Vec<DateRange> },
}

/// Collection mode for data collection
#[derive(Debug, Clone, PartialEq)]
pub enum CollectionMode {
    FullHistorical,    // Jan 1, 2020 to today for all stocks
    IncrementalUpdate, // Latest data only
    CustomRange {      // User-specified date range
        start: NaiveDate,
        end: NaiveDate,
    },
    SingleStock {      // Individual stock collection
        symbol: String,
        start: NaiveDate,
        end: NaiveDate,
    },
}

/// Data coverage analysis for stocks
#[derive(Debug, Clone)]
pub struct DataCoverage {
    pub earliest_date: Option<NaiveDate>,
    pub latest_date: Option<NaiveDate>,
    pub total_records: usize,
    pub missing_ranges: Vec<DateRange>,
    pub coverage_percentage: f64,
}

/// Stock metrics for analysis
#[derive(Debug, Clone)]
pub struct StockMetrics {
    pub pe_ratio_trend: Option<PETrend>,
    pub price_performance: PricePerformance,
    pub volatility_metrics: VolatilityMetrics,
}

/// P/E ratio trend analysis
#[derive(Debug, Clone)]
pub struct PETrend {
    pub current_pe: Option<f64>,
    pub year_ago_pe: Option<f64>,
    pub decline_percentage: f64,
    pub trend_direction: TrendDirection,
}

/// Price performance metrics
#[derive(Debug, Clone)]
pub struct PricePerformance {
    pub current_price: f64,
    pub year_ago_price: f64,
    pub change_percentage: f64,
    pub ytd_return: f64,
    pub volatility: f64,
}

/// Volatility metrics
#[derive(Debug, Clone)]
pub struct VolatilityMetrics {
    pub daily_volatility: f64,
    pub monthly_volatility: f64,
    pub beta: Option<f64>,
    pub sharpe_ratio: Option<f64>,
}

/// Trend direction enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
    Unknown,
}

/// Overall progress tracking
#[derive(Debug, Clone)]
pub struct OverallProgress {
    pub target_start_date: NaiveDate, // Jan 1, 2020
    pub total_target_records: usize,   // ~1.5M records
    pub current_records: usize,
    pub completion_percentage: f64,
    pub stocks_completed: usize,       // 100% data from Jan 1, 2020
    pub stocks_partial: usize,         // Some data but gaps
    pub stocks_missing: usize,         // No data at all
}

/// Individual stock progress
#[derive(Debug, Clone)]
pub struct StockProgress {
    pub stock: Stock,
    pub data_range: Option<(NaiveDate, NaiveDate)>,
    pub record_count: usize,
    pub expected_records: usize,
    pub missing_ranges: Vec<DateRange>,
    pub priority_score: f64, // Higher = needs attention
}

/// Data gap information
#[derive(Debug, Clone)]
pub struct DataGap {
    pub symbol: String,
    pub missing_range: DateRange,
    pub missing_days: usize,
    pub priority_score: f64,
}

/// Gap analysis summary
#[derive(Debug, Clone)]
pub struct GapAnalysis {
    pub total_missing_days: usize,
    pub largest_gaps: Vec<DataGap>,
    pub stocks_needing_attention: Vec<String>,
    pub estimated_collection_time: std::time::Duration,
}

/// Stock data statistics for UI display
#[derive(Debug, Clone)]
pub struct StockDataStats {
    pub data_points: usize,
    pub earliest_date: Option<NaiveDate>,
    pub latest_date: Option<NaiveDate>,
}

