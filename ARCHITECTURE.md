# Stock Analysis System - Architecture Document

## Executive Summary
A high-performance desktop application for stock analysis using Tauri (Rust backend + React frontend) that fetches, stores, and analyzes S&P 500 stock data using the Charles Schwab API. Features comprehensive fundamental data collection, real-time market data, and interactive web-based UI.

## Current System Architecture

### Technology Stack
- **Frontend**: React with JSX, modern JavaScript ES6+
- **Backend**: Rust with Tauri framework 
- **Database**: SQLite for local persistence
- **API Integration**: Charles Schwab Market Data Production API
- **Desktop Framework**: Tauri for cross-platform desktop application
- **UI Framework**: Web-based interface rendered in Tauri webview

### System Components

```
┌──────────────────────────────────────────────────────────────┐
│                 Stock Analysis Desktop App                   │
├──────────────────────────────────────────────────────────────┤
│  React Frontend (JSX) ←→ Tauri IPC ←→ Rust Backend          │
│         ↓                              ↓                     │
│  [UI Components]              [Tauri Commands]               │
│  [State Management]           [Database Manager]             │
│  [Data Visualization]         [Schwab API Client]            │
│                              [Analysis Engine]               │
└──────────────────────────────────────────────────────────────┘
```

## Enhanced Schwab API Integration Plan

Based on official Schwab Market Data Production API capabilities:

### Available Data Endpoints

#### 1. Market Data Production API
- **Quotes**: Real-time stock quotes with bid/ask spreads
- **Price History**: Historical OHLCV data with multiple timeframes
- **Option Chains**: Options data with Greeks calculations
- **Market Movers**: Top gainers/losers by index
- **Market Hours**: Trading calendar and market status
- **Instruments**: Symbol search and company fundamentals

#### 2. Data Fields Available
**Price Data:**
- Open, High, Low, Close prices
- Volume and average volume
- Adjusted close prices
- Extended hours trading data

**Fundamental Data:**
- P/E ratios (trailing and forward)
- Market capitalization
- Dividend yield and dividend data
- EPS (earnings per share)
- Beta values
- 52-week high/low ranges
- Price-to-book ratios
- Sector and industry classification

**Real-time Quote Data:**
- Bid/Ask prices and sizes
- Last trade price and volume
- Market status indicators
- Real-time changes and percentages

### Enhanced Database Schema

```sql
-- Enhanced stocks table with comprehensive company data
CREATE TABLE stocks_enhanced (
    id INTEGER PRIMARY KEY,
    symbol TEXT UNIQUE NOT NULL,
    company_name TEXT,
    exchange TEXT,
    sector TEXT,
    industry TEXT,
    market_cap REAL,
    description TEXT,
    employees INTEGER,
    founded_year INTEGER,
    headquarters TEXT,
    website TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Enhanced daily prices with comprehensive fundamental metrics
CREATE TABLE daily_prices_enhanced (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER,
    date TEXT,
    open_price REAL,
    high_price REAL,
    low_price REAL,
    close_price REAL,
    adjusted_close REAL,
    volume INTEGER,
    average_volume INTEGER,
    
    -- Fundamental ratios
    pe_ratio REAL,
    pe_ratio_forward REAL,
    pb_ratio REAL,
    ps_ratio REAL,
    dividend_yield REAL,
    dividend_per_share REAL,
    eps REAL,
    eps_forward REAL,
    beta REAL,
    
    -- 52-week data
    week_52_high REAL,
    week_52_low REAL,
    week_52_change_percent REAL,
    
    -- Market metrics
    shares_outstanding REAL,
    float_shares REAL,
    revenue_ttm REAL,
    profit_margin REAL,
    operating_margin REAL,
    return_on_equity REAL,
    return_on_assets REAL,
    debt_to_equity REAL,
    
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (stock_id) REFERENCES stocks_enhanced (id)
);

-- Separate real-time quotes table for live data
CREATE TABLE real_time_quotes (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER,
    timestamp TIMESTAMP,
    bid_price REAL,
    bid_size INTEGER,
    ask_price REAL,
    ask_size INTEGER,
    last_price REAL,
    last_size INTEGER,
    volume INTEGER,
    change_amount REAL,
    change_percent REAL,
    day_high REAL,
    day_low REAL,
    FOREIGN KEY (stock_id) REFERENCES stocks_enhanced (id)
);

-- Intraday price data for detailed analysis
CREATE TABLE intraday_prices (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER,
    datetime TIMESTAMP,
    interval_type TEXT, -- '1min', '5min', '15min', '30min', '1hour'
    open_price REAL,
    high_price REAL,
    low_price REAL,
    close_price REAL,
    volume INTEGER,
    FOREIGN KEY (stock_id) REFERENCES stocks_enhanced (id)
);

-- Option chains data
CREATE TABLE option_chains (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER,
    expiration_date DATE,
    strike_price REAL,
    option_type TEXT, -- 'CALL' or 'PUT'
    bid REAL,
    ask REAL,
    last_price REAL,
    volume INTEGER,
    open_interest INTEGER,
    implied_volatility REAL,
    delta REAL,
    gamma REAL,
    theta REAL,
    vega REAL,
    rho REAL,
    FOREIGN KEY (stock_id) REFERENCES stocks_enhanced (id)
);

-- Performance indexes for fast queries
CREATE INDEX idx_daily_prices_enhanced_stock_date ON daily_prices_enhanced(stock_id, date);
CREATE INDEX idx_daily_prices_enhanced_date ON daily_prices_enhanced(date);
CREATE INDEX idx_real_time_quotes_stock_timestamp ON real_time_quotes(stock_id, timestamp);
CREATE INDEX idx_intraday_prices_stock_datetime ON intraday_prices(stock_id, datetime);
CREATE INDEX idx_option_chains_stock_expiration ON option_chains(stock_id, expiration_date);
```

