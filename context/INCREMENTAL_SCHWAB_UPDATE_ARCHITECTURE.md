# Incremental Schwab Price Data Update - Architecture & Implementation

## 🎯 Objectives

### Performance Goals
- **Incremental Updates**: Only fetch missing data since last update
- **Dynamic Date Ranges**: Use current date as end date, optimize start dates
- **API Efficiency**: Minimize API calls by skipping existing data
- **Smart Resumption**: Handle interrupted imports gracefully

### Data Management Goals
- **IPO/Listing Tracking**: Store and use company listing dates for optimal date ranges
- **Gap Detection**: Identify and fill missing trading days
- **Historical Preservation**: Maintain existing data while adding new records
- **Real-time Updates**: Support daily/weekly incremental updates

## 🏗️ Architecture Design

### High-Level Architecture
```
┌─────────────────────────────────────────────────────────────────────────┐
│                 Incremental Schwab Update System                        │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌─────────────────┐    ┌──────────────────┐    ┌──────────────────┐   │
│  │   Company       │    │   Date Range     │    │   Update         │   │
│  │   Metadata      │───▶│   Calculator     │───▶│   Coordinator    │   │
│  │   (IPO dates)   │    │                  │    │                  │   │
│  └─────────────────┘    └──────────────────┘    └──────────────────┘   │
│           │                        │                        │           │
│           ▼                        ▼                        ▼           │
│  ┌─────────────────┐    ┌──────────────────┐    ┌──────────────────┐   │
│  │   Database      │    │   Gap            │    │   Progress       │   │
│  │   Coverage      │◀──▶│   Detection      │◀──▶│   Tracking       │   │
│  │   Analysis      │    │                  │    │                  │   │
│  └─────────────────┘    └──────────────────┘    └──────────────────┘   │
│           │                        │                        │           │
│           ▼                        ▼                        ▼           │
│  ┌─────────────────┐    ┌──────────────────┐    ┌──────────────────┐   │
│  │   Schwab API    │    │   Database       │    │   Validation     │   │
│  │   Client        │    │   Writer         │    │   & Reporting    │   │
│  │                 │    │                  │    │                  │   │
│  └─────────────────┘    └──────────────────┘    └──────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

## 📊 Database Schema Enhancements

### 1. Company Metadata Table
```sql
-- Store IPO/listing dates and metadata
CREATE TABLE company_metadata (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL UNIQUE,
    symbol TEXT NOT NULL,
    company_name TEXT NOT NULL,

    -- Key dates for data range optimization
    ipo_date DATE,                    -- Initial public offering date
    listing_date DATE,                -- First trading date (may differ from IPO)
    delisting_date DATE,              -- If delisted (for historical symbols)
    spinoff_date DATE,                -- If spun off from parent company

    -- Data coverage tracking
    earliest_data_date DATE,          -- Earliest available price data
    latest_data_date DATE,            -- Latest available price data
    total_trading_days INTEGER,       -- Total trading days with data

    -- Metadata
    exchange TEXT,                    -- Primary exchange (NYSE, NASDAQ, etc.)
    sector TEXT,                     -- Business sector
    market_cap_category TEXT,        -- Large/Mid/Small cap
    data_source TEXT DEFAULT 'schwab', -- Primary data source

    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    INDEX idx_company_metadata_symbol (symbol),
    INDEX idx_company_metadata_ipo_date (ipo_date),
    INDEX idx_company_metadata_listing_date (listing_date)
);
```

### 2. Data Coverage Tracking Table
```sql
-- Track data coverage gaps and update status
CREATE TABLE price_data_coverage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    symbol TEXT NOT NULL,

    -- Coverage analysis
    expected_start_date DATE NOT NULL, -- Based on IPO/listing date
    actual_start_date DATE,            -- Earliest data we have
    expected_end_date DATE NOT NULL,   -- Current date or delisting
    actual_end_date DATE,              -- Latest data we have

    -- Gap tracking
    total_expected_days INTEGER,       -- Expected trading days
    total_actual_days INTEGER,         -- Actual days with data
    coverage_percentage REAL,          -- Actual/Expected ratio
    missing_days_count INTEGER,        -- Days without data

    -- Update tracking
    last_update_attempt DATETIME,      -- Last time we tried to update
    last_successful_update DATETIME,   -- Last successful data fetch
    consecutive_failures INTEGER DEFAULT 0, -- Track API failures

    -- Status flags
    needs_backfill BOOLEAN DEFAULT 0,  -- Has historical gaps
    needs_current_update BOOLEAN DEFAULT 1, -- Needs recent data
    is_active BOOLEAN DEFAULT 1,       -- Still trading

    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE (stock_id),
    INDEX idx_coverage_symbol (symbol),
    INDEX idx_coverage_needs_update (needs_current_update),
    INDEX idx_coverage_needs_backfill (needs_backfill)
);
```

## 🔄 Incremental Update Logic

### 1. Date Range Calculator
```rust
#[derive(Debug, Clone)]
pub struct DateRangeCalculator {
    db_pool: SqlitePool,
}

