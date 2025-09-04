# Stock Analysis System - Product Requirements Document (PRD)

## Executive Summary
A high-performance Rust-based stock analysis system that fetches, stores, and analyzes S&P 500 stock data using the Charles Schwab API. Features concurrent data collection, intelligent incremental updates, P/E ratio trend analysis, and an interactive terminal-based UI.

## Product Requirements

### Core Data Management
1. **Data Collection**
   - Fetch all S&P 500 constituents using Schwab API
   - Collect daily OHLC data, P/E ratios, volume, market cap
   - Start date: January 1, 2020
   - Concurrent fetching for all 500+ stocks

2. **Data Storage** 
   - SQLite database for local persistence
   - Incremental updates: only fetch data since last update
   - Track last update timestamp per stock
   - Handle delisted stocks with status flags

3. **Data Freshness Management**
   - Daily update frequency
   - Skip weekends/holidays (market closed days)
   - Graceful handling of data gaps
   - Resume from last successful update point

### Analysis Features
1. **P/E Ratio Analysis**
   - Calculate 1-year P/E ratio decline for all stocks
   - Rank top 10 stocks with maximum P/E decline
   - Support pagination (next 10, previous 10)
   - Real-time P/E change calculations

2. **Stock Search & Discovery**
   - Search by ticker symbol (exact or partial)
   - Search by company name (fuzzy matching)
   - Auto-complete suggestions
   - Historical lookup for delisted stocks

### User Interface (Ratatui TUI)
1. **Main Dashboard**
   - Top 10 P/E decliners list
   - Real-time data freshness status  
   - Quick search interface

2. **Interactive Navigation**
   - Arrow key navigation
   - Stock detail drill-down
   - Pagination controls
   - Search filters

3. **Stock Detail View**
   - Historical price charts (ASCII/Unicode)
   - Key financial metrics
   - P/E trend analysis
   - Trading volume patterns

## Technical Architecture

### System Components

```
┌──────────────────────────────────────────────────────────────┐
│                     Stock Analysis System                   │
├──────────────────────────────────────────────────────────────┤
│  [Ratatui UI] ←→ [Analysis Engine] ←→ [Database Manager]    │
│                        ↓                     ↓               │
│              [Data Collector] ←→ [Market Calendar]          │
│                        ↓                     ↓               │
│              [Concurrent Fetcher] ←→ [Schwab API Client]    │
└──────────────────────────────────────────────────────────────┘
```

### Concurrent Data Fetching Architecture

#### Overview
The system now supports concurrent fetching of stock data using multiple worker threads. Each thread processes one stock at a time, with thread-safe coordination to avoid conflicts.

#### Core Components

1. **Concurrent Fetcher** (`src/concurrent_fetcher.rs`)
   - Main orchestrator for parallel stock data fetching
   - Uses `tokio::sync::mpsc` for thread communication
   - Uses `std::sync::Arc<Mutex<Vec<Stock>>>` for thread-safe stock queue
   - Manages worker thread pool and progress tracking

2. **Stock Queue Manager** (built into Concurrent Fetcher)
   - Thread-safe queue of stocks to process
   - Each thread claims next available stock from ordered list
   - Uses existing `get_active_stocks()` ordered by symbol
   - Prevents multiple threads from working on same stock

3. **Worker Thread Function**
   - Each thread processes one stock at a time
   - Uses existing `count_existing_records()` to check data existence
   - Uses existing `SchwabClient` for API calls
   - Uses existing `ApiRateLimiter` per thread (simplest approach)
   - Implements retry logic with configurable attempts

4. **Progress Tracking**
   - Uses `tokio::sync::broadcast` for real-time progress updates
   - Each thread reports: "Thread X: Processing SYMBOL", "Thread X: Completed SYMBOL"
   - Supports error reporting and skip notifications

#### Data Structures

```rust
pub struct ConcurrentFetchConfig {
    pub date_range: DateRange,
    pub num_threads: usize,
    pub retry_attempts: u32,
}

pub struct DateRange {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

pub struct FetchProgress {
    pub thread_id: usize,
    pub stock_symbol: String,
    pub status: FetchStatus,
    pub message: String,
}

pub enum FetchStatus {
    Started,
    Skipped, // Data already exists
    Completed,
    Failed(String),
}
```

#### Thread Safety Design

1. **Database Operations**: 
   - Database operations are already thread-safe (uses `Arc<Mutex<Connection>>`)
   - Each thread gets its own `SchwabClient` with its own `ApiRateLimiter`
   - Stock queue uses `Arc<Mutex<Vec<Stock>>>` for thread-safe access

2. **Rate Limiting**: 
   - Each thread has its own `ApiRateLimiter` instance
   - No global coordination needed (simplest approach)
   - Prevents API rate limit violations

