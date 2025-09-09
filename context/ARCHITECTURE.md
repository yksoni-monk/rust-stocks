# Stock Analysis System - Architecture Document

## Executive Summary
A high-performance desktop application for stock analysis using Tauri (Rust backend + React frontend) that imports and analyzes comprehensive stock data from SimFin CSV files. Features offline-first architecture with 5,000+ stocks, comprehensive fundamental data, daily price history, and expandable panels UI for efficient analysis.

## Current System Architecture

### Technology Stack
- **Frontend**: React with JSX, modern JavaScript ES6+, expandable panels UI
- **Backend**: Rust with Tauri framework 
- **Database**: SQLite for local persistence
- **Data Source**: SimFin CSV import system (offline-first)
- **Future API Integration**: Charles Schwab API (for real-time quotes and options)
- **Desktop Framework**: Tauri for cross-platform desktop application
- **UI Framework**: Web-based interface rendered in Tauri webview

### System Components

```
┌──────────────────────────────────────────────────────────────┐
│                 Stock Analysis Desktop App                   │
├──────────────────────────────────────────────────────────────┤
│  React Frontend (JSX) ←→ Tauri IPC ←→ Rust Backend          │
│         ↓                              ↓                     │
│  [Expandable Panels UI]       [Tauri Commands]               │
│  [Stock Row Management]       [Database Manager]             │
│  [Data Visualization]         [SimFin Importer]              │
│  [User-Driven Analysis]       [Analysis Engine]              │
│                              [Future: Schwab API]            │
└──────────────────────────────────────────────────────────────┘
```

## Current Data Architecture - SimFin Offline System

The system uses SimFin CSV data for comprehensive historical stock analysis:

### SimFin Data Import System

#### 1. Data Sources
- **Daily Prices CSV**: `us-shareprices-daily.csv` (~6.2M records, 5,876+ stocks, 2019-2024)
- **Quarterly Financials CSV**: `us-income-quarterly.csv` (~52k financial records)
- **Coverage**: Professional-grade financial data for 5,000+ US companies
- **Update Frequency**: Manual import of fresh CSV data

#### 2. Import Process
**Phase 1**: Stock Discovery
- Extract unique stocks from daily prices CSV
- Create stock records with SimFin IDs and symbols

**Phase 2**: Daily Price Import
- Batch import of OHLCV data with shares outstanding
- 10,000 record batches for performance
- Progress tracking with real-time feedback

**Phase 3**: Quarterly Financials Import
- Comprehensive income statement data
- Revenue, expenses, net income, shares outstanding
- Fiscal year and period tracking

**Phase 4**: EPS Calculation
- Automated EPS calculation: Net Income ÷ Diluted Shares Outstanding
- Stored in quarterly_financials table

**Phase 5**: P/E Ratio Calculation
- Automated P/E calculation: Close Price ÷ Latest Available EPS
- Applied to all historical daily prices

**Phase 6**: Performance Indexing
- Create database indexes for fast queries
- Optimize for analysis and visualization

#### 3. Current Data Coverage
**Price Data:**
- Open, High, Low, Close prices (daily)
- Volume and shares outstanding
- Complete historical coverage 2019-2024
- ~6.2M price records across 5,876+ stocks

**Fundamental Data:**
- Revenue and cost metrics
- Operating and net income
- Shares basic and diluted
- Calculated EPS values
- Calculated P/E ratios
- Comprehensive quarterly coverage

### Current Database Schema (SimFin-Based)