#[derive(Debug, Clone)]
pub struct UpdateDateRange {
    pub symbol: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub update_type: UpdateType,
    pub expected_days: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateType {
    FullHistorical,     // New symbol, get all data since IPO
    IncrementalUpdate,  // Get data since last update
    GapFill,           // Fill missing historical data
    CurrentUpdate,     // Get only recent data (last few days)
}

impl DateRangeCalculator {
    pub async fn calculate_update_range(&self, symbol: &str) -> Result<UpdateDateRange> {
        let metadata = self.get_company_metadata(symbol).await?;
        let coverage = self.get_coverage_info(symbol).await?;
        let current_date = Utc::now().naive_utc().date();

        let (start_date, end_date, update_type) = match coverage {
            // New symbol - get all historical data
            None => {
                let start = metadata.listing_date.unwrap_or_else(|| {
                    NaiveDate::from_ymd_opt(2015, 1, 1).unwrap()
                });
                (start, current_date, UpdateType::FullHistorical)
            }

            // Existing symbol - incremental update
            Some(cov) if cov.actual_end_date < current_date => {
                let start = cov.actual_end_date + chrono::Duration::days(1);
                (start, current_date, UpdateType::IncrementalUpdate)
            }

            // Up to date
            Some(cov) => {
                return Ok(UpdateDateRange {
                    symbol: symbol.to_string(),
                    start_date: current_date,
                    end_date: current_date,
                    update_type: UpdateType::CurrentUpdate,
                    expected_days: 0,
                });
            }
        };

        let expected_days = self.calculate_trading_days(start_date, end_date);

        Ok(UpdateDateRange {
            symbol: symbol.to_string(),
            start_date,
            end_date,
            update_type,
            expected_days,
        })
    }

    async fn get_company_metadata(&self, symbol: &str) -> Result<CompanyMetadata> {
        // Get from company_metadata table
    }

    async fn get_coverage_info(&self, symbol: &str) -> Result<Option<CoverageInfo>> {
        // Get from price_data_coverage table
    }

    fn calculate_trading_days(&self, start: NaiveDate, end: NaiveDate) -> usize {
        // Calculate expected trading days (weekdays minus holidays)
        let total_days = (end - start).num_days() as usize;
        let weekdays = total_days / 7 * 5 + (total_days % 7).min(5);
        // Approximate: ~252 trading days per year, accounting for holidays
        (weekdays as f64 * 0.96) as usize
    }
}
```

### 2. Gap Detection Engine
```rust
#[derive(Debug)]
pub struct GapDetector {
    db_pool: SqlitePool,
}

#[derive(Debug, Clone)]
pub struct DataGap {
    pub symbol: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub missing_days: usize,
    pub gap_type: GapType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GapType {
    Historical,    // Gap in historical data
    Recent,        // Missing recent data
    Intraday,      // Missing days within a range
}

impl GapDetector {
    pub async fn detect_gaps(&self, symbol: &str) -> Result<Vec<DataGap>> {
        let stock_id = self.get_stock_id(symbol).await?;

        // Get all trading days with data
        let existing_dates: HashSet<NaiveDate> = sqlx::query_scalar(
            "SELECT date FROM daily_prices WHERE stock_id = ? ORDER BY date"
        )
        .bind(stock_id)
        .fetch_all(&self.db_pool)
        .await?
        .into_iter()
        .collect();

        if existing_dates.is_empty() {
            return Ok(vec![]); // No data yet
        }

        let min_date = *existing_dates.iter().min().unwrap();
        let max_date = *existing_dates.iter().max().unwrap();

        let mut gaps = Vec::new();
        let mut gap_start: Option<NaiveDate> = None;

        // Check for gaps in the range
        for date in self.generate_trading_days(min_date, max_date) {
            if !existing_dates.contains(&date) {
                // Start of a gap
                if gap_start.is_none() {
                    gap_start = Some(date);
                }
            } else {
                // End of a gap
                if let Some(start) = gap_start {
                    gaps.push(DataGap {
                        symbol: symbol.to_string(),
                        start_date: start,
                        end_date: date - chrono::Duration::days(1),
                        missing_days: (date - start).num_days() as usize,
                        gap_type: GapType::Intraday,
                    });
                    gap_start = None;
                }
            }
        }

        Ok(gaps)
    }