3. **Error Handling**:
   - Retry logic built into `SchwabClient` (already exists)
   - Thread reports error and moves to next stock
   - Configurable retry attempts per stock

4. **Progress Tracking**:
   - Uses `tokio::sync::broadcast` channel
   - Main thread can listen to progress updates
   - Real-time status reporting

#### Function Signature
```rust
pub async fn fetch_stocks_concurrently(
    database: Arc<DatabaseManager>,
    config: ConcurrentFetchConfig,
) -> Result<FetchResult>
```

#### Usage Example
```rust
let config = ConcurrentFetchConfig {
    date_range: DateRange {
        start_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
        end_date: NaiveDate::from_ymd_opt(2025, 8, 31).unwrap(),
    },
    num_threads: 10,
    retry_attempts: 3,
};

let result = fetch_stocks_concurrently(database, config).await?;
```

### Core Modules

#### 1. Schwab API Client (`schwab_client.rs`)
```rust
struct SchwabClient {
    api_key: String,
    secret: String,
    access_token: String,
    client: reqwest::Client,
    rate_limiter: RateLimiter,
}

impl SchwabClient {
    async fn get_sp500_symbols() -> Result<Vec<String>>
    async fn get_quotes(symbols: Vec<String>) -> Result<Vec<Quote>>  
    async fn get_price_history(symbol: String, from: Date, to: Date) -> Result<Vec<PriceBar>>
    async fn get_fundamentals(symbol: String) -> Result<Fundamentals>
    async fn get_market_hours(market: &str) -> Result<Value>
    async fn get_market_hours_for_date(market: &str, date: &str) -> Result<Value>
}
```

#### 2. Database Manager (`database.rs`)
```rust
struct DatabaseManager {
    connection: Arc<Mutex<rusqlite::Connection>>,
}

// Tables:
// - stocks (id, symbol, company_name, sector, status, last_updated)
// - daily_prices (stock_id, date, open, high, low, close, volume, pe_ratio)
// - metadata (key, value) // For system state tracking
```

#### 3. Data Collector (`data_collector.rs`)
```rust
struct DataCollector {
    schwab_client: SchwabClient,
    database: DatabaseManager,
    concurrency_limit: usize,
}

impl DataCollector {
    async fn sync_sp500_list() -> Result<()>
    async fn fetch_incremental_data() -> Result<()>
    async fn fetch_historical_data(from_date: Date) -> Result<()>
}
```

#### 4. Market Calendar (`utils.rs`) ✅ **NEW**
```rust
struct MarketCalendar {
    client: SchwabClient,
}

impl MarketCalendar {
    async fn is_trading_day(date: NaiveDate) -> Result<bool>
    async fn get_last_trading_day(date: NaiveDate) -> Result<NaiveDate>
    async fn get_next_trading_day(date: NaiveDate) -> Result<NaiveDate>
    async fn adjust_date_range(start: NaiveDate, end: NaiveDate) -> Result<(NaiveDate, NaiveDate)>
}
```

#### 5. Analysis Engine (`analysis.rs`)
```rust
struct AnalysisEngine {
    database: DatabaseManager,
}

impl AnalysisEngine {
    async fn get_top_pe_decliners(limit: usize, offset: usize) -> Result<Vec<StockAnalysis>>
    async fn search_stocks(query: String) -> Result<Vec<Stock>>
    async fn get_stock_details(symbol: String) -> Result<StockDetail>
}
```

#### 6. Ratatui UI (`ui/mod.rs`)
```rust
struct StockApp {
    analysis_engine: AnalysisEngine,
    current_view: AppView,
    selected_stocks: Vec<StockAnalysis>,
    list_state: ListState,
}

enum AppView {
    Dashboard,
    StockDetail(String),
    Search,
    Settings,
}
```

### Database Schema

```sql
-- Core stock information
CREATE TABLE stocks (
    id INTEGER PRIMARY KEY,
    symbol TEXT UNIQUE NOT NULL,
    company_name TEXT NOT NULL,
    sector TEXT,
    industry TEXT,
    market_cap REAL,
    status TEXT DEFAULT 'active', -- 'active', 'delisted'
    first_trading_date DATE,
    last_updated DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Daily price and fundamental data
CREATE TABLE daily_prices (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER NOT NULL,
    date DATE NOT NULL,
    open_price REAL NOT NULL,
    high_price REAL NOT NULL, 
    low_price REAL NOT NULL,
    close_price REAL NOT NULL,
    volume INTEGER,
    pe_ratio REAL,
    market_cap REAL,
    dividend_yield REAL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, date)
);

-- System metadata and state tracking
CREATE TABLE metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for performance
CREATE INDEX idx_daily_prices_stock_date ON daily_prices(stock_id, date);
CREATE INDEX idx_daily_prices_date ON daily_prices(date);
CREATE INDEX idx_stocks_symbol ON stocks(symbol);
CREATE INDEX idx_stocks_company_name ON stocks(company_name);
```

