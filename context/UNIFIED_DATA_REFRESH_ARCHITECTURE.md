# Unified Data Refresh Architecture

## Overview
A comprehensive data refresh system that ensures all screening features have current, complete data before analysis.

## Current Screening Features & Data Dependencies

### 1. GARP P/E Screening (`commands/garp_pe.rs`)
**Data Sources Required:**
- `daily_valuation_ratios` → P/E ratios, current prices, market caps
- `income_statements` → TTM & Annual revenue, net income, shares
- `balance_sheets` → TTM debt, equity data

**Generated Views:**
- `garp_pe_screening_data` → Comprehensive GARP metrics
- `peg_ratio_analysis` → PEG ratios and screening filters

### 2. Graham Value Screening (`commands/graham_screening.rs`)
**Data Sources Required:**
- `daily_prices` → Current stock prices
- `daily_valuation_ratios` → P/E ratios
- `income_statements` → Annual/TTM revenue, earnings, operating income
- `balance_sheets` → Total debt, equity, assets, cash

**Generated Tables:**
- `graham_screening_results` → Historical screening results
- `v_graham_screening_stats` → Aggregated statistics

### 3. Valuation Analysis (`commands/analysis.rs`)
**Data Sources Required:**
- `daily_valuation_ratios` → P/S, EV/S ratios, enterprise values
- `income_statements` → Revenue data for ratio calculations

**Generated Tables:**
- `enterprise_value_analysis` → EV calculations
- `revenue_growth_analysis` → Growth metrics

## Current Data Update Processes

### Price Data Pipeline
```
Schwab API → import-schwab-prices → daily_prices (2.5M records)
```

### Financial Data Pipeline
```
EDGAR API → concurrent-edgar-extraction → income_statements + balance_sheets (54K records)
```

### Calculated Ratios Pipeline
```
daily_prices + income_statements → run_pe_calculation → daily_valuation_ratios.pe_ratio
daily_prices + income_statements → calculate-ratios → daily_valuation_ratios.ps_ratio_*, evs_ratio_*
```

## Problem: Data Staleness
- **Price data**: Current (2025-08-22)
- **P/E ratios**: Stale (2024-09-13) ❌
- **P/S/EV/S ratios**: Current (2025-09-11) ✅
- **Financial data**: Current (TTM through Q2 2024) ✅

## Unified Refresh Architecture

### Core Design Principles
1. **Dependency Order**: Update base data before calculated data
2. **Incremental Updates**: Only refresh what's needed
3. **Progress Reporting**: Real-time status for UI
4. **Error Resilience**: Graceful degradation on partial failures
5. **Configurable Modes**: Quick vs. Full refresh options

### Architecture Components

#### 1. Data Refresh Orchestrator (`src/tools/data_refresh_orchestrator.rs`)
**Responsibilities:**
- Coordinate all refresh operations in proper dependency order
- Track progress and report status to UI
- Handle errors and partial failures
- Manage different refresh modes

**Refresh Modes:**
- `Quick`: Price data + ratio calculations (daily use)
- `Standard`: Price + ratios + recent financial data (weekly)
- `Full`: Complete refresh including historical data (monthly)

#### 2. Data Freshness Checker (`src/tools/data_freshness_checker.rs`)
**Responsibilities:**
- Check staleness of each data source
- Determine which components need refresh
- Validate data completeness and quality

**Freshness Criteria:**
- Price data: Updated within 1 trading day
- P/E ratios: Within 2 trading days of latest prices
- P/S ratios: Within 1 week of latest prices
- Financial data: Within current quarter

#### 3. Price Data Refresher (`src/tools/price_data_refresher.rs`)
**Responsibilities:**
- Wrapper around existing `import-schwab-prices` logic
- Incremental updates using our new date range calculator
- Progress reporting

**Implementation:**
```rust
use crate::tools::date_range_calculator::DateRangeCalculator;

impl PriceDataRefresher {
    async fn refresh_incremental(&self) -> RefreshResult {
        // Use existing incremental logic
        // Report progress to orchestrator
    }
}
```

#### 4. Ratio Calculator Refresher (`src/tools/ratio_calculator_refresher.rs`)
**Responsibilities:**
- Wrapper around `run_pe_calculation` and `calculate-ratios`
- Only calculate for dates missing ratios
- Ensure dependency order (P/E before P/S)

#### 5. Financial Data Refresher (`src/tools/financial_data_refresher.rs`) *[Future]*
**Responsibilities:**
- Direct EDGAR API integration for quarterly updates
- Incremental financial statement downloads
- Replace manual `concurrent-edgar-extraction` runs

#### 6. Progress Reporter (`src/tools/refresh_progress_reporter.rs`)
**Responsibilities:**
- Websocket/SSE communication with frontend
- Progress tracking with detailed status
- Error reporting and recovery suggestions

### Database Schema Extensions