    fn generate_trading_days(&self, start: NaiveDate, end: NaiveDate) -> Vec<NaiveDate> {
        // Generate expected trading days (weekdays, excluding known holidays)
        let mut dates = Vec::new();
        let mut current = start;

        while current <= end {
            // Skip weekends
            if current.weekday().num_days_from_monday() < 5 {
                // TODO: Add holiday calendar check
                dates.push(current);
            }
            current += chrono::Duration::days(1);
        }

        dates
    }
}
```

### 3. Update Coordinator
```rust
#[derive(Debug)]
pub struct IncrementalUpdateCoordinator {
    schwab_client: SchwabClient,
    db_pool: SqlitePool,
    date_calculator: DateRangeCalculator,
    gap_detector: GapDetector,
}

impl IncrementalUpdateCoordinator {
    pub async fn run_incremental_update(&mut self) -> Result<UpdateSummary> {
        info!("🔄 Starting incremental Schwab price data update");

        // 1. Get all symbols that need updates
        let symbols_to_update = self.get_symbols_needing_update().await?;
        info!("Found {} symbols needing updates", symbols_to_update.len());

        // 2. Calculate optimal date ranges for each symbol
        let mut update_plans = Vec::new();
        for symbol in &symbols_to_update {
            let range = self.date_calculator.calculate_update_range(symbol).await?;
            if range.expected_days > 0 {
                update_plans.push(range);
            }
        }

        // 3. Prioritize updates (recent data first, then gaps)
        update_plans.sort_by_key(|plan| match plan.update_type {
            UpdateType::IncrementalUpdate => 1, // Highest priority
            UpdateType::CurrentUpdate => 2,
            UpdateType::GapFill => 3,
            UpdateType::FullHistorical => 4,    // Lowest priority
        });

        // 4. Execute updates with progress tracking
        let mut summary = UpdateSummary::new();
        for (index, plan) in update_plans.iter().enumerate() {
            info!("📈 {}/{}: Updating {} ({:?}, {} days)",
                  index + 1, update_plans.len(), plan.symbol, plan.update_type, plan.expected_days);

            match self.execute_update_plan(plan).await {
                Ok(result) => {
                    summary.successful_updates += 1;
                    summary.total_records_added += result.records_added;
                    self.update_coverage_tracking(&plan.symbol, &result).await?;
                }
                Err(e) => {
                    warn!("❌ Failed to update {}: {}", plan.symbol, e);
                    summary.failed_updates += 1;
                    self.record_failure(&plan.symbol, &e).await?;
                }
            }

            // Rate limiting
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        // 5. Update metadata and coverage tables
        self.refresh_coverage_analysis().await?;

        info!("✅ Incremental update complete: {} success, {} failed, {} records added",
              summary.successful_updates, summary.failed_updates, summary.total_records_added);

        Ok(summary)
    }

    async fn execute_update_plan(&self, plan: &UpdateDateRange) -> Result<UpdateResult> {
        // Fetch data from Schwab API for specific date range
        let price_bars = self.schwab_client
            .get_price_history(&plan.symbol, plan.start_date, plan.end_date)
            .await?;

        if price_bars.is_empty() && plan.expected_days > 0 {
            return Err(anyhow!("No data returned for {} in range {:?} to {:?}",
                              plan.symbol, plan.start_date, plan.end_date));
        }

        // Insert into database (INSERT OR REPLACE handles duplicates)
        let stock_id = self.get_stock_id(&plan.symbol).await?;
        let records_added = self.insert_price_data(stock_id, &price_bars).await?;

        Ok(UpdateResult {
            symbol: plan.symbol.clone(),
            records_added,
            date_range: (plan.start_date, plan.end_date),
            update_type: plan.update_type.clone(),
        })
    }
}
```

## 📅 IPO/Listing Date Database

### Known IPO/Listing Dates (from analysis)
```sql
-- Populate company_metadata with known dates
INSERT INTO company_metadata (stock_id, symbol, company_name, ipo_date, listing_date, spinoff_date) VALUES
-- Recent IPOs/Listings
((SELECT id FROM stocks WHERE symbol = 'COIN'), 'COIN', 'Coinbase Global Inc', '2021-04-14', '2021-04-14', NULL),
((SELECT id FROM stocks WHERE symbol = 'KVUE'), 'KVUE', 'Kenvue Inc', '2023-05-04', '2023-05-04', '2023-05-04'), -- J&J spinoff
((SELECT id FROM stocks WHERE symbol = 'CEG'), 'CEG', 'Constellation Energy Corp', '2022-02-02', '2022-02-02', '2022-02-02'), -- Exelon spinoff
((SELECT id FROM stocks WHERE symbol = 'PLTR'), 'PLTR', 'Palantir Technologies Inc', '2020-09-30', '2020-09-30', NULL),

-- Carrier spinoffs
((SELECT id FROM stocks WHERE symbol = 'CARR'), 'CARR', 'Carrier Global Corp', '2020-04-03', '2020-04-03', '2020-04-03'), -- UTC spinoff
((SELECT id FROM stocks WHERE symbol = 'OTIS'), 'OTIS', 'Otis Worldwide Corp', '2020-04-03', '2020-04-03', '2020-04-03'), -- UTC spinoff

-- Other recent additions
((SELECT id FROM stocks WHERE symbol = 'UBER'), 'UBER', 'Uber Technologies Inc', '2019-05-10', '2019-05-10', NULL),
((SELECT id FROM stocks WHERE symbol = 'DELL'), 'DELL', 'Dell Technologies Inc', '2018-12-28', '2018-12-28', NULL), -- Re-listing

-- Recent spinoffs
((SELECT id FROM stocks WHERE symbol = 'GEV'), 'GEV', 'GE Vernova Inc', '2024-04-02', '2024-04-02', '2024-04-02'), -- GE spinoff
((SELECT id FROM stocks WHERE symbol = 'GEHC'), 'GEHC', 'GE HealthCare Technologies Inc', '2023-01-04', '2023-01-04', '2023-01-04'), -- GE spinoff
((SELECT id FROM stocks WHERE symbol = 'SOLV'), 'SOLV', 'Solventum Corp', '2024-04-01', '2024-04-01', '2024-04-01'), -- 3M spinoff
((SELECT id FROM stocks WHERE symbol = 'VLTO'), 'VLTO', 'Veralto Corp', '2023-10-02', '2023-10-02', '2023-10-02'), -- Danaher spinoff
((SELECT id FROM stocks WHERE symbol = 'TKO'), 'TKO', 'TKO Group Holdings Inc', '2023-09-12', '2023-09-12', NULL);

-- For established companies, use conservative estimates
UPDATE company_metadata SET
    listing_date = '2015-01-01',  -- Use our data start date for established companies
    ipo_date = '2015-01-01'       -- Conservative estimate
WHERE listing_date IS NULL;
```

## 🔧 Implementation Plan

### Phase 1: Database Schema (1 hour)
1. Create migration for `company_metadata` table
2. Create migration for `price_data_coverage` table
3. Populate known IPO/listing dates
4. Create indexes for performance

### Phase 2: Core Logic (2 hours)
1. Implement `DateRangeCalculator`
2. Implement `GapDetector`
3. Create coverage analysis functions
4. Add incremental update logic

### Phase 3: Integration (1 hour)
1. Update `import-schwab-prices.rs` to use incremental logic
2. Add CLI flags for update modes
3. Implement progress tracking improvements
4. Add validation and error handling

### Phase 4: Testing & Validation (1 hour)
1. Test incremental updates with sample symbols
2. Validate gap detection accuracy
3. Test error recovery and resumption
4. Performance testing with full S&P 500

## 🎯 Expected Benefits

### Performance Improvements
- **90% API Call Reduction**: Only fetch missing data
- **5x Faster Updates**: Skip existing data
- **Smart Scheduling**: Prioritize recent data over historical gaps

### Data Quality Improvements
- **Complete Coverage**: Fill historical gaps systematically
- **Real-time Updates**: Daily/weekly incremental updates
- **Metadata Tracking**: Know exactly what data we have/need

### Operational Benefits
- **Automated Updates**: Set-and-forget daily updates
- **Error Recovery**: Resume interrupted updates seamlessly
- **Cost Optimization**: Minimize API usage and rate limiting

This architecture provides a robust foundation for maintaining complete, up-to-date S&P 500 price data with minimal manual intervention.