### Key Implementation Details

#### Concurrency Strategy
- **Tokio async runtime** for concurrent API calls
- **Semaphore-based rate limiting** (120 requests/minute per Schwab limits)
- **Batch processing** - group stocks into batches of 50 for bulk quotes
- **Error isolation** - failed stocks don't block others

#### Incremental Update Algorithm
```rust
async fn fetch_incremental_data() -> Result<()> {
    let last_update = get_last_update_date().await?;
    let today = chrono::Utc::now().date_naive();
    
    if last_update >= today {
        return Ok(()); // Already up to date
    }
    
    let symbols = get_active_symbols().await?;
    let date_range = generate_trading_days(last_update + 1, today);
    
    for date in date_range {
        fetch_data_for_date(symbols.clone(), date).await?;
        update_last_sync_date(date).await?;
    }
}
```

#### P/E Analysis Algorithm
```rust
async fn calculate_pe_decline(stock_id: i64) -> Result<f64> {
    let one_year_ago = chrono::Utc::now().date_naive() - chrono::Duration::days(365);
    let latest = get_latest_price(stock_id).await?;
    let year_ago = get_price_on_date(stock_id, one_year_ago).await?;
    
    if year_ago.pe_ratio > 0.0 && latest.pe_ratio > 0.0 {
        Ok((year_ago.pe_ratio - latest.pe_ratio) / year_ago.pe_ratio * 100.0)
    } else {
        Ok(0.0) // Handle invalid P/E ratios
    }
}
```

### Professional CLI Implementation

#### Command Line Interface Features ✅ **COMPLETED**

The system now includes a professional CLI with comprehensive validation:

```bash
# Professional named arguments
cargo run --bin collect_with_detailed_logs -- --start-date 20230101 --end-date 20231231

# Short form arguments  
cargo run --bin collect_with_detailed_logs -- -s 20240101 -e 20241231

# Configurable batch processing
cargo run --bin collect_with_detailed_logs -- -s 20230101 -b 10 --batch-delay 5

# Built-in help system
cargo run --bin collect_with_detailed_logs -- --help
```

#### Smart Market Calendar ✅ **COMPLETED**

The system includes intelligent weekend and holiday handling:

```bash
# Smart collection with automatic weekend handling
cargo run --bin smart_collect -- 20250810  # Saturday → Returns Friday 2025-08-08 data

# Date ranges with automatic trading day adjustment
cargo run --bin smart_collect -- 20240101 20240131  # Adjusts to trading days only
```

**Market Calendar Features:**
- 🗓️ **Schwab API Integration**: Uses official market hours endpoint for accurate trading day detection
- 📅 **Weekend/Holiday Handling**: Saturday/Sunday requests automatically return Friday data  
- 🔄 **Automatic Date Adjustment**: Shows original vs adjusted date ranges for transparency
- ⚡ **7-Day Look-ahead**: Real-time trading day validation for recent dates
- 🛡️ **Fallback Logic**: Weekend detection for historical dates beyond API limit

#### CLI Argument Validation
- ✅ **Date Format Validation**: Strict YYYYMMDD format with digit-only validation  
- ✅ **Date Range Validation**: Start < End, End ≤ Today, reasonable bounds (1970-2050)
- ✅ **Business Logic Validation**: Prevents future dates, warns on large ranges (>10 years)
- ✅ **Parameter Validation**: Batch size (1-50), batch delay (1-60 seconds)
- ✅ **Professional Help**: Comprehensive usage examples and argument descriptions

#### Progress Tracking Architecture
```
📦 BATCH 1/101 - Processing 5 stocks:
🔄 [1/503] Starting AAPL: Apple Inc.  
✅ [1/503] AAPL completed: 417 records in 2.3s
📊 BATCH SUMMARY: ✅ 5/5 successful, 📈 2,085 records, ⏱️ 12.1s
📊 OVERALL PROGRESS: 5/503 stocks, 2,085 total records
```

## Dependencies & Technology Stack

### Core Dependencies
```toml
[dependencies]
tokio = { version = "1.0", features = ["full"] }
reqwest = { version = "0.11", features = ["json"] }
rusqlite = { version = "0.30", features = ["bundled", "chrono"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1.0"
thiserror = "1.0"
dotenvy = "0.15"

# UI Framework
ratatui = "0.28"
crossterm = "0.27"

# Async utilities
futures = "0.3"
tokio-stream = "0.1"

# Rate limiting
governor = "0.6"

# Fuzzy search
fuzzy-matcher = "0.3"

# Command line argument parsing
clap = { version = "4.0", features = ["derive"] }

# Market calendar utilities
serde_json = "1.0"  # For parsing API responses
```

