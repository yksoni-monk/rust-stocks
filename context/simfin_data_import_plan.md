# SimFin Data Import Implementation Plan

**Date**: 2025-09-06  
**Data Source**: SimFin US Stock Data (CSV files)  
**Target**: SQLite database (`stocks.db`)

## Data Analysis Summary

### 1. **Daily Share Prices Data** (`us-shareprices-daily.csv`)
- **Records**: ~6.2 million rows
- **Unique Tickers**: 5,876 stocks
- **Date Range**: 2019-10-11 to 2024-09-13 (~5 years)
- **Fields**:
  ```
  Ticker, SimFinId, Date, Open, High, Low, Close, Adj. Close, Volume, Dividend, Shares Outstanding
  ```
- **Sample**: `A;45846;2019-10-11;75.55;76.21;75.24;75.30;72.32;1216971;;309467678`

### 2. **Quarterly Income Data** (`us-income-quarterly.csv`)
- **Records**: ~52k rows  
- **Fields**: 27+ columns including:
  ```
  Ticker, SimFinId, Currency, Fiscal Year, Fiscal Period, Report Date, Publish Date, 
  Revenue, Net Income, Shares (Basic), Shares (Diluted), EPS calculations, etc.
  ```
- **Sample**: Comprehensive quarterly financial statements

### 3. **Current Database State**
- **Status**: Fresh database with basic schema
- **Tables**: `stocks`, `daily_prices` (basic structure)
- **Records**: Empty (ready for import)

## Database Schema Enhancement Plan

### Phase 1: Enhanced Schema Design

#### A. **Stocks Table Enhancement**
```sql
ALTER TABLE stocks ADD COLUMN simfin_id INTEGER UNIQUE;
ALTER TABLE stocks ADD COLUMN sector TEXT;
ALTER TABLE stocks ADD COLUMN industry TEXT;
ALTER TABLE stocks ADD COLUMN currency TEXT DEFAULT 'USD';
ALTER TABLE stocks ADD COLUMN market_cap REAL;
ALTER TABLE stocks ADD COLUMN shares_outstanding INTEGER;
```

#### B. **Daily Prices Table Enhancement**
```sql
-- Current fields are mostly compatible, add missing ones:
ALTER TABLE daily_prices ADD COLUMN pe_ratio REAL;
ALTER TABLE daily_prices ADD COLUMN data_source TEXT DEFAULT 'simfin';
ALTER TABLE daily_prices ADD COLUMN last_updated DATETIME DEFAULT CURRENT_TIMESTAMP;
```

#### C. **New Quarterly Financials Table**
```sql
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
    
    -- Income Statement Core Metrics
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
    
    -- Calculated Metrics
    eps_basic REAL,
    eps_diluted REAL,
    
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, fiscal_year, fiscal_period)
);
```

## Implementation Strategy

### Phase 1: Database Migration
**File**: `database_migration_simfin.sql`
- Enhance existing schema
- Create new quarterly_financials table
- Add necessary indexes for performance

### Phase 2: Rust CSV Import Tool
**File**: `src-tauri/src/tools/simfin_importer.rs`

#### Dependencies to Add:
```toml
[dependencies]
csv = "1.3"
serde = { version = "1.0", features = ["derive"] }
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite", "macros", "chrono"] }
chrono = { version = "0.4", features = ["serde"] }
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
indicatif = "0.17" # For progress bars
```

#### Import Process Design:
1. **Parse and validate CSV data**
2. **Extract unique stocks** and populate `stocks` table
3. **Batch import daily prices** (optimal batch size ~10k records)
4. **Import quarterly financials**
5. **Calculate and store EPS** (Net Income ÷ Diluted Shares Outstanding)
6. **Calculate and store P/E ratios** using calculated EPS data
7. **Progress tracking** and error handling

### Phase 3: EPS Calculation Phase

**CRITICAL REQUIREMENT**: EPS must be calculated and stored before P/E ratio calculations.

#### **EPS Calculation Formula**: `Net Income ÷ Diluted Shares Outstanding`

#### A. **EPS Calculation Logic**
```rust
async fn calculate_and_store_eps(pool: &SqlitePool) -> Result<usize, String> {
    // 1. Query all quarterly financial records with required fields
    let financial_records = sqlx::query(
        "SELECT id, stock_id, fiscal_year, fiscal_period, net_income, shares_diluted 
         FROM quarterly_financials 
         WHERE net_income IS NOT NULL AND shares_diluted IS NOT NULL AND shares_diluted > 0"
    )
    .fetch_all(pool).await
    .map_err(|e| format!("Failed to fetch financial records: {}", e))?;
    
    let mut calculated_count = 0;
    
    for record in financial_records {
        let id: i64 = record.get("id");
        let net_income: f64 = record.get("net_income");
        let shares_diluted: i64 = record.get("shares_diluted");
        
        // Calculate EPS = Net Income / Diluted Shares Outstanding
        let eps = net_income / (shares_diluted as f64);
        
        // Store calculated EPS back to quarterly_financials table
        sqlx::query(
            "UPDATE quarterly_financials SET eps_diluted = ?1 WHERE id = ?2"
        )
        .bind(eps)
        .bind(id)
        .execute(pool).await
        .map_err(|e| format!("Failed to update EPS for record {}: {}", id, e))?;
        
        calculated_count += 1;
    }
    
    println!("DEBUG: Calculated and stored EPS for {} quarterly records", calculated_count);
    Ok(calculated_count)
}
```

