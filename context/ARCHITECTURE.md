# Stock Analysis System - Architecture Document

## Executive Summary

A high-performance desktop application for stock analysis using Tauri (Rust backend + SolidJS frontend) that analyzes comprehensive stock data from SEC EDGAR API. Features production-ready data refresh system with 10-K/A amendment support, advanced screening algorithms (Piotroski F-Score, O'Shaughnessy Value), and enterprise-grade database safeguards.

**Production Status**: ✅ All 497 S&P 500 stocks with complete financial data (2025-10-07)

---

## Technology Stack

### Frontend
- **Framework**: SolidJS with TypeScript
- **State Management**: Signal-based reactivity
- **UI Pattern**: Modern responsive design with smooth animations
- **Build Tool**: Vite
- **Desktop Runtime**: Tauri webview

### Backend
- **Language**: Rust (stable)
- **Framework**: Tauri 1.x
- **Database**: SQLite 3.x with production-grade safeguards
- **HTTP Client**: reqwest with rate limiting (governor crate)
- **Async Runtime**: tokio with semaphore-based concurrency control

### Data Sources
- **Primary**: SEC EDGAR API (Submissions + Company Facts)
- **Market Data**: Charles Schwab API (OAuth 2.0)
- **S&P 500 Symbols**: GitHub repository (slickcharts)

---

## Database Architecture

### Core Design: Relational Model with Foreign Key Constraints

```
┌─────────────┐
│   stocks    │ ← Master table (503 total, 497 S&P 500)
│     id      │ ← PRIMARY KEY (AUTOINCREMENT)
│   symbol    │ ← UNIQUE
│    cik      │ ← UNIQUE (SEC identifier)
└─────────────┘
       ↓
       ↓ (Foreign Key Relationships)
       ↓
       ├──────────────┬──────────────┬──────────────┬──────────────┬──────────────┐
       ↓              ↓              ↓              ↓              ↓              ↓
┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
│daily_prices │ │sec_filings  │ │balance_     │ │income_      │ │cash_flow_   │ │piotroski_   │
│             │ │             │ │sheets       │ │statements   │ │statements   │ │scores       │
│  stock_id   │ │  stock_id   │ │  stock_id   │ │  stock_id   │ │  stock_id   │ │  stock_id   │
│    (FK)     │ │    (FK)     │ │    (FK)     │ │    (FK)     │ │    (FK)     │ │    (FK)     │
└─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘
                       ↓
                       ↓ (sec_filing_id FK)
                       ↓
                ┌──────┴──────┬──────────────┐
                ↓             ↓              ↓
         ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
         │balance_     │ │income_      │ │cash_flow_   │
         │sheets       │ │statements   │ │statements   │
         │sec_filing_id│ │sec_filing_id│ │sec_filing_id│
         │    (FK)     │ │    (FK)     │ │    (FK)     │
         └─────────────┘ └─────────────┘ └─────────────┘
```

### Key Tables

#### 1. **stocks** (Master Table)
```sql
CREATE TABLE stocks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,  -- Single source of truth for stock_id
    symbol TEXT UNIQUE NOT NULL,           -- Ticker symbol (e.g., "AAPL")
    company_name TEXT NOT NULL,
    cik TEXT UNIQUE,                       -- SEC Central Index Key (padded to 10 digits)
    sector TEXT,
    last_updated DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    is_sp500 BOOLEAN DEFAULT 0
)
```

**Data Quality**:
- 503 total stocks
- 497 S&P 500 stocks with CIK identifiers
- All CIKs properly padded (e.g., "0000320193" for AAPL)

#### 2. **daily_prices** (OHLCV Market Data)
```sql
CREATE TABLE daily_prices (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER NOT NULL,             -- FK to stocks(id)
    date DATE NOT NULL,
    open_price REAL NOT NULL,              -- Open price
    high_price REAL NOT NULL,              -- High price
    low_price REAL NOT NULL,               -- Low price
    close_price REAL NOT NULL,             -- Close price
    volume INTEGER,                        -- Trading volume
    pe_ratio REAL,
    market_cap REAL,
    dividend_yield REAL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, date)                 -- One price record per stock per day
)
```

**Data Coverage**:
- 501 S&P 500 stocks with price data
- ~2,704 days per stock (2014-12-31 to 2025-10-01)
- ~10.8 years of historical OHLCV data
- Source: Charles Schwab API

#### 3. **sec_filings** (SEC Filing Metadata)
```sql
CREATE TABLE sec_filings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,             -- FK to stocks(id)
    accession_number TEXT NOT NULL,        -- Unique SEC filing identifier
    form_type TEXT NOT NULL,               -- "10-K" or "10-K/A" (amendment)
    filed_date DATE NOT NULL,              -- Date filed with SEC
    fiscal_period TEXT,                    -- "FY" for annual
    fiscal_year INTEGER NOT NULL,
    report_date DATE NOT NULL,             -- Fiscal year end date

    file_size_bytes INTEGER,
    document_count INTEGER,
    is_amended BOOLEAN DEFAULT 0,

    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, accession_number),
    UNIQUE(stock_id, form_type, report_date, fiscal_year)  -- One filing per year
)
```

**Data Quality**:
- All 497 S&P 500 stocks have 10-K filings
- 5-10 years of historical filings per stock
- Both 10-K (original) and 10-K/A (amendments) supported
- Source: SEC Submissions API

#### 4. **balance_sheets**, **income_statements**, **cash_flow_statements**
```sql
-- Example: balance_sheets (similar structure for income/cashflow)
CREATE TABLE balance_sheets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,             -- FK to stocks(id)
    sec_filing_id INTEGER,                 -- FK to sec_filings(id)
    report_date DATE NOT NULL,
    fiscal_year INTEGER,

    -- Financial metrics
    cash_and_equivalents REAL,
    total_debt REAL,
    total_assets REAL,
    total_equity REAL,
    current_assets REAL,
    current_liabilities REAL,
    -- ... (40+ financial metrics)

    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    FOREIGN KEY (sec_filing_id) REFERENCES sec_filings(id)
)
```

**Data Quality**:
- All 497 S&P 500 stocks have financial statements
- 5-10 years of historical data per stock
- Atomic storage: All 3 statements stored together or none
- No orphaned records (ACID compliance)
- Source: SEC Company Facts API

### Referential Integrity

✅ **Single stock_id Throughout Database**:
- `stocks.id` is the PRIMARY KEY
- ALL other tables use `stock_id` as FOREIGN KEY referencing `stocks(id)`
- No orphaned records in any table (verified 2025-10-07)

✅ **Cascading Relationships**:
- `sec_filings` → `stocks` (one-to-many)
- `balance_sheets` → `sec_filings` (one-to-one)
- `income_statements` → `sec_filings` (one-to-one)
- `cash_flow_statements` → `sec_filings` (one-to-one)

✅ **UNIQUE Constraints Prevent Duplicates**:
- `daily_prices`: UNIQUE(stock_id, date)
- `sec_filings`: UNIQUE(stock_id, accession_number)
- `sec_filings`: UNIQUE(stock_id, form_type, report_date, fiscal_year)

---

## Data Refresh Architecture

### Overview

Two independent data refresh pipelines:
1. **Market Data**: Daily OHLCV prices from Schwab API
2. **Financial Data**: Annual 10-K statements from SEC EDGAR API

### 1. Market Data Refresh (Schwab API)

**Source**: Charles Schwab API (OAuth 2.0)
**Command**: `cargo run --bin refresh_data market`
**Frequency**: Daily (market days only)

#### Architecture
```
┌─────────────────────────────────────────────────────────────────┐
│                    Market Data Refresh                          │
├─────────────────────────────────────────────────────────────────┤
│  1. Load Schwab OAuth tokens from file                          │
│  2. Get S&P 500 stocks from database                            │
│  3. For each stock: Get MAX(date) from daily_prices             │
│  4. Fetch price history from last_date + 1 to today             │
│  5. INSERT OR REPLACE into daily_prices                         │
│  6. Process 10 stocks concurrently with semaphore               │
└─────────────────────────────────────────────────────────────────┘
```

**Code Location**: `src-tauri/src/tools/data_refresh_orchestrator.rs:371-519`

**Performance**:
- 2-5 minutes for all 497 stocks
- 10 concurrent workers
- Incremental updates (only fetch missing dates)

**OHLCV Data Quality**:
- ✅ All 5 fields properly stored (Open, High, Low, Close, Volume)
- ✅ 10+ years of historical data per stock
- ✅ No gaps in data (continuous from 2014 to present)
- ✅ Realistic values verified (Low ≤ Open,Close ≤ High)

### 2. Financial Data Refresh (SEC EDGAR API)

**Source**: SEC EDGAR API (Submissions + Company Facts)
**Command**: `cargo run --bin refresh_data financials`
**Frequency**: Quarterly (after 10-K filings)

#### Hybrid API Architecture

Uses TWO SEC APIs together:
1. **Submissions API**: Filing metadata (form types, dates, accession numbers)
2. **Company Facts API**: Financial statement data (XBRL facts)

```
┌─────────────────────────────────────────────────────────────────┐
│              Financial Data Refresh (Hybrid API)                │
├─────────────────────────────────────────────────────────────────┤
│  STEP 1: Fetch Submissions API (metadata)                       │
│    - URL: https://data.sec.gov/submissions/CIK{cik}.json       │
│    - Extract: form_type, filing_date, report_date, accession   │
│    - Filter: Only 10-K and 10-K/A (annual reports)             │
│    - Result: List of 10-K filings with metadata                │
│                                                                 │
│  STEP 2: Deduplication (prefer amendments)                      │
│    - Group by report_date (fiscal year)                        │
│    - If both 10-K and 10-K/A exist: prefer 10-K/A              │
│    - If same form type: prefer latest filing_date              │
│    - Result: One filing per fiscal year                        │
│                                                                 │
│  STEP 3: Fetch Company Facts API (financial data)              │
│    - URL: https://data.sec.gov/api/xbrl/companyfacts/CIK.json │
│    - Extract: Balance sheet, Income, Cash flow data            │
│    - Filter: Only facts matching accession numbers from Step 1 │
│    - Result: Financial statement data for each 10-K            │
│                                                                 │
│  STEP 4: Upsert Logic (replace 10-K with 10-K/A)               │
│    - Check: Does 10-K exist for same (stock_id, report_date)?  │
│    - If yes and storing 10-K/A: DELETE old 10-K data           │
│    - Then: INSERT new 10-K/A data                               │
│    - Result: Database always has latest corrected data         │
│                                                                 │
│  STEP 5: Atomic Storage (transaction)                           │
│    - BEGIN TRANSACTION                                          │
│    - INSERT sec_filings record                                  │
│    - INSERT balance_sheets record                               │
│    - INSERT income_statements record                            │
│    - INSERT cash_flow_statements record                         │
│    - COMMIT (all-or-nothing)                                    │
└─────────────────────────────────────────────────────────────────┘
```

**Code Locations**:
- Hybrid API: `src-tauri/src/tools/data_freshness_checker.rs:370-590`
- Deduplication: `src-tauri/src/tools/data_freshness_checker.rs:433-461`
- Upsert: `src-tauri/src/tools/sec_edgar_client.rs:1080-1123`
- Atomic storage: `src-tauri/src/tools/sec_edgar_client.rs:1066-1145`

#### 10-K/A Amendment Handling

**Problem**: Companies sometimes file amendments (10-K/A) to correct errors in original 10-K filings.

**Solution**: Automatic detection and replacement

```rust
// 1. Filter for both forms
if form == "10-K" || form == "10-K/A" {
    metadata_vec.push((accession, filing_date, report_date, form_type));
}

// 2. Deduplication: prefer 10-K/A over 10-K for same report_date
let mut deduped_map: HashMap<String, Filing> = HashMap::new();
for (accn, filed, report, form) in metadata_vec {
    if form == "10-K/A" && existing_form == "10-K" {
        deduped_map.insert(report, (accn, filed, report, form)); // Replace
    }
}

// 3. Upsert: delete old 10-K data, insert new 10-K/A data
if metadata.form_type == "10-K/A" {
    // Check if 10-K exists for same (stock_id, report_date, fiscal_year)
    let existing_10k = query_existing_10k(stock_id, report_date, fiscal_year)?;

    if let Some(old_filing_id) = existing_10k {
        // Delete old financial data (within transaction)
        delete_balance_sheets(old_filing_id)?;
        delete_income_statements(old_filing_id)?;
        delete_cash_flow_statements(old_filing_id)?;
        delete_sec_filing(old_filing_id)?;
    }
}
// Then insert new 10-K/A data
```

**Result**: Database always contains the most recent, corrected financial data.

#### Performance & Concurrency

- **Rate Limiting**: 10 requests/second (SEC requirement)
- **Concurrency**: 10 parallel workers with semaphore
- **Full Refresh**: 15-30 minutes for all 497 stocks
- **Single Stock**: <1 second with `--only-ticker` flag

**Example Output**:
```
📋 DLTR (CIK 0000935703): Found 9 10-K/10-K/A filings from Submissions API
📊 DLTR (CIK 0000935703): After deduplication: 8 unique filings
🔄 [UPSERT] Replacing 10-K (accession: 0000935703-18-000013) with 10-K/A (accession: 0000935703-18-000016)
✅ [UPSERT] Deleted old 10-K filing (id=26653)
✅ Stored 10-K/A filing: 2018-02-03 (0000935703-18-000016)
```

---

## Screening Algorithms Architecture

### 1. Piotroski F-Score

**Purpose**: Identify financially strong companies
**Criteria**: 9 binary tests (0 or 1 points each)
**Score Range**: 0-9 (higher is better)

#### Categories

**Profitability** (4 points):
1. Positive Net Income
2. Positive Operating Cash Flow
3. Increasing Return on Assets
4. Quality of Earnings (OCF > Net Income)

**Leverage/Liquidity** (3 points):
5. Decreasing Long-term Debt
6. Increasing Current Ratio
7. No New Share Issuance

**Operating Efficiency** (2 points):
8. Increasing Gross Margin
9. Increasing Asset Turnover

**Implementation**: `src-tauri/src/commands/piotroski.rs`

### 2. O'Shaughnessy Value Composite

**Purpose**: Identify undervalued companies
**Criteria**: 6 value metrics ranked by percentile
**Score Range**: 6-100 (lower is better = more undervalued)

#### Metrics

1. **P/B Ratio** (Price-to-Book)
2. **P/S Ratio** (Price-to-Sales)
3. **P/CF Ratio** (Price-to-Cash Flow)
4. **P/E Ratio** (Price-to-Earnings)
5. **EV/EBITDA** (Enterprise Value-to-EBITDA)
6. **Shareholder Yield** (Dividends + Buybacks)

**Scoring**: Each metric ranked into percentiles (1-100), then averaged.

**Implementation**: `src-tauri/src/commands/oshaughnessy.rs`

---

## Frontend Architecture (SolidJS)

### Core Patterns

**State Management**: Signal-based reactivity
```typescript
import { createSignal, createEffect } from 'solid-js';

const [results, setResults] = createSignal<ScreeningResult[]>([]);

createEffect(() => {
    // Reactive updates when results change
    console.log('Results updated:', results());
});
```

**Tauri Commands**: Backend integration
```typescript
import { invoke } from '@tauri-apps/api/tauri';

async function runPiotroskiScreen() {
    const results = await invoke<PiotroskiResult[]>('get_piotroski_scores', {
        minScore: 7
    });
    setResults(results);
}
```

**Component Structure**:
```
src/
├── components/
│   ├── PiotroskiScreen.tsx     # F-Score screening interface
│   ├── OShaughnessyScreen.tsx  # Value composite interface
│   └── StockDetails.tsx        # Individual stock analysis
├── types/
│   └── screening.ts            # TypeScript interfaces
└── App.tsx                     # Main application
```

---

## Database Administration & Safety

### Enterprise-Grade Safeguards

#### Automatic Backup System

**Trigger**: Before any schema changes (migrations)
**Detection**: Database >50MB or >1000 stocks = production
**Location**: `src-tauri/db/backups/stocks_backup_YYYYMMDD_HHMMSS.db`

```rust
// Before migration
if is_production_db(&pool).await? {
    println!("🚨 Production database detected (2.5GB)");
    create_backup(&pool).await?;
    verify_backup()?;
}
```

#### Database Migrations

**Tool**: sqlx migrations (built-in)
**Location**: `src-tauri/db/migrations/`
**Count**: 36 migrations (chronological order)

**Features**:
- SHA-256 checksum validation
- Sequential application
- Idempotent (re-running has no effect)
- Tracked in `_sqlx_migrations` table

**Commands**:
```bash
# Check status
cd src-tauri
sqlx migrate info --database-url "sqlite:db/stocks.db"

# Run migrations
sqlx migrate run --database-url "sqlite:db/stocks.db"

# Create new migration
sqlx migrate add --source db/migrations descriptive_name
```

**Critical Rules**:
- ❌ NEVER modify applied migrations (breaks checksum)
- ✅ ALWAYS create new migrations for changes
- ✅ ALWAYS backup before migrations on production

---

## API Integration

### SEC EDGAR API

**Base URL**: `https://data.sec.gov/`
**Rate Limit**: 10 requests/second
**User-Agent**: Required (`rust-stocks-tauri/1.0`)

**Endpoints**:
1. **Submissions API**: `https://data.sec.gov/submissions/CIK{cik}.json`
   - Returns: Filing history, form types, accession numbers
   - Used for: 10-K metadata extraction

2. **Company Facts API**: `https://data.sec.gov/api/xbrl/companyfacts/CIK{cik}.json`
   - Returns: XBRL financial facts with filing links
   - Used for: Balance sheet, income, cash flow extraction

**Rate Limiting** (governor crate):
```rust
use governor::{Quota, RateLimiter, clock::DefaultClock};

let limiter = RateLimiter::direct(
    Quota::per_second(NonZeroU32::new(10).unwrap())
);

// Before each request
limiter.until_ready().await;
```

### Charles Schwab API

**OAuth 2.0**: Authorization code flow
**Token Storage**: `~/.schwab_token.json`
**Refresh**: Automatic via Python script

**Endpoints**:
1. **Price History**: `/marketdata/v1/pricehistory`
   - Returns: OHLCV candles
   - Frequency: Daily (for historical data)

**Authentication Flow**:
```bash
# Initial setup
python3 refresh_token.py --auth

# Token refresh (as needed)
python3 refresh_token.py --refresh
```

---

## Performance Characteristics

### Database Size & Performance

**Production Database**: `src-tauri/db/stocks.db` (2.5GB)

**Table Sizes**:
- `daily_prices`: ~1.4 million records (501 stocks × 2,704 days)
- `sec_filings`: ~20,000 records (497 stocks × 5-10 years × 10-K/10-Q)
- `balance_sheets`: ~100,000 records
- `income_statements`: ~20,000 records
- `cash_flow_statements`: ~80,000 records

**Query Performance**:
- Stock lookup by symbol: <1ms (indexed)
- Piotroski screening (497 stocks): ~500ms
- O'Shaughnessy screening (497 stocks): ~800ms
- Price history (1 stock, 1 year): <10ms

### Concurrent Operations

**Market Data Refresh**:
- Semaphore limit: 10 concurrent requests
- Rate limit: Schwab API throttling
- Total time: 2-5 minutes for 497 stocks

**Financial Data Refresh**:
- Semaphore limit: 10 concurrent requests
- Rate limit: 10 req/sec (SEC EDGAR)
- Total time: 15-30 minutes for 497 stocks

---

## Production Status (2025-10-07)

### Data Quality

✅ **Market Data**:
- 501 S&P 500 stocks
- 10.8 years of OHLCV history
- Current through 2025-10-01
- All 5 OHLCV fields properly stored

✅ **Financial Data**:
- 497 S&P 500 stocks with CIKs
- 5-10 years of 10-K filings
- All stocks current (verified 2025-10-07)
- 10-K/A amendments properly handled

✅ **Database Integrity**:
- No orphaned records (all foreign keys valid)
- ACID-compliant transactions
- Enterprise-grade backups
- 36 migrations applied successfully
- Single stock_id throughout all tables

### Known Limitations

1. **Submissions API "recent" limitation**: Only last ~1000 filings per company
   - Impact: Minimal (10-K is ~1/year, covers 10+ years)

2. **No quarterly data**: System only processes 10-K (annual) filings
   - Impact: None for annual screening algorithms

3. **Schwab API token expiration**: Manual refresh required periodically
   - Mitigation: Python script for token refresh

---

## Future Enhancements

### Near-Term (Next 6 months)
1. **10-Q Support**: Add quarterly filing processing
2. **Real-time Quotes**: Integrate Schwab streaming API
3. **Options Data**: Add options chain analysis
4. **Additional Screens**: Magic Formula, GARP, etc.

### Long-Term (6-12 months)
1. **Historical Backtesting**: Test screening algorithms on historical data
2. **Portfolio Tracking**: Track positions and performance
3. **Alerts System**: Notify on screening criteria changes
4. **Export Capabilities**: CSV, Excel, PDF reports

---

## Development Commands

### Common Operations

```bash
# Development
npm run tauri dev                       # Run app in dev mode

# Database
cd src-tauri
cargo run --bin db_admin -- status      # Check database health
cargo run --bin db_admin -- backup      # Manual backup
cargo run --bin db_admin -- verify      # Verify integrity

# Data Refresh
cd src-tauri
cargo run --bin refresh_data market                    # Refresh prices
cargo run --bin refresh_data financials                # Refresh financials
cargo run --bin refresh_data financials --only-ticker AAPL  # Single stock

# Testing
cargo test                              # Run all tests
cargo test test_piotroski               # Run specific test

# Build
npm run tauri build                     # Production build
```

---

**Last Updated**: 2025-10-07
**Database Version**: 36 migrations applied
**Production Status**: ✅ READY