## Implementation Phases

### Phase 1: Core Infrastructure ✅ COMPLETED
- [x] Schwab API client with authentication
- [x] SQLite database setup and migrations
- [x] Basic data models and error handling
- [x] Configuration management
- [x] Ratatui TUI framework
- [x] Analysis engine foundation

### Phase 2: Data Collection System ✅ COMPLETED
- [x] S&P 500 symbol fetching (200+ comprehensive symbols)
- [x] Historical data backfill (Jan 2020 - present)
- [x] Incremental update mechanism with state tracking
- [x] Concurrent data fetching with semaphore-based rate limiting
- [x] Batch processing and error isolation
- [x] Data validation and integrity checking
- [x] Interactive setup wizard

### Phase 3: Enhanced Analysis & UI ✅ COMPLETED
- [x] P/E ratio decline calculations
- [x] Stock ranking and pagination
- [x] Fuzzy search functionality (symbol/company name)
- [x] Professional CLI with named arguments and comprehensive validation
- [x] High-performance concurrent data collection with detailed progress tracking
- [x] Complete S&P 500 integration (all 503 companies)
- [x] Smart Market Calendar with automatic weekend/holiday handling
- [ ] ASCII price charts and trend visualizations
- [ ] Sector-based analysis and filtering
- [ ] Volatility and correlation calculations
- [ ] Enhanced stock detail views

### Phase 4: Advanced Features (Planned)
- [ ] Real-time data updates during TUI session
- [ ] Portfolio tracking and performance analysis
- [ ] Custom screening criteria
- [ ] Export functionality (CSV, JSON)
- [ ] Configuration management UI
- [ ] Help system and tutorials

### Phase 5: Polish & Production (Planned)
- [ ] Comprehensive unit and integration tests
- [ ] Performance profiling and optimization
- [ ] Error recovery and resilience testing  
- [ ] User documentation and deployment guides
- [ ] CI/CD pipeline setup

## Success Metrics
- **Data Coverage**: 100% S&P 500 stocks with complete historical data
- **Update Performance**: Daily sync completes within 5 minutes
- **UI Responsiveness**: <100ms response time for all interactions
- **Data Accuracy**: P/E calculations match reference sources within 1%
- **Reliability**: 99.9% uptime with graceful error recovery

## Development Status

### Current Implementation Status
- **Project Structure**: ✅ Complete - All modules implemented
- **PRD & Architecture**: ✅ Complete
- **Dependencies**: ✅ Complete - All Cargo.toml dependencies configured
- **Core Infrastructure**: ✅ Complete - Database, API client, and UI framework ready
- **Database Schema**: ✅ Complete - SQLite schema with migrations
- **API Client**: ✅ Complete - Schwab client with OAuth2 token management
- **Analysis Engine**: ✅ Complete - P/E ratio analysis and stock search
- **UI Framework**: ✅ Complete - Ratatui-based terminal interface
- **Build Status**: ✅ Successfully compiles with no errors

### Completed Phase 1 Components
✅ **Database Manager** (`database/mod.rs`)
- SQLite database with stocks, daily_prices, and metadata tables
- Automatic schema migrations and indexing
- CRUD operations for stocks and pricing data
- State tracking for incremental updates

✅ **Schwab API Client** (`api/schwab_client.rs`)  
- OAuth2 authentication with token refresh
- Rate limiting and error handling
- S&P 500 symbol support (hardcoded list for now)
- Price quotes and historical data endpoints
- Market hours and calendar API integration for trading day detection

✅ **Analysis Engine** (`analysis/mod.rs`)
- P/E ratio decline calculations over 1-year periods
- Stock ranking and pagination (top 10, next 10)
- Fuzzy search by symbol and company name
- Performance-optimized database queries

✅ **Terminal UI** (`ui/mod.rs`)
- Interactive dashboard with multiple views
- Stock list navigation with arrow keys
- Detailed stock information display
- Search interface and real-time status updates

### Completed Phase 2 Components
✅ **Data Collector** (`data_collector.rs`)
- Concurrent fetching system using futures and tokio semaphores
- S&P 500 symbol list management (200+ comprehensive stock symbols)
- Batched API calls with rate limiting and error isolation
- Historical data backfill from January 1, 2020 to present
- Incremental update mechanism with state persistence
- Data validation and integrity checking