```sql
-- Stocks table with SimFin integration
CREATE TABLE stocks (
    id INTEGER PRIMARY KEY,
    symbol TEXT UNIQUE NOT NULL,
    company_name TEXT NOT NULL,
    sector TEXT,
    industry TEXT,
    market_cap REAL,
    status TEXT DEFAULT 'active',
    first_trading_date DATE,
    last_updated DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    currency TEXT DEFAULT 'USD',
    shares_outstanding INTEGER,
    simfin_id INTEGER  -- SimFin unique identifier
);

-- Daily prices with calculated fundamentals
CREATE TABLE daily_prices (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER NOT NULL,
    date DATE NOT NULL,
    open_price REAL NOT NULL,
    high_price REAL NOT NULL,
    low_price REAL NOT NULL,
    close_price REAL NOT NULL,
    volume INTEGER,
    
    -- Calculated fundamental ratios
    pe_ratio REAL,           -- Calculated: Close Price ÷ Latest EPS
    market_cap REAL,
    dividend_yield REAL,
    eps REAL,
    beta REAL,
    week_52_high REAL,
    week_52_low REAL,
    pb_ratio REAL,
    ps_ratio REAL,
    shares_outstanding REAL,
    profit_margin REAL,
    operating_margin REAL,
    return_on_equity REAL,
    return_on_assets REAL,
    debt_to_equity REAL,
    dividend_per_share REAL,
    
    data_source TEXT DEFAULT 'simfin',  -- Track data source
    last_updated DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, date)
);

-- Quarterly financials from SimFin
CREATE TABLE quarterly_financials (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER NOT NULL,
    simfin_id INTEGER NOT NULL,
    currency TEXT NOT NULL DEFAULT 'USD',
    fiscal_year INTEGER NOT NULL,
    fiscal_period TEXT NOT NULL, -- Q1, Q2, Q3, Q4
    report_date DATE NOT NULL,
    publish_date DATE,
    restated_date DATE,
    
    -- Share Information
    shares_basic INTEGER,
    shares_diluted INTEGER,
    
    -- Income Statement Metrics
    revenue REAL,
    cost_of_revenue REAL,
    gross_profit REAL,
    operating_expenses REAL,
    selling_general_admin REAL,
    research_development REAL,
    depreciation_amortization REAL,
    operating_income REAL,
    non_operating_income REAL,
    interest_expense_net REAL,
    pretax_income_adj REAL,
    pretax_income REAL,
    income_tax_expense REAL,
    income_continuing_ops REAL,
    net_extraordinary_gains REAL,
    net_income REAL,
    net_income_common REAL,
    
    -- Calculated EPS
    eps_calculated REAL, -- Net Income ÷ Diluted Shares Outstanding
    eps_calculation_date DATETIME,
    
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, fiscal_year, fiscal_period)
);

-- Earnings data and processing status tables
CREATE TABLE earnings_data (...);
CREATE TABLE processing_status (...);

-- Performance indexes for fast analysis
CREATE INDEX idx_stocks_simfin_id ON stocks(simfin_id);
CREATE INDEX idx_daily_prices_stock_date ON daily_prices(stock_id, date);
CREATE INDEX idx_daily_prices_pe_ratio ON daily_prices(pe_ratio);
CREATE INDEX idx_quarterly_financials_eps ON quarterly_financials(eps_calculated);
CREATE INDEX idx_quarterly_financials_stock_period ON quarterly_financials(stock_id, fiscal_year, fiscal_period);
```

### Future Schema Extensions (Schwab API)

```sql
-- Future: Real-time quotes table for live data
CREATE TABLE real_time_quotes (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER,
    timestamp TIMESTAMP,
    bid_price REAL,
    ask_price REAL,
    last_price REAL,
    volume INTEGER,
    change_amount REAL,
    change_percent REAL,
    FOREIGN KEY (stock_id) REFERENCES stocks (id)
);

-- Future: Option chains data  
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
    FOREIGN KEY (stock_id) REFERENCES stocks (id)
);
```

## Current Frontend Architecture - Expandable Panels System

### React Component Structure (Current Implementation)

```
frontend/src/
├── App.jsx                    # Main application - expandable panels system
├── components/
│   ├── StockRow.jsx          # Individual stock with expand controls
│   ├── ExpandablePanel.jsx   # Generic expandable container with animations
│   ├── AnalysisPanel.jsx     # User-driven metric analysis interface  
│   ├── DataFetchingPanel.jsx # Unified data fetching interface
│   └── [Legacy components preserved for future reference]
├── hooks/
│   ├── useStocks.js         # Stock data management
│   ├── useAnalysis.js       # Analysis calculations
│   └── useTauri.js          # Tauri API integration
└── utils/
    ├── formatters.js        # Data formatting utilities
    ├── calculations.js      # Financial calculations
    └── api.js              # API helper functions
```