#### Data Freshness Tracking Table
```sql
CREATE TABLE data_refresh_status (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    data_source TEXT NOT NULL,           -- 'prices', 'pe_ratios', 'financial', etc.
    last_refresh_start DATETIME,
    last_refresh_complete DATETIME,
    last_successful_refresh DATETIME,
    next_recommended_refresh DATETIME,
    refresh_status TEXT,                 -- 'current', 'stale', 'refreshing', 'error'
    records_updated INTEGER,
    latest_data_date DATE,
    error_message TEXT,
    refresh_duration_seconds INTEGER
);
```

#### Progress Tracking Table
```sql
CREATE TABLE refresh_progress (
    session_id TEXT PRIMARY KEY,
    start_time DATETIME DEFAULT CURRENT_TIMESTAMP,
    total_steps INTEGER,
    completed_steps INTEGER,
    current_step_name TEXT,
    current_step_progress REAL,
    estimated_completion DATETIME,
    status TEXT,                         -- 'running', 'completed', 'error', 'cancelled'
    error_details TEXT
);
```

## Binary Implementation: `refresh_data`

### Command Structure
```bash
# Quick refresh (prices + ratios only)
cargo run --bin refresh_data

# Standard refresh (prices + ratios + recent financials)
cargo run --bin refresh_data --mode standard

# Full refresh (everything including historical data)
cargo run --bin refresh_data --mode full

# Check status only
cargo run --bin refresh_data --status

# Force refresh specific components
cargo run --bin refresh_data --force prices,ratios
```

### Core Implementation Structure
```rust
// src/bin/refresh_data.rs
use clap::{Parser, ValueEnum};
use rust_stocks_tauri_lib::tools::{
    data_refresh_orchestrator::DataRefreshOrchestrator,
    refresh_progress_reporter::ProgressReporter,
};

#[derive(Parser)]
#[command(about = "Unified data refresh system for stock analysis")]
struct Cli {
    #[arg(long, value_enum, default_value = "quick")]
    mode: RefreshMode,

    #[arg(long)]
    status: bool,

    #[arg(long, value_delimiter = ',')]
    force: Vec<String>,

    #[arg(long)]
    progress_websocket: Option<String>,
}

#[derive(Clone, ValueEnum)]
enum RefreshMode {
    Quick,    // Prices + ratios (5-10 min)
    Standard, // + Recent financials (15-30 min)
    Full,     // + Historical data (1-2 hours)
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.status {
        return show_refresh_status().await;
    }

    let orchestrator = DataRefreshOrchestrator::new().await?;
    let progress_reporter = ProgressReporter::new(cli.progress_websocket);

    orchestrator.execute_refresh(cli.mode, cli.force, progress_reporter).await
}
```

## Integration with Screening Commands

### Pre-Analysis Data Validation
All screening commands will check data freshness before execution:

```rust
// In garp_pe.rs, graham_screening.rs, etc.
#[tauri::command]
pub async fn get_garp_pe_screening_results(
    stock_tickers: Vec<String>,
    criteria: Option<GarpPeScreeningCriteria>,
    limit: Option<i32>
) -> Result<Vec<GarpPeScreeningResult>, String> {

    // Check data freshness first
    let freshness_checker = DataFreshnessChecker::new().await?;
    let status = freshness_checker.check_garp_data_freshness().await?;

    if !status.is_current() {
        return Err(format!(
            "Data refresh required: {}. Please run 'refresh_data' first.",
            status.get_stale_components_message()
        ));
    }

    // Proceed with analysis...
}
```

### UI Integration Strategy

#### Frontend Data Refresh Flow
1. **Check Status**: Before any screening, check refresh status
2. **Show Refresh UI**: If stale, show refresh progress modal
3. **Real-time Updates**: WebSocket progress updates during refresh
4. **Auto-retry**: After refresh completion, retry original analysis

#### Status API Endpoints
```rust
#[tauri::command]
pub async fn get_data_refresh_status() -> Result<DataRefreshStatus, String> {
    // Return current freshness status for all components
}

#[tauri::command]
pub async fn start_data_refresh(mode: RefreshMode) -> Result<String, String> {
    // Start refresh and return session ID for progress tracking
}

#[tauri::command]
pub async fn get_refresh_progress(session_id: String) -> Result<RefreshProgress, String> {
    // Get current progress for UI updates
}
```

## Error Handling & Recovery

### Graceful Degradation
- If price refresh fails → Use existing price data with warning
- If ratio calculation fails → Skip affected screening features
- If financial refresh fails → Use existing financial data

### Recovery Strategies
- Retry failed components automatically
- Provide manual retry options in UI
- Fall back to cached data with staleness warnings

## Performance Optimizations

### Comprehensive Parallel Processing Architecture (September 2025)