✅ **Enhanced Main Application** (`main.rs`)
- Interactive setup wizard for new users
- Data collection prompts and progress reporting
- Comprehensive error handling and user guidance
- Integration between all system components

✅ **Market Calendar System** (`utils.rs`)
- Smart weekend and holiday detection using Schwab market hours API
- Automatic date range adjustment for non-trading days
- Seamless integration with data collection tools
- Fallback weekend detection for historical dates beyond API limits

✅ **Advanced Database Features**
- Clone trait implementation for shared database access
- Thread-safe concurrent access patterns
- Optimized queries for large datasets

### Key Phase 2 Achievements
🚀 **Scalable Data Pipeline**: Handles 200+ stocks concurrently with proper rate limiting
📊 **Smart Updates**: Only fetches new data since last update, preserving bandwidth
🔄 **Robust Error Recovery**: Individual stock failures don't break the entire pipeline
📈 **Historical Analysis Ready**: Complete price history from 2020 enables P/E trend analysis
⚡ **Performance Optimized**: Semaphore-based concurrency control and batched processing

## Enhanced Main Application Architecture ✅ **NEW DESIGN**

### Interactive Ratatui Application Design

The main application will be redesigned as a comprehensive interactive TUI with the following architecture:

#### Main Application Views
```
┌─────────────────────────────────────────────────────────────┐
│                   Stock Analysis System                     │
├─────────────────────────────────────────────────────────────┤
│  [Dashboard] [Data Collection] [Stock Analysis] [Settings]  │
└─────────────────────────────────────────────────────────────┘
```

#### 1. Dashboard View (`ui/dashboard.rs`)
**Purpose**: System overview and quick navigation
```rust
struct Dashboard {
    database_stats: DatabaseStats,
    collection_progress: CollectionProgress,
    recent_updates: Vec<RecentUpdate>,
    quick_actions: Vec<QuickAction>,
}

struct DatabaseStats {
    total_stocks: usize,
    total_price_records: usize,
    data_coverage_percentage: f64,
    last_update_date: Option<NaiveDate>,
    oldest_data_date: Option<NaiveDate>,
}

struct CollectionProgress {
    stocks_with_data: usize,
    stocks_missing_data: usize,
    target_start_date: NaiveDate, // Jan 1, 2020
    completion_percentage: f64,
    estimated_records_remaining: usize,
}
```

**Features**:
- 📊 Real-time database statistics
- 📈 Progress toward Jan 1, 2020 goal 
- 🎯 Data collection completion metrics
- ⚡ Quick action buttons for common tasks

#### 2. Data Collection View (`ui/data_collection.rs`)
**Purpose**: Interactive data fetching and progress monitoring

```rust
struct DataCollectionView {
    selected_action: usize,
    actions: Vec<DataCollectionAction>,
    is_executing: bool,
    current_operation: Option<ActiveOperation>,
    log_messages: Vec<LogMessage>,
    date_range: Option<DateRange>,
}

struct DataCollectionAction {
    id: String,
    title: String,
    description: String,
    action_type: ActionType,
    requires_confirmation: bool,
}

enum ActionType {
    UpdateSP500List,
    CollectHistoricalData { start_date: NaiveDate, end_date: NaiveDate },
    IncrementalUpdate,
    ValidateData,
    ViewProgress,
}

struct ActiveOperation {
    action_id: String,
    start_time: DateTime<Utc>,
    progress: f64,
    current_message: String,
    logs: Vec<String>,
}

struct LogMessage {
    timestamp: DateTime<Utc>,
    level: LogLevel,
    message: String,
}

enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
}
```

**Features**:
- 🎯 **Interactive Action Selection**: Arrow keys navigation with Enter to execute
- 🚀 **Direct Execution**: Run data collection operations from within TUI
- 📊 **Real-time Progress**: Live progress bars and status updates
- 📝 **Live Logging**: Real-time log display during operations
- 📅 **Date Range Selection**: Interactive date picker for historical data
- ⏸️ **Operation Control**: Start, pause, cancel operations
- 🔄 **Background Processing**: Non-blocking UI during long operations
- ✅ **Status Feedback**: Success/failure indicators with detailed messages

#### 3. Stock Analysis View (`ui/stock_analysis.rs`)
**Purpose**: Interactive stock search, selection, and analysis