### Current Features (Phase 3 Complete)
1. **Expandable Panel Interface**: Single-page stock list with contextual expansion
2. **User-Driven Analysis**: Dynamic metric selection (P/E, EPS, Price, Volume, etc.)  
3. **S&P 500 Filtering**: Toggle between all stocks and S&P 500 subset
4. **Paginated Stock Loading**: 50 stocks per page with load more functionality
5. **Real-Time Search**: Search stocks by symbol or company name
6. **Visual Data Indicators**: 📊 for stocks with data, 📋 for no data
7. **Multiple Panel Support**: Multiple stocks can have expanded panels simultaneously
8. **Smooth Animations**: Professional expand/collapse transitions

## Current Backend Architecture (Tauri + SimFin)

### Tauri Backend Structure

```rust
src-tauri/src/
├── main.rs                   # Tauri application entry point
├── lib.rs                    # Library exports
├── commands/
│   ├── mod.rs               # Commands module exports
│   ├── stocks.rs            # Stock information commands
│   ├── analysis.rs          # Data analysis commands
│   ├── fetching.rs          # Data fetching commands (legacy)
│   └── earnings.rs          # Earnings data commands
├── database/
│   ├── mod.rs              # Database management (SQLx-based)
│   ├── helpers.rs          # Database helper functions
│   ├── processing.rs       # Data processing operations
│   └── earnings.rs         # Earnings data operations
├── tools/
│   ├── mod.rs              # Tool modules
│   └── simfin_importer.rs  # SimFin CSV import system
├── bin/
│   └── import_simfin.rs    # SimFin import CLI tool
├── models/
│   └── mod.rs              # Data models and structures
├── analysis/
│   └── mod.rs              # Analysis engine
├── api/ (Future use)
│   ├── mod.rs              # API clients
│   ├── schwab_client.rs    # Schwab API client (preserved)
│   └── alpha_vantage_client.rs # Alpha Vantage client (legacy)
├── data_collector.rs        # Data collection logic
├── concurrent_fetcher.rs    # Concurrent processing utilities
└── utils.rs                 # Utility functions
```

### Current Tauri Commands (SimFin-Based)

```rust
// Stock information commands
#[tauri::command]
async fn get_stocks_paginated(limit: u32, offset: u32) -> Result<Vec<StockInfo>, String>

#[tauri::command]
async fn search_stocks(query: String) -> Result<Vec<StockInfo>, String>

#[tauri::command]
async fn get_stocks_with_data_status() -> Result<Vec<StockWithData>, String>

#[tauri::command]
async fn get_sp500_symbols() -> Result<Vec<String>, String>

// Analysis commands  
#[tauri::command]
async fn get_price_history(symbol: String, start_date: String, end_date: String) -> Result<Vec<PriceData>, String>

#[tauri::command]
async fn get_price_and_pe_data(symbol: String, start_date: String, end_date: String) -> Result<PriceAndPeData, String>

#[tauri::command]
async fn export_stock_data(symbol: String, format: String, start_date: String, end_date: String) -> Result<String, String>

// Database and statistics commands
#[tauri::command]
async fn get_database_stats() -> Result<DatabaseStats, String>

#[tauri::command]
async fn get_stock_summary(symbol: String) -> Result<StockSummary, String>

// Legacy/Future commands (preserved)
#[tauri::command]
async fn get_initialization_status() -> Result<InitProgress, String>

// SimFin Import (CLI tool - not Tauri command)
// cargo run --bin import-simfin -- --prices [CSV] --income [CSV]
```

### SimFin Import System