## Current Frontend Architecture

### React Component Structure

```
frontend/src/
├── App.jsx                    # Main application component
├── components/
│   ├── Dashboard.jsx         # Overview dashboard
│   ├── StockList.jsx        # Stock selection and display
│   ├── Analysis.jsx         # Price history and charts
│   ├── DataFetching.jsx     # Data collection interface
│   └── Settings.jsx         # Application settings
├── hooks/
│   ├── useStocks.js         # Stock data management
│   ├── useAnalysis.js       # Analysis calculations
│   └── useTauri.js          # Tauri API integration
└── utils/
    ├── formatters.js        # Data formatting utilities
    ├── calculations.js      # Financial calculations
    └── api.js              # API helper functions
```

### Current Features
1. **Stock Selection**: Dropdown with visual indicators (📊 for data available, 📋 for no data)
2. **S&P 500 Initialization**: Fetch and populate S&P 500 company list
3. **Data Collection**: Single stock and bulk data fetching
4. **Price History Analysis**: Historical price data visualization
5. **Data Export**: Export functionality for analysis

## Current Backend Architecture (Tauri Commands)

### Tauri Command Structure

```rust
src-tauri/src/
├── main.rs                   # Tauri application entry point
├── commands/
│   ├── mod.rs               # Commands module exports
│   ├── stocks.rs            # Stock information commands
│   ├── analysis.rs          # Data analysis commands
│   ├── fetching.rs          # Data fetching commands
│   └── initialization.rs    # S&P 500 initialization
├── database/
│   ├── mod.rs              # Database management
│   └── migrations.rs       # Schema migrations
├── schwab/
│   ├── mod.rs              # Schwab API client
│   ├── auth.rs             # OAuth authentication
│   ├── market_data.rs      # Market data endpoints
│   └── rate_limiter.rs     # API rate limiting
└── utils/
    ├── mod.rs              # Utility functions
    └── market_calendar.rs  # Trading day calculations
```

### Current Tauri Commands

```rust
// Stock information commands
#[tauri::command]
async fn get_all_stocks() -> Result<Vec<StockInfo>, String>

#[tauri::command]
async fn search_stocks(query: String) -> Result<Vec<StockInfo>, String>

#[tauri::command]
async fn get_stocks_with_data_status() -> Result<Vec<StockWithData>, String>

// Analysis commands
#[tauri::command]
async fn get_price_history(symbol: String, start_date: String, end_date: String) -> Result<Vec<PriceData>, String>

#[tauri::command]
async fn export_data(symbol: String, format: String) -> Result<String, String>

// Data fetching commands
#[tauri::command]
async fn fetch_single_stock_data(symbol: String, start_date: String, end_date: String) -> Result<String, String>

#[tauri::command]
async fn fetch_all_stocks_concurrent(start_date: String, end_date: String) -> Result<String, String>

// Initialization commands
#[tauri::command]
async fn initialize_sp500_stocks() -> Result<String, String>

#[tauri::command]
async fn get_initialization_status() -> Result<InitProgress, String>
```

## Enhanced Implementation Plan

### Phase 1: Database Migration
1. **Backup Current Data**: Export existing price data
2. **Create Enhanced Schema**: Implement new table structure
3. **Data Migration Scripts**: Transfer existing data to new format
4. **Verification**: Ensure data integrity after migration

### Phase 2: Enhanced Schwab API Integration
1. **API Client Enhancement**: 
   - Add fundamentals data endpoints
   - Implement real-time quotes
   - Add intraday data support
   - Enhance error handling and retry logic

2. **New Tauri Commands**:
   ```rust
   #[tauri::command]
   async fn fetch_comprehensive_data(symbol: String, start_date: String, end_date: String) -> Result<ComprehensiveStockData, String>

   #[tauri::command]
   async fn get_real_time_quote(symbol: String) -> Result<RealTimeQuote, String>

   #[tauri::command]
   async fn fetch_fundamentals(symbol: String) -> Result<FundamentalData, String>

   #[tauri::command]
   async fn get_intraday_data(symbol: String, interval: String) -> Result<Vec<IntradayPrice>, String>

   #[tauri::command]
   async fn get_option_chain(symbol: String) -> Result<OptionChain, String>
   ```