```rust
struct StockAnalysisView {
    search_state: SearchState,
    selected_stock: Option<StockDetail>,
    analysis_panels: AnalysisPanels,
    chart_view: ChartView,
}

struct SearchState {
    query: String,
    search_results: Vec<Stock>,
    selected_index: usize,
    search_mode: SearchMode,
}

enum SearchMode {
    BySymbol,
    ByCompanyName,
    BySector,
}

struct StockDetail {
    stock: Stock,
    price_history: Vec<DailyPrice>,
    analysis_metrics: StockMetrics,
    data_coverage: DataCoverage,
}

struct StockMetrics {
    pe_ratio_trend: Option<PETrend>,
    price_performance: PricePerformance,
    volatility_metrics: VolatilityMetrics,
}

struct DataCoverage {
    earliest_date: Option<NaiveDate>,
    latest_date: Option<NaiveDate>,
    total_records: usize,
    missing_ranges: Vec<DateRange>,
    coverage_percentage: f64,
}
```

**Features**:
- 🔍 **Intelligent Search**: Fuzzy matching by symbol, company name, or sector
- 📊 **Comprehensive Analysis**: P/E trends, price performance, volatility analysis
- 📈 **ASCII Charts**: Historical price visualization in terminal
- 📅 **Data Coverage Analysis**: Visual representation of available vs missing data
- 🏆 **Top Performers**: P/E decline rankings and performance metrics

#### 4. Data Progress Analyzer (`ui/progress_analyzer.rs`) 
**Purpose**: Comprehensive progress tracking and gap analysis

```rust
struct ProgressAnalyzer {
    overall_progress: OverallProgress,
    stock_progress_list: Vec<StockProgress>,
    gap_analysis: GapAnalysis,
    recommendations: Vec<ActionRecommendation>,
}

struct OverallProgress {
    target_start_date: NaiveDate, // Jan 1, 2020
    total_target_records: usize,   // ~1.5M records
    current_records: usize,
    completion_percentage: f64,
    stocks_completed: usize,       // 100% data from Jan 1, 2020
    stocks_partial: usize,         // Some data but gaps
    stocks_missing: usize,         // No data at all
}

struct StockProgress {
    stock: Stock,
    data_range: Option<(NaiveDate, NaiveDate)>,
    record_count: usize,
    expected_records: usize,
    missing_ranges: Vec<DateRange>,
    priority_score: f64, // Higher = needs attention
}

struct GapAnalysis {
    total_missing_days: usize,
    largest_gaps: Vec<DataGap>,
    stocks_needing_attention: Vec<String>,
    estimated_collection_time: std::time::Duration,
}

struct ActionRecommendation {
    action_type: ActionType,
    priority: Priority,
    description: String,
    estimated_impact: String,
}

enum ActionType {
    CollectMissingData { symbols: Vec<String>, date_range: DateRange },
    FillDataGaps { symbol: String, gaps: Vec<DateRange> },
    ValidateDataQuality { symbols: Vec<String> },
    UpdateRecentData,
}
```

**Features**:
- 📊 **Progress Dashboard**: Visual progress toward Jan 1, 2020 goal
- 📋 **Stock Priority List**: Ranked by data completeness and importance
- 🔍 **Gap Analysis**: Detailed missing data identification
- 💡 **Smart Recommendations**: Actionable next steps for data collection
- ⏱️ **Time Estimates**: Projected completion times for remaining work

#### Navigation & User Experience

```rust
enum AppView {
    Dashboard,
    DataCollection,
    StockAnalysis,
    ProgressAnalyzer,
    Settings,
}

struct MainApp {
    current_view: AppView,
    database: DatabaseManager,
    schwab_client: SchwabClient,
    market_calendar: MarketCalendar,
    
    // View components
    dashboard: Dashboard,
    data_collection: DataCollectionView,
    stock_analysis: StockAnalysisView,
    progress_analyzer: ProgressAnalyzer,
}
```

**Navigation Features**:
- ⌨️ **Keyboard Shortcuts**: Tab (next view), Shift+Tab (prev view), Enter (select), Esc (back)
- 📱 **Context-Sensitive Help**: F1 for help in current view
- 🎨 **Visual Indicators**: Progress bars, status colors, real-time updates
- 🔄 **Background Operations**: Non-blocking data collection with progress updates

#### Integration Points

1. **Data Collection Integration**:
   - Launch `collect_with_detailed_logs` from within the TUI
   - Real-time progress updates during collection
   - Background task monitoring and control

2. **Market Calendar Integration**:
   - Smart date validation using `MarketCalendar`
   - Weekend/holiday handling for data requests
   - Trading day calculations for progress metrics

3. **Database Integration**:
   - Enhanced `DatabaseManager` with progress analysis methods
   - Real-time statistics updates
   - Transaction support for bulk operations

### Phase 3 Implementation Plan: Enhanced UI & Analysis
1. **Enhanced Terminal UI**: ✅ **COMPLETED** - Comprehensive Ratatui application with multiple views
2. **Interactive Data Collection**: 🔄 **IN PROGRESS** - Interactive action selection and execution
3. **Data Progress Tracking**: ✅ **DESIGNED** - Complete progress analysis and gap detection
4. **Advanced Stock Analysis**: ✅ **DESIGNED** - Search, charts, and metrics analysis
5. **Real-time Monitoring**: ✅ **DESIGNED** - Background operations with live updates