```rust
// Located in src-tauri/src/bin/import_simfin.rs and src-tauri/src/tools/simfin_importer.rs

pub async fn import_stocks_from_daily_prices(pool: &SqlitePool, csv_path: &str) -> Result<usize>
pub async fn import_daily_prices(pool: &SqlitePool, csv_path: &str, batch_size: usize) -> Result<usize>
pub async fn import_quarterly_financials(pool: &SqlitePool, csv_path: &str) -> Result<usize>
pub async fn calculate_and_store_eps(pool: &SqlitePool) -> Result<usize>
pub async fn calculate_and_store_pe_ratios(pool: &SqlitePool) -> Result<usize>
pub async fn add_performance_indexes(pool: &SqlitePool) -> Result<()>

// Usage:
// cargo run --bin import-simfin -- --prices ~/simfin_data/us-shareprices-daily.csv --income ~/simfin_data/us-income-quarterly.csv
```

## Current Implementation Status & Future Roadmap

### ✅ Completed Phases

**Phase 1: SimFin Data Infrastructure (COMPLETE)**
- ✅ Offline-first architecture with SimFin CSV import system
- ✅ Comprehensive database schema with calculated fundamentals
- ✅ 6-phase import process: Stock extraction → Price import → Financials → EPS calculation → P/E calculation → Performance indexing
- ✅ 5,876+ stocks with 6.2M price records and 52k+ financial records

**Phase 2: Modern Desktop Frontend (COMPLETE)**  
- ✅ Expandable panels UI system (single-page, contextual expansion)
- ✅ User-driven analysis (no artificial "basic vs enhanced" tiers)
- ✅ S&P 500 filtering and pagination system
- ✅ Real-time search and visual data indicators
- ✅ Professional animations and responsive design

**Phase 3: Backend Integration (COMPLETE)**
- ✅ Tauri commands for paginated stock loading
- ✅ Analysis commands for price history and P/E data
- ✅ Database statistics and stock summary commands
- ✅ Export functionality with multiple formats

### 🔄 Active Development

**Current Priority: S&P 500 Offline Support**
- 🔄 Fix database migration system (sector column error)
- 🔄 Create `sp500_symbols` table migration
- 🔄 Test offline S&P 500 functionality with ~500 symbols

### 🚀 Future Enhancements

**Phase 4: Advanced Analysis Tools**
1. **Technical Indicators**: Moving averages, RSI, MACD, Bollinger Bands
2. **Comparative Analysis**: Multi-stock comparison in expandable panels
3. **Sector Analysis**: Industry-wide trend analysis
4. **Portfolio Tracking**: Track and analyze custom stock portfolios

**Phase 5: Real-Time Features (Future Schwab API)**
1. **Real-Time Quotes**: Live price updates during market hours
2. **Options Data**: Options chain visualization and analysis
3. **Market News**: Real-time news feed integration
4. **Alert System**: Price and fundamental metric alerts

**Phase 6: Advanced Features**
1. **Custom Screening**: Build complex stock screens
2. **PDF Reports**: Export comprehensive analysis reports
3. **Data Sync**: Cloud backup and multi-device sync
4. **Advanced Charts**: Candlestick charts with overlays

## Data Import Usage

### SimFin Data Import Commands

```bash
# From project root directory (recommended)
cargo run --bin import-simfin -- \
  --prices ~/simfin_data/us-shareprices-daily.csv \
  --income ~/simfin_data/us-income-quarterly.csv \
  --db stocks.db

# Alternative: From src-tauri directory
cd src-tauri
cargo run --bin import_simfin -- \
  --prices ~/simfin_data/us-shareprices-daily.csv \
  --income ~/simfin_data/us-income-quarterly.csv \
  --db ../stocks.db
```

### Expected Performance
- **Data Processing**: 15-30 minutes for full dataset
- **Records Imported**: ~6.2M price records + ~52k financial records
- **Database Size**: 2-3 GB final size
- **EPS & P/E Calculations**: Automated during import

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

## Current System Performance

### Achieved Metrics
- **Data Coverage**: 5,876+ stocks with comprehensive historical data (2019-2024)
- **Database Performance**: Optimized with performance indexes for fast queries
- **UI Responsiveness**: <100ms response time for expandable panel interactions
- **Data Quality**: Professional-grade SimFin data with calculated fundamentals
- **Application Performance**: Smooth desktop application with paginated loading