#### Database Connection Pool Optimization
**SQLite Connection Pool with WAL Mode:**
```rust
// Optimized for parallel processing (50 connections vs previous 5)
SqlitePoolOptions::new()
    .max_connections(50)  // 10x increase for high concurrency
    .acquire_timeout(std::time::Duration::from_secs(30))

// WAL Mode + Performance Tuning:
"PRAGMA journal_mode = WAL"      // Enable concurrent reads during writes
"PRAGMA synchronous = NORMAL"    // Reduce fsync waits (safe mode)
"PRAGMA cache_size = 10000"      // 10MB cache for better performance
"PRAGMA temp_store = memory"     // Store temp data in memory
```

#### Parallel EDGAR Data Extraction
**Architecture:** 20 concurrent workers with semaphore limiting
```rust
let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(20));
// Each worker processes one S&P 500 company independently
// Expected speedup: 15-20x (503 companies: ~503s → ~25-30s)
```

#### Parallel P/E Ratio Calculation
**Architecture:** Stock-level parallelization with batch processing
```rust
// EPS Calculation: 30 concurrent workers
let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(30));
// Batch UPDATE operations per stock vs individual record processing

// P/E Calculation: 25 concurrent workers
let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(25));
// Batch INSERT/REPLACE with complex joins per stock
// Expected speedup: 50-100x (6.2M records: hours → minutes)
```

#### Parallel Market Data Refresh
**Architecture:** 10 concurrent workers (API rate limited)
```rust
let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(10));
// Achieved: 8.6x speedup (503 stocks: 8+ minutes → 58 seconds)
```

#### Intelligent Concurrency Tuning
**Worker Allocation Strategy:**
- **EDGAR Extraction**: 20 workers (file I/O + disk bound operations)
- **EPS Calculation**: 30 workers (database write intensive operations)
- **P/E Calculation**: 25 workers (complex joins + batch writes)
- **Market Data**: 10 workers (external API rate limited)

#### Performance Benchmarks Achieved
**Market Data Refresh:**
- **Before**: 503+ seconds (sequential, 1 second per stock)
- **After**: 58 seconds (parallel, 69,464 records)
- **Improvement**: 8.6x speedup

**Expected Performance Gains:**
- **EDGAR Extraction**: 15-20x improvement (503 companies)
- **P/E Calculation**: 50-100x improvement (6.2M price records)
- **EPS Calculation**: 20-30x improvement (TTM financial data)

#### Technical Implementation Details
**Data Consistency Maintained Through:**
- **Atomic Operations**: Each worker processes complete units (stocks, companies)
- **Error Isolation**: Failed tasks don't affect successful parallel operations
- **Progress Tracking**: Real-time feedback on parallel operation status
- **Resource Management**: Semaphores prevent database connection exhaustion

**SQLite Concurrency Research Findings:**
- **No hard connection limit** - System handles ~1,000 connections per process
- **Single writer constraint** - Only one write transaction at a time
- **WAL mode benefits** - Allows concurrent reads during writes
- **Optimal pool size** - 50 connections for read-heavy workloads with parallel processing

### Caching Strategy
- Cache expensive calculations during refresh
- Store intermediate results for faster retries
- Use materialized views for complex queries

## Future Extensions

### Additional Data Sources
- Real-time market data feeds
- Alternative financial data providers
- ESG and sustainability metrics
- Options and derivatives data

### Advanced Scheduling
- Cron-like scheduling for automatic refreshes
- Market hours awareness (no updates during trading)
- Holiday calendar integration

### Data Quality Monitoring
- Anomaly detection in refreshed data
- Data validation rules and alerts
- Historical data integrity checks

## Implementation Timeline

### Phase 1: Core Infrastructure (Week 1-2)
- [ ] Data freshness checker
- [ ] Progress reporter
- [ ] Basic orchestrator

### Phase 2: Price & Ratio Integration (Week 3)
- [ ] Price data refresher wrapper
- [ ] Ratio calculator integration
- [ ] Database schema updates

### Phase 3: Screening Integration (Week 4)
- [ ] Pre-analysis validation
- [ ] Error handling
- [ ] UI status endpoints

### Phase 4: Advanced Features (Week 5+)
- [ ] Financial data refresher
- [ ] Advanced scheduling
- [ ] Performance optimizations

## Success Metrics

### Data Quality
- 99%+ screening data freshness
- <2 hour staleness for price data
- <24 hour staleness for calculated ratios

### Performance (Updated with Parallel Architecture)
- **Quick refresh**: <3 minutes (was <10 minutes)
- **Standard refresh**: <8 minutes (was <30 minutes)
- **Full refresh**: <20 minutes (was 1-2 hours)
- **Market data refresh**: <1 minute (503 stocks)
- **EDGAR extraction**: <30 seconds (503 companies)
- **P/E calculation**: <2 minutes (6.2M records)
- **95% success rate** for all refresh operations

### User Experience
- One-click data refresh from UI
- Real-time progress feedback
- Automatic retry on failures
- Clear error messages and recovery guidance