### Interactive Data Collection Implementation

#### Action Definitions
```rust
let actions = vec![
    DataCollectionAction {
        id: "update_sp500".to_string(),
        title: "📋 Update S&P 500 company list".to_string(),
        description: "Fetch latest S&P 500 constituents from official sources".to_string(),
        action_type: ActionType::UpdateSP500List,
        requires_confirmation: false,
    },
    DataCollectionAction {
        id: "collect_historical".to_string(),
        title: "📈 Collect historical data (2020-2025)".to_string(),
        description: "Fetch complete historical OHLC data for all stocks".to_string(),
        action_type: ActionType::CollectHistoricalData { 
            start_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            end_date: chrono::Utc::now().date_naive(),
        },
        requires_confirmation: true,
    },
    DataCollectionAction {
        id: "incremental_update".to_string(),
        title: "🔄 Incremental daily updates".to_string(),
        description: "Fetch latest data since last update".to_string(),
        action_type: ActionType::IncrementalUpdate,
        requires_confirmation: false,
    },
    DataCollectionAction {
        id: "validate_data".to_string(),
        title: "📊 Validate data integrity".to_string(),
        description: "Check data completeness and identify gaps".to_string(),
        action_type: ActionType::ValidateData,
        requires_confirmation: false,
    },
    DataCollectionAction {
        id: "view_progress".to_string(),
        title: "📊 View collection progress".to_string(),
        description: "Show current data collection status".to_string(),
        action_type: ActionType::ViewProgress,
        requires_confirmation: false,
    },
];
```

#### User Interaction Flow
1. **Navigation**: Arrow keys (↑/↓) to select action
2. **Selection**: Enter to execute selected action
3. **Confirmation**: For destructive operations, show confirmation dialog
4. **Execution**: Run operation in background with progress updates
5. **Feedback**: Display real-time logs and status messages
6. **Completion**: Show success/failure summary

#### Operation Execution
```rust
async fn execute_action(&mut self, action: &DataCollectionAction) -> Result<()> {
    match &action.action_type {
        ActionType::UpdateSP500List => {
            self.log_info("Starting S&P 500 list update...");
            // Execute: cargo run --bin update_sp500
            self.run_update_sp500().await?;
        }
        ActionType::CollectHistoricalData { start_date, end_date } => {
            self.log_info(&format!("Starting historical data collection from {} to {}", start_date, end_date));
            // Execute: cargo run --bin collect_with_detailed_logs -- -s {start_date} -e {end_date}
            self.run_historical_collection(*start_date, *end_date).await?;
        }
        ActionType::IncrementalUpdate => {
            self.log_info("Starting incremental update...");
            // Execute: cargo run --bin smart_collect
            self.run_incremental_update().await?;
        }
        ActionType::ValidateData => {
            self.log_info("Validating data integrity...");
            self.validate_data_integrity().await?;
        }
        ActionType::ViewProgress => {
            self.show_collection_progress();
        }
    }
    Ok(())
}
```

### Known Challenges & Solutions
- **Rate Limiting**: Use governor crate for request throttling
- **Large Dataset**: Implement streaming and batched processing
- **API Authentication**: Leverage existing Python token management
- **Error Recovery**: Implement robust retry mechanisms with exponential backoff

## Data Collection Architecture Consolidation (Phase 4A)

### Problem Statement

The current data collection system has architectural redundancy between single stock and concurrent stock fetching:

**Current Issues:**
- **Duplicate Batching Logic**: Both single and concurrent flows implement trading week batching
- **Multiple Similar Functions**: Three functions in DataCollector doing similar work with different interfaces
- **Inconsistent UI Logging**: Manual batch processing in UI layer duplicates existing batching logic
- **Code Maintenance Burden**: Changes must be made in multiple places

**Current Architecture:**
```
Single Stock Flow:
UI (run_single_stock_collection) 
  → Manual batching with TradingWeekBatchCalculator
  → DataCollector::fetch_stock_history() (no batching)
  → Custom TUI logging per batch

Concurrent Stock Flow:  
UI (start_concurrent_fetching)
  → ConcurrentFetcher
  → DataCollector::fetch_stock_history_with_batching_ref() (with batching)
  → Thread-based TUI logging
```

### Consolidation Plan

**Philosophical Truth**: Single stock fetching is just concurrent fetching with `threads=1` and `stocks=[selected_stock]`