### System Architecture Benefits
- **Offline-First**: Full functionality without internet connectivity
- **Comprehensive Data**: Both price and fundamental data in single system
- **Modern UI**: Expandable panels eliminate tab navigation complexity
- **Professional Quality**: SimFin institutional-grade financial data
- **Scalable Design**: Modular architecture supports future API integrations
- **Enterprise Safety**: Production-grade database safeguards and backup system

## Database Migration & Safety System

### Enterprise-Grade Database Protection

#### Database Manager
```rust
// Located in src-tauri/src/database/migrations.rs
pub struct DatabaseManager {
    pool: SqlitePool,
    db_path: String,
}

impl DatabaseManager {
    // Automatic backup before any operations
    pub async fn create_backup(db_path: &str) -> Result<String, Box<dyn std::error::Error>>
    
    // Production database detection with safeguards
    pub async fn verify_data_safety(&self) -> Result<DatabaseStats, Box<dyn std::error::Error>>
    
    // Safe migration runner with multiple verification steps
    pub async fn run_migrations_safely(&self) -> Result<(), Box<dyn std::error::Error>>
}
```

#### Safety Features
1. **Production Detection**: Automatically detects databases >50MB or >1000 stocks
2. **Automatic Backups**: Created before any schema changes with verification
3. **Data Integrity Verification**: Post-migration validation prevents data loss
4. **Rollback Support**: Timestamped backups for easy restoration
5. **Health Monitoring**: Real-time database statistics and alerts

#### Database Admin CLI Tool
```bash
# Check database health and statistics
cargo run --bin db_admin -- status
# Output: Shows stocks count, price records, size, and production warnings

# Create manual backup with verification
cargo run --bin db_admin -- backup
# Output: Timestamped backup in backups/ directory

# Run migrations with safety checks (requires explicit confirmation)
cargo run --bin db_admin -- migrate --confirm
# Output: Multi-layer backup creation, verification, and rollback capabilities

# Verify database integrity  
cargo run --bin db_admin -- verify
# Output: Comprehensive health check and data validation
```

#### Migration Safety Process
1. **Pre-Migration Backup**: Automatic backup with size verification
2. **Production Detection**: Large database warning with confirmation requirement
3. **Migration Execution**: SQLx migrations with error handling
4. **Post-Migration Verification**: Data integrity checks prevent silent data loss
5. **Cleanup**: Optional backup management (keeps last 5 backups)

#### Backup System
```bash
# Automatic backup script
./backup_database.sh

# Creates: backups/stocks_backup_YYYYMMDD_HHMMSS.db
# Includes: Size verification, integrity checks, automatic cleanup
```

### Migration Architecture

#### SQLx Migration System
- **Migration Files**: Located in `src-tauri/migrations/`
- **Automatic Tracking**: SQLx manages applied migrations in `_sqlx_migrations` table
- **Additive Only**: Migrations designed to add features, not destroy data
- **Production Safe**: Explicit confirmation required for large databases

#### Protected Initialization
```rust
// Located in src-tauri/src/database/protected_init.rs
pub async fn initialize_database_safely(db_path: &str) -> Result<SqlitePool, Box<dyn std::error::Error>>
pub async fn run_manual_migration(db_path: &str, confirm: bool) -> Result<(), Box<dyn std::error::Error>>
```

**Safety Levels:**
- **Small Databases** (<50MB, <100 stocks): Automatic migrations allowed
- **Medium Databases** (50MB-1GB, 100-1000 stocks): Backup + confirmation  
- **Production Databases** (>1GB, >1000 stocks): Manual confirmation required + multiple backups

#### Current Database Protection Status
```
Database: stocks.db (2,110.83 MB)
📊 Stocks: 5,892
📈 Price records: 6,198,657  
🏢 Financial records: 50,673
🚨 PRODUCTION DATABASE - Extra safeguards active
```

---
*Last Updated: 2025-09-09*
*Version: 3.1 - Added Enterprise Database Migration & Safety System*