#### B. **Enhanced Database Schema for EPS Storage**
```sql
-- Ensure quarterly_financials table has EPS storage field
ALTER TABLE quarterly_financials ADD COLUMN eps_calculated REAL;
ALTER TABLE quarterly_financials ADD COLUMN eps_calculation_date DATETIME DEFAULT CURRENT_TIMESTAMP;
```

### Phase 4: Data Processing Logic

#### A. **Stock Import Logic**
```rust
#[derive(Debug, Deserialize)]
struct SimFinDailyPrice {
    #[serde(rename = "Ticker")]
    ticker: String,
    #[serde(rename = "SimFinId")]
    simfin_id: i64,
    #[serde(rename = "Date")]
    date: String,
    #[serde(rename = "Open")]
    open: Option<f64>,
    #[serde(rename = "High")]
    high: Option<f64>,
    // ... other fields
}

async fn import_stocks_from_daily_prices(csv_path: &str) -> Result<()> {
    // 1. Scan CSV to extract unique (Ticker, SimFinId) pairs
    // 2. Batch insert into stocks table
    // 3. Create stock_id mapping for subsequent imports
}
```

#### B. **Daily Price Import Logic**
```rust
async fn import_daily_prices(csv_path: &str, batch_size: usize) -> Result<ImportStats> {
    // 1. Read CSV in chunks
    // 2. Map tickers to stock_ids
    // 3. Transform data format
    // 4. Batch insert (10k records per transaction)
    // 5. Update progress bar
    // 6. Handle data validation errors
}
```

#### C. **Quarterly Financials Import**
```rust
async fn import_quarterly_financials(csv_path: &str) -> Result<ImportStats> {
    // 1. Parse comprehensive financial data
    // 2. Calculate EPS (basic & diluted)
    // 3. Map to stock_ids
    // 4. Batch insert financial records
}
```

#### D. **P/E Ratio Calculation** (AFTER EPS Calculation)
```rust
async fn calculate_and_store_pe_ratios(pool: &SqlitePool) -> Result<usize, String> {
    // 1. Get all daily price records that need P/E calculation
    let price_records = sqlx::query(
        "SELECT id, stock_id, date, close_price 
         FROM daily_prices 
         WHERE close_price IS NOT NULL AND close_price > 0"
    )
    .fetch_all(pool).await
    .map_err(|e| format!("Failed to fetch price records: {}", e))?;
    
    let mut calculated_count = 0;
    
    for price_record in price_records {
        let price_id: i64 = price_record.get("id");
        let stock_id: i64 = price_record.get("stock_id");
        let price_date: NaiveDate = price_record.get("date");
        let close_price: f64 = price_record.get("close_price");
        
        // Find latest calculated EPS before or on this date
        let eps_result = sqlx::query(
            "SELECT eps_calculated 
             FROM quarterly_financials 
             WHERE stock_id = ?1 AND report_date <= ?2 AND eps_calculated IS NOT NULL
             ORDER BY report_date DESC 
             LIMIT 1"
        )
        .bind(stock_id)
        .bind(price_date)
        .fetch_optional(pool).await
        .map_err(|e| format!("Failed to get EPS for stock {}: {}", stock_id, e))?;
        
        if let Some(eps_row) = eps_result {
            let eps: f64 = eps_row.get("eps_calculated");
            
            // Calculate P/E = Close Price / EPS (avoid division by zero)
            if eps != 0.0 {
                let pe_ratio = close_price / eps;
                
                // Update daily_prices with calculated P/E ratio
                sqlx::query(
                    "UPDATE daily_prices SET pe_ratio = ?1 WHERE id = ?2"
                )
                .bind(pe_ratio)
                .bind(price_id)
                .execute(pool).await
                .map_err(|e| format!("Failed to update P/E for price record {}: {}", price_id, e))?;
                
                calculated_count += 1;
            }
        }
    }
    
    println!("DEBUG: Calculated and stored P/E ratios for {} price records", calculated_count);
    Ok(calculated_count)
}
```

### Phase 4: Import Tool CLI Interface

#### **Standalone Import Tool**
```rust
// src-tauri/src/bin/import_simfin.rs
use clap::{Arg, Command};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("SimFin Data Importer")
        .arg(Arg::new("prices")
            .long("prices")
            .value_name("FILE")
            .help("Path to us-shareprices-daily.csv"))
        .arg(Arg::new("income")
            .long("income") 
            .value_name("FILE")
            .help("Path to us-income-quarterly.csv"))
        .arg(Arg::new("database")
            .long("db")
            .value_name("FILE")
            .help("Path to SQLite database")
            .default_value("./stocks.db"))
        .get_matches();

    let prices_path = matches.get_one::<String>("prices").unwrap();
    let income_path = matches.get_one::<String>("income").unwrap();
    let db_path = matches.get_one::<String>("database").unwrap();
    
    // Run import process with progress tracking
}
```