**Target Architecture:**
```
Unified Flow:
UI → Unified Stock Fetcher → Consistent TUI Logging

Single Stock: UnifiedFetcher(threads=1, stocks=[selected])
All Stocks:   UnifiedFetcher(threads=5-10, stocks=get_active_stocks())
```

### Implementation Plan

#### Step 1: Create Unified Configuration
```rust
#[derive(Debug, Clone)]
pub struct UnifiedFetchConfig {
    pub stocks: Vec<Stock>,           // Single=[selected], All=get_active_stocks()  
    pub date_range: DateRange,
    pub num_threads: usize,           // Single=1, Concurrent=5-10
    pub retry_attempts: u32,
    pub rate_limit_ms: u64,
    pub max_stocks: Option<usize>,    // For testing limits
}
```

#### Step 2: Consolidate DataCollector Functions
**Remove:**
- `fetch_stock_history()` (simple, no batching)
- Manual batching logic in `run_single_stock_collection`

**Keep & Enhance:**
- `fetch_stock_history_with_batching_ref()` → rename to `fetch_single_stock_with_batching()`
- Make it the single source of truth for all stock data fetching

#### Step 3: Update UI Layer
**Single Stock Collection:**
```rust
async fn run_single_stock_collection(&mut self, stock: Stock, date_range: DateRange) {
    let config = UnifiedFetchConfig {
        stocks: vec![stock],
        date_range,
        num_threads: 1,
        retry_attempts: 3,
        rate_limit_ms: 500,
        max_stocks: None,
    };
    
    fetch_stocks_unified_with_logging(database, config, global_broadcast_sender).await
}
```

**Concurrent Collection:**
```rust
async fn start_concurrent_fetching(&mut self, date_range: DateRange) {
    let all_stocks = database.get_active_stocks().await?;
    let config = UnifiedFetchConfig {
        stocks: all_stocks,
        date_range,
        num_threads: 5,
        retry_attempts: 3,
        rate_limit_ms: 500,
        max_stocks: None,
    };
    
    fetch_stocks_unified_with_logging(database, config, global_broadcast_sender).await
}
```

#### Step 4: Unified Function Signature
```rust
pub async fn fetch_stocks_unified_with_logging(
    database: Arc<DatabaseManagerSqlx>,
    config: UnifiedFetchConfig,
    global_broadcast_sender: Option<Arc<broadcast::Sender<StateUpdate>>>,
) -> Result<FetchResult>
```

### Expected Benefits

1. **Code Elimination:**
   - Remove ~200 lines of duplicate batching logic from UI
   - Eliminate `fetch_stock_history()` function
   - Single function handles all data fetching scenarios

2. **Consistent Behavior:**
   - Same error handling and retry logic for both flows
   - Unified logging format and timing
   - Same rate limiting strategy

3. **Easier Testing:**
   - Single code path to test for all scenarios
   - Test binaries automatically get improvements
   - Consistent behavior across UI and CLI

4. **Maintainability:**
   - Changes made in one place affect all flows
   - Clear separation of concerns: UI handles UX, DataCollector handles fetching
   - Simpler debugging and profiling

### Migration Checklist

- [ ] Create `UnifiedFetchConfig` struct
- [ ] Rename `fetch_stocks_concurrently_with_logging` → `fetch_stocks_unified_with_logging`
- [ ] Update function to accept `UnifiedFetchConfig` instead of `ConcurrentFetchConfig`
- [ ] Remove manual batching from `run_single_stock_collection`
- [ ] Update single stock UI to call unified fetcher with threads=1
- [ ] Remove `fetch_stock_history()` function from DataCollector
- [ ] Update test binaries to use unified interface
- [ ] Verify UI functionality: both "single stock" and "all stocks" work identically
- [ ] Test with different thread counts (1, 5, 10)

### Test Compatibility

**Test Binaries Impact:**
- `data_collection_test.rs`: Currently calls `DataCollector::fetch_stock_history()` - needs update
- `test_concurrent_fetcher_sqlx.rs`: Uses concurrent fetcher - minimal changes needed
- Other test binaries: Use tracing directly, unaffected

**UI Functionality Preserved:**
- ✅ User can still select single stock + date range
- ✅ User can still select "all stocks" + date range  
- ✅ Same TUI experience, cleaner implementation
- ✅ Same performance characteristics
- ✅ Same error handling and recovery

### Rollback Plan

If issues arise during consolidation:
1. Revert to git commit before consolidation starts
2. Each step is incremental and can be backed out individually
3. Tests will catch any functional regressions immediately
4. UI behavior should be identical - any differences indicate bugs

---
*Last Updated: 2025-01-04*
*Version: 1.1 - Added Data Collection Consolidation Plan*