3. **Enhanced Data Models**:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ComprehensiveStockData {
       pub price_data: Vec<EnhancedPriceData>,
       pub fundamentals: FundamentalData,
       pub real_time_quote: Option<RealTimeQuote>,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct FundamentalData {
       pub pe_ratio: Option<f64>,
       pub pe_ratio_forward: Option<f64>,
       pub market_cap: Option<f64>,
       pub dividend_yield: Option<f64>,
       pub eps: Option<f64>,
       pub beta: Option<f64>,
       pub week_52_high: Option<f64>,
       pub week_52_low: Option<f64>,
       pub debt_to_equity: Option<f64>,
       pub return_on_equity: Option<f64>,
   }
   ```

### Phase 3: Frontend Enhancements
1. **Enhanced UI Components**:
   - Comprehensive dashboard with fundamental metrics
   - Real-time quote display with live updates
   - Advanced charting with technical indicators
   - Fundamental analysis dashboard
   - Options data visualization

2. **New React Components**:
   ```jsx
   // Real-time quote display
   const RealTimeQuote = ({ symbol }) => { ... }

   // Fundamental metrics dashboard
   const FundamentalsDashboard = ({ symbol }) => { ... }

   // Advanced price charts with indicators
   const AdvancedChart = ({ priceData, indicators }) => { ... }

   // Options chain display
   const OptionChain = ({ symbol, expiration }) => { ... }
   ```

### Phase 4: Advanced Analytics
1. **Technical Analysis**: Moving averages, RSI, MACD, Bollinger Bands
2. **Fundamental Analysis**: P/E trend analysis, dividend analysis, growth metrics
3. **Comparative Analysis**: Stock comparison and sector analysis
4. **Portfolio Tracking**: Track and analyze stock portfolios
5. **Screening Tools**: Custom stock screening criteria

### Phase 5: Performance & Production
1. **Caching Strategy**: Implement intelligent data caching
2. **Background Updates**: Automatic data refresh during market hours
3. **Export Enhancements**: Advanced export formats (PDF reports, Excel)
4. **User Preferences**: Customizable dashboards and settings
5. **Performance Optimization**: Database query optimization and UI performance

## Implementation Timeline

| Phase | Duration | Key Deliverables |
|-------|----------|------------------|
| Phase 1 | 1 week | Enhanced database schema, data migration |
| Phase 2 | 2 weeks | Enhanced Schwab API integration, comprehensive data fetching |
| Phase 3 | 2 weeks | Enhanced React UI, real-time features |
| Phase 4 | 3 weeks | Advanced analytics and screening tools |
| Phase 5 | 1 week | Performance optimization, production readiness |

## Database Migration Strategy

### Migration Steps
1. **Create Migration Scripts**: SQL scripts for schema changes
2. **Data Backup**: Export current data to CSV/JSON
3. **Schema Update**: Apply new table structure
4. **Data Import**: Migrate existing data to new format
5. **Index Creation**: Add performance indexes
6. **Verification**: Validate data integrity and completeness

### Migration Script Example
```rust
async fn migrate_to_enhanced_schema(db: &DatabaseManager) -> Result<(), String> {
    // Step 1: Create backup tables
    db.execute("CREATE TABLE stocks_backup AS SELECT * FROM stocks").await?;
    db.execute("CREATE TABLE daily_prices_backup AS SELECT * FROM daily_prices").await?;

    // Step 2: Create new enhanced tables
    db.execute(CREATE_STOCKS_ENHANCED_SQL).await?;
    db.execute(CREATE_DAILY_PRICES_ENHANCED_SQL).await?;

    // Step 3: Migrate existing data
    migrate_stocks_data(db).await?;
    migrate_price_data(db).await?;

    // Step 4: Verify data integrity
    verify_migration(db).await?;

    // Step 5: Drop backup tables (optional)
    // db.execute("DROP TABLE stocks_backup").await?;
    
    Ok(())
}
```

## Success Metrics
- **Data Coverage**: 100% S&P 500 stocks with comprehensive fundamental data
- **Real-time Performance**: <500ms response time for real-time quotes
- **UI Responsiveness**: <100ms response time for all UI interactions
- **Data Accuracy**: Fundamental ratios match reference sources within 1%
- **Application Performance**: Smooth desktop application experience

## Risk Mitigation
- **API Rate Limits**: Implement intelligent rate limiting and request queuing
- **Data Validation**: Comprehensive validation of all market data
- **Error Recovery**: Robust error handling with automatic retries
- **Database Performance**: Proper indexing and query optimization
- **UI Performance**: Efficient state management and component optimization

---
*Last Updated: 2025-01-07*
*Version: 2.0 - Updated for Tauri + React Architecture with Enhanced Schwab API Integration*