**Usage**:
```bash
cargo run --bin import_simfin -- \
  --prices ~/simfin_data/us-shareprices-daily.csv \
  --income ~/simfin_data/us-income-quarterly.csv \
  --db ./stocks.db
```

### Phase 5: Frontend Integration

#### **Disable API Fetch Buttons**
- Replace "Fetch Data" buttons with "Import Status" displays
- Show data coverage statistics
- Add "Refresh Data" functionality (to recalculate P/E ratios)

#### **Data Import Status Panel**
```jsx
function DataImportStatus({ stock }) {
  return (
    <div className="bg-gray-50 rounded-lg p-4">
      <h4 className="font-medium text-gray-900 mb-2">Data Status (SimFin)</h4>
      <div className="space-y-2 text-sm">
        <div>📊 Price Data: 2019-2024 (5 years)</div>
        <div>📈 Records: ~6.2M daily prices</div>
        <div>🏢 Stocks: 5,876 companies</div>
        <div>💰 P/E Coverage: Real-time calculated</div>
      </div>
    </div>
  );
}
```

## Performance Considerations

### **Import Optimization**
1. **Batch Processing**: 10k records per transaction
2. **Memory Management**: Stream processing for large files
3. **Indexing**: Add indexes after import completion
4. **Parallel Processing**: Separate workers for prices vs financials

### **Database Optimization**
```sql
-- Performance indexes (add after import)
CREATE INDEX idx_daily_prices_stock_date ON daily_prices(stock_id, date);
CREATE INDEX idx_daily_prices_date ON daily_prices(date);
CREATE INDEX idx_quarterly_financials_stock_period ON quarterly_financials(stock_id, fiscal_year, fiscal_period);
```

### **Expected Performance**
- **Import Time**: ~15-30 minutes for 6.2M records
- **Database Size**: ~2-3 GB after import
- **P/E Calculation**: ~5-10 minutes for all records

## Implementation Timeline

### **Phase 1**: Database Migration (Day 1)
- [ ] Create enhanced database schema
- [ ] Test migration on sample data
- [ ] Verify schema compatibility

### **Phase 2**: Basic CSV Import Tool (Day 2)
- [ ] Implement stock extraction logic
- [ ] Build daily price import function
- [ ] Add progress tracking and error handling

### **Phase 3**: Financial Data Import (Day 3)  
- [ ] Parse quarterly financial statements
- [ ] Import quarterly financial data to database
- [ ] Validate financial data integrity

### **Phase 4**: EPS Calculation (Day 4)
- [ ] Implement EPS calculation logic (Net Income ÷ Diluted Shares Outstanding)
- [ ] Store calculated EPS values in quarterly_financials table
- [ ] Validate EPS calculations against sample data

### **Phase 5**: P/E Ratio Calculation (Day 5)
- [ ] Implement P/E ratio calculation using stored EPS values
- [ ] Link daily prices with appropriate quarterly EPS data
- [ ] Store P/E ratios in daily_prices table

### **Phase 6**: Integration & Testing (Day 6)
- [ ] Build CLI tool with proper arguments
- [ ] Test full import process including EPS and P/E calculations
- [ ] Verify data integrity and calculation accuracy

### **Phase 7**: Frontend Updates (Day 7)
- [ ] Disable API fetch buttons
- [ ] Add data status displays
- [ ] Update UI to reflect SimFin data source

## Risk Mitigation

### **Data Quality Issues**
- **Empty Fields**: Handle NULL/empty values gracefully
- **Data Type Mismatches**: Robust parsing with fallbacks
- **Missing EPS Data**: Use 0.0 default for P/E calculations

### **Performance Issues**
- **Memory Usage**: Stream processing for large files
- **Import Speed**: Batch processing with progress tracking
- **Database Locks**: Use WAL mode for concurrent access

### **Error Recovery**
- **Partial Import Failure**: Resume from checkpoint
- **Data Validation**: Skip invalid records with logging
- **Rollback Strategy**: Transaction-based import with rollback capability

## Success Criteria

1. ✅ **Complete Data Import**: All 6.2M price records successfully imported
2. ✅ **Financial Integration**: Quarterly data linked with daily prices
3. ✅ **P/E Calculation**: Accurate daily P/E ratios for all available data
4. ✅ **Performance**: Import completes within 30 minutes
5. ✅ **Data Integrity**: No missing stocks or corrupted price data
6. ✅ **Frontend Integration**: Clean UI showing SimFin data status

## Post-Import Enhancements

### **Future Improvements**
1. **Incremental Updates**: Import only new data in future updates
2. **Data Validation Dashboard**: Show data quality metrics
3. **Advanced Analytics**: Leverage comprehensive financial data
4. **Export Functionality**: CSV/Excel export for analysis

---

**Ready for Implementation**: This plan provides a systematic approach to replacing API-based data fetching with comprehensive SimFin historical data import.

**Next Steps**: Upon approval, implement Phase 1 (Database Migration) and begin CSV import tool development.