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
┌─────────────────────────────────────────────────────────────┐
│                    Stock Analysis System                    │
├─────────────────────────────────────────────────────────────┤
│  [Ratatui UI] ←→ [Analysis Engine] ←→ [Database Manager]   │
│                        ↓                     ↓              │
│                [Data Collector] ←→ [Schwab API Client]     │
└─────────────────────────────────────────────────────────────┘
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

#### 4. Analysis Engine (`analysis.rs`)
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

#### 5. Ratatui UI (`ui/mod.rs`)
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

### Phase 3: Enhanced Analysis & UI (In Progress)
- [x] P/E ratio decline calculations
- [x] Stock ranking and pagination
- [x] Fuzzy search functionality (symbol/company name)
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

### Phase 3 Ready: Enhanced UI & Analysis
1. **Enhanced Terminal UI**: ASCII charts, better stock detail views, real-time updates
2. **Advanced Analysis**: Sector analysis, volatility calculations, correlation matrices  
3. **Performance Monitoring**: Database query optimization, memory usage tracking
4. **User Experience**: Progress bars, better error messages, help system
5. **Testing & Documentation**: Comprehensive test suite and user documentation

### Known Challenges & Solutions
- **Rate Limiting**: Use governor crate for request throttling
- **Large Dataset**: Implement streaming and batched processing
- **API Authentication**: Leverage existing Python token management
- **Error Recovery**: Implement robust retry mechanisms with exponential backoff

---
*Last Updated: 2025-08-29*
*Version: 1.0*