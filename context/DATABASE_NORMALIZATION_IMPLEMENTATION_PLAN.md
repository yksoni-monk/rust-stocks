# Database Normalization Implementation Plan

## Executive Summary

This document provides a detailed, phase-by-phase implementation plan for database normalization with proper migrations, code changes, and testing. Each phase is designed to be **incremental**, **testable**, and **reversible**.

## Implementation Phases

### **PHASE 0: Obsolete Columns Cleanup** (SAFEST - Start Here)

**Risk Level**: 🟢 **ZERO RISK**  
**Impact**: 🟢 **IMMEDIATE BENEFITS**  
**Dependencies**: 🟢 **NONE**  
**Timeline**: **1-2 days**

#### **Phase 0.1: Create Migration File**
```bash
# Create migration file
cd /Users/yksoni/code/misc/rust-stocks/src-tauri
sqlx migrate add remove_obsolete_stocks_columns
```

#### **Phase 0.2: Migration Content**
**File**: `src-tauri/db/migrations/YYYYMMDDHHMMSS_remove_obsolete_stocks_columns.sql`

```sql
-- Migration: Remove obsolete columns from stocks table
-- Risk: ZERO (columns are unused)
-- Impact: Immediate storage reduction and cleaner schema

-- Step 1: Create backup table (safety measure)
CREATE TABLE stocks_backup AS SELECT * FROM stocks;

-- Step 2: Create new stocks table without obsolete columns
CREATE TABLE stocks_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT UNIQUE NOT NULL,
    company_name TEXT NOT NULL,
    cik TEXT UNIQUE,
    
    -- Classification (keep only essential)
    sector TEXT,  -- Keep for S&P 500 screening algorithms
    
    -- Audit fields (keep essential)
    last_updated DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    -- Flags
    is_sp500 BOOLEAN DEFAULT 0
);

-- Step 3: Copy data from old table (excluding obsolete columns)
INSERT INTO stocks_new 
SELECT 
    id, symbol, company_name, cik, sector, 
    last_updated, created_at, is_sp500
FROM stocks;

-- Step 4: Create indexes
CREATE INDEX idx_stocks_symbol ON stocks_new(symbol);
CREATE INDEX idx_stocks_cik ON stocks_new(cik);
CREATE INDEX idx_stocks_sp500 ON stocks_new(is_sp500);
CREATE INDEX idx_stocks_sector ON stocks_new(sector);

-- Step 5: Drop old table and rename new one
DROP TABLE stocks;
ALTER TABLE stocks_new RENAME TO stocks;

-- Step 6: Verify data integrity
-- This will be done in testing phase
```

#### **Phase 0.3: Code Updates Required**
**Files to Update**:
- `src-tauri/src/models/mod.rs` - Remove obsolete fields from `Stock` struct
- `src-tauri/src/database_sqlx.rs` - Update queries to remove obsolete columns
- `src-tauri/src/commands/initialization.rs` - Update stock creation logic

**Specific Changes**:

1. **Update `Stock` struct** (`src-tauri/src/models/mod.rs`):
```rust
// BEFORE
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stock {
    pub id: i64,
    pub symbol: String,
    pub company_name: String,
    pub sector: Option<String>,
    pub industry: Option<String>,
    pub market_cap: Option<f64>,
    pub status: StockStatus,
    pub first_trading_date: Option<NaiveDate>,
    pub last_updated: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub currency: Option<String>,
    pub shares_outstanding: Option<i64>,
    pub simfin_id: Option<i64>,
    pub is_sp500: bool,
    pub gics_sector: Option<String>,
    pub gics_industry_group: Option<String>,
    pub gics_industry: Option<String>,
    pub gics_sub_industry: Option<String>,
    pub cik: Option<String>,
}

// AFTER
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stock {
    pub id: i64,
    pub symbol: String,
    pub company_name: String,
    pub sector: Option<String>,
    pub cik: Option<String>,
    pub last_updated: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub is_sp500: bool,
}
```

2. **Update Database Queries** (`src-tauri/src/database_sqlx.rs`):
```rust
// BEFORE
let query = r#"
    SELECT id, symbol, company_name, sector, industry, market_cap, status, first_trading_date, last_updated, created_at
    FROM stocks
    WHERE status = 'active'
"#;

// AFTER
let query = r#"
    SELECT id, symbol, company_name, sector, cik, last_updated, created_at, is_sp500
    FROM stocks
"#;
```

3. **Remove `StockStatus` enum** (no longer needed):
```rust
// REMOVE this entire enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StockStatus {
    Active,
    Delisted,
    Suspended,
}
```

#### **Phase 0.4: Testing Procedures**
```bash
# 1. Run migration
cd /Users/yksoni/code/misc/rust-stocks/src-tauri
sqlx migrate run --database-url "sqlite:db/stocks.db" --source db/migrations

# 2. Verify data integrity
sqlite3 db/stocks.db "
SELECT 
    COUNT(*) as total_stocks,
    COUNT(CASE WHEN sector IS NOT NULL THEN 1 END) as with_sector,
    COUNT(CASE WHEN cik IS NOT NULL THEN 1 END) as with_cik,
    COUNT(CASE WHEN is_sp500 = 1 THEN 1 END) as sp500_count
FROM stocks;
"

# 3. Test application compilation
cargo check

# 4. Test basic functionality
cargo run --bin refresh_data -- --status

# 5. Verify no data loss
sqlite3 db/stocks.db "
SELECT COUNT(*) FROM stocks_backup;
SELECT COUNT(*) FROM stocks;
-- Should be equal
"
```

#### **Phase 0.5: Rollback Procedure**
```sql
-- Rollback migration (if needed)
DROP TABLE stocks;
ALTER TABLE stocks_backup RENAME TO stocks;
```

#### **Phase 0.6: Success Criteria**
- ✅ Migration runs without errors
- ✅ All 5,892 stocks preserved
- ✅ Application compiles without warnings
- ✅ Freshness checker works
- ✅ No data loss
- ✅ Database size reduced by ~30%

---

### **PHASE 1: EDGAR Metadata Consolidation**

**Risk Level**: 🟡 **LOW RISK**  
**Impact**: 🟢 **HIGH IMPACT**  
**Dependencies**: ✅ **Phase 0 Complete**  
**Timeline**: **3-5 days**

#### **Phase 1.1: Create `sec_filings` Table**
```bash
sqlx migrate add create_sec_filings_table
```

**File**: `src-tauri/db/migrations/YYYYMMDDHHMMSS_create_sec_filings_table.sql`

```sql
-- Migration: Create sec_filings table
-- Risk: LOW (new table, no data loss)
-- Impact: Foundation for EDGAR metadata consolidation

CREATE TABLE sec_filings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    accession_number TEXT NOT NULL,
    form_type TEXT NOT NULL,
    filed_date DATE NOT NULL,
    fiscal_period TEXT,
    fiscal_year INTEGER NOT NULL,
    report_date DATE NOT NULL,
    
    -- Filing metadata
    file_size_bytes INTEGER,
    document_count INTEGER,
    is_amended BOOLEAN DEFAULT 0,
    
    -- Audit fields
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, accession_number),
    UNIQUE(stock_id, form_type, report_date, fiscal_year)
);

-- Performance indexes
CREATE INDEX idx_sec_filings_stock_id ON sec_filings(stock_id);
CREATE INDEX idx_sec_filings_filed_date ON sec_filings(filed_date);
CREATE INDEX idx_sec_filings_form_type ON sec_filings(form_type);
CREATE INDEX idx_sec_filings_accession ON sec_filings(accession_number);
CREATE INDEX idx_sec_filings_report_date ON sec_filings(report_date);
```

#### **Phase 1.2: Populate `sec_filings` Table**
```bash
sqlx migrate add populate_sec_filings
```

**File**: `src-tauri/db/migrations/YYYYMMDDHHMMSS_populate_sec_filings.sql`

```sql
-- Migration: Populate sec_filings from existing data
-- Risk: LOW (read-only operation)
-- Impact: Creates normalized filing data

-- Step 1: Insert from income_statements
INSERT INTO sec_filings (stock_id, accession_number, form_type, filed_date, fiscal_period, fiscal_year, report_date)
SELECT DISTINCT 
    stock_id,
    accession_number,
    form_type,
    filed_date,
    fiscal_period,
    fiscal_year,
    report_date
FROM income_statements 
WHERE accession_number IS NOT NULL 
    AND form_type IS NOT NULL 
    AND filed_date IS NOT NULL;

-- Step 2: Insert from balance_sheets (ignore duplicates)
INSERT OR IGNORE INTO sec_filings (stock_id, accession_number, form_type, filed_date, fiscal_period, fiscal_year, report_date)
SELECT DISTINCT 
    stock_id,
    accession_number,
    form_type,
    filed_date,
    fiscal_period,
    fiscal_year,
    report_date
FROM balance_sheets 
WHERE accession_number IS NOT NULL 
    AND form_type IS NOT NULL 
    AND filed_date IS NOT NULL;

-- Step 3: Insert from cash_flow_statements (ignore duplicates)
INSERT OR IGNORE INTO sec_filings (stock_id, accession_number, form_type, filed_date, fiscal_period, fiscal_year, report_date)
SELECT DISTINCT 
    stock_id,
    accession_number,
    form_type,
    filed_date,
    fiscal_period,
    fiscal_year,
    report_date
FROM cash_flow_statements 
WHERE accession_number IS NOT NULL 
    AND form_type IS NOT NULL 
    AND filed_date IS NOT NULL;

-- Step 4: Verify data integrity
-- This will be done in testing phase
```

#### **Phase 1.3: Add `sec_filing_id` References**
```bash
sqlx migrate add add_sec_filing_references
```

**File**: `src-tauri/db/migrations/YYYYMMDDHHMMSS_add_sec_filing_references.sql`

```sql
-- Migration: Add sec_filing_id to financial tables
-- Risk: LOW (adding nullable columns)
-- Impact: Enables linking financial statements to filings

-- Step 1: Add sec_filing_id columns
ALTER TABLE income_statements ADD COLUMN sec_filing_id INTEGER;
ALTER TABLE balance_sheets ADD COLUMN sec_filing_id INTEGER;
ALTER TABLE cash_flow_statements ADD COLUMN sec_filing_id INTEGER;

-- Step 2: Update financial records with sec_filing_id
UPDATE income_statements 
SET sec_filing_id = (
    SELECT sf.id 
    FROM sec_filings sf 
    WHERE sf.stock_id = income_statements.stock_id 
        AND sf.accession_number = income_statements.accession_number
        AND sf.form_type = income_statements.form_type
        AND sf.filed_date = income_statements.filed_date
)
WHERE accession_number IS NOT NULL;

UPDATE balance_sheets 
SET sec_filing_id = (
    SELECT sf.id 
    FROM sec_filings sf 
    WHERE sf.stock_id = balance_sheets.stock_id 
        AND sf.accession_number = balance_sheets.accession_number
        AND sf.form_type = balance_sheets.form_type
        AND sf.filed_date = balance_sheets.filed_date
)
WHERE accession_number IS NOT NULL;

UPDATE cash_flow_statements 
SET sec_filing_id = (
    SELECT sf.id 
    FROM sec_filings sf 
    WHERE sf.stock_id = cash_flow_statements.stock_id 
        AND sf.accession_number = cash_flow_statements.accession_number
        AND sf.form_type = cash_flow_statements.form_type
        AND sf.filed_date = cash_flow_statements.filed_date
)
WHERE accession_number IS NOT NULL;
```

#### **Phase 1.4: Code Updates**
**Files to Update**:
- `src-tauri/src/tools/data_freshness_checker.rs` - Update queries to use `sec_filings`
- `src-tauri/src/tools/sec_edgar_client.rs` - Update data insertion logic
- `src-tauri/src/models/mod.rs` - Add `SecFiling` struct

**Specific Changes**:

1. **Add `SecFiling` struct** (`src-tauri/src/models/mod.rs`):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecFiling {
    pub id: i64,
    pub stock_id: i64,
    pub accession_number: String,
    pub form_type: String,
    pub filed_date: String,
    pub fiscal_period: Option<String>,
    pub fiscal_year: i32,
    pub report_date: String,
    pub file_size_bytes: Option<i64>,
    pub document_count: Option<i32>,
    pub is_amended: bool,
    pub created_at: String,
    pub updated_at: String,
}
```

2. **Update Freshness Checker** (`src-tauri/src/tools/data_freshness_checker.rs`):
```rust
// BEFORE
let query = r#"
    SELECT 
        s.cik,
        MAX(i.filed_date) as latest_filed_date
    FROM stocks s
    INNER JOIN income_statements i ON s.id = i.stock_id
    WHERE s.is_sp500 = 1 AND s.cik IS NOT NULL AND i.filed_date IS NOT NULL
    GROUP BY s.cik
"#;

// AFTER
let query = r#"
    SELECT 
        s.cik,
        MAX(sf.filed_date) as latest_filed_date
    FROM stocks s
    INNER JOIN sec_filings sf ON s.id = sf.stock_id
    WHERE s.is_sp500 = 1 AND s.cik IS NOT NULL AND sf.filed_date IS NOT NULL
    GROUP BY s.cik
"#;
```

#### **Phase 1.5: Testing Procedures**
```bash
# 1. Run all Phase 1 migrations
sqlx migrate run --database-url "sqlite:db/stocks.db" --source db/migrations

# 2. Verify sec_filings table
sqlite3 db/stocks.db "
SELECT COUNT(*) as total_filings FROM sec_filings;
SELECT COUNT(*) as linked_income FROM income_statements WHERE sec_filing_id IS NOT NULL;
SELECT COUNT(*) as linked_balance FROM balance_sheets WHERE sec_filing_id IS NOT NULL;
SELECT COUNT(*) as linked_cashflow FROM cash_flow_statements WHERE sec_filing_id IS NOT NULL;
"

# 3. Test application compilation
cargo check

# 4. Test freshness checker
cargo run --bin refresh_data -- --status

# 5. Verify no data loss
sqlite3 db/stocks.db "
SELECT COUNT(*) FROM income_statements;
SELECT COUNT(*) FROM balance_sheets;
SELECT COUNT(*) FROM cash_flow_statements;
-- Should be same as before
"
```

#### **Phase 1.6: Success Criteria**
- ✅ All migrations run without errors
- ✅ `sec_filings` table populated with unique filings
- ✅ All financial statements linked to `sec_filings`
- ✅ Freshness checker works with new schema
- ✅ No data loss
- ✅ Application compiles without warnings

---

### **PHASE 2: Remove Duplicate EDGAR Metadata**

**Risk Level**: 🟡 **MEDIUM RISK**  
**Impact**: 🟢 **HIGH IMPACT**  
**Dependencies**: ✅ **Phase 1 Complete**  
**Timeline**: **2-3 days**

#### **Phase 2.1: Remove Duplicate Columns**
```bash
sqlx migrate add remove_duplicate_edgar_metadata
```

**File**: `src-tauri/db/migrations/YYYYMMDDHHMMSS_remove_duplicate_edgar_metadata.sql`

```sql
-- Migration: Remove duplicate EDGAR metadata columns
-- Risk: MEDIUM (recreating tables)
-- Impact: Eliminates data duplication

-- Step 1: Create backup tables
CREATE TABLE income_statements_backup AS SELECT * FROM income_statements;
CREATE TABLE balance_sheets_backup AS SELECT * FROM balance_sheets;
CREATE TABLE cash_flow_statements_backup AS SELECT * FROM cash_flow_statements;

-- Step 2: Recreate income_statements without duplicate columns
CREATE TABLE income_statements_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    sec_filing_id INTEGER NOT NULL,
    period_type TEXT NOT NULL,
    report_date DATE NOT NULL,
    fiscal_year INTEGER,
    
    -- Financial data only
    revenue REAL,
    gross_profit REAL,
    operating_income REAL,
    net_income REAL,
    shares_basic REAL,
    shares_diluted REAL,
    cost_of_revenue REAL,
    research_development REAL,
    selling_general_admin REAL,
    depreciation_expense REAL,
    amortization_expense REAL,
    interest_expense REAL,
    
    -- Import metadata (keep minimal)
    currency TEXT DEFAULT 'USD',
    data_source TEXT DEFAULT 'sec_edgar',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    FOREIGN KEY (sec_filing_id) REFERENCES sec_filings(id),
    UNIQUE(stock_id, period_type, report_date)
);

-- Step 3: Copy data from old table
INSERT INTO income_statements_new 
SELECT 
    id, stock_id, sec_filing_id, period_type, report_date, fiscal_year,
    revenue, gross_profit, operating_income, net_income, shares_basic, shares_diluted,
    cost_of_revenue, research_development, selling_general_admin, depreciation_expense,
    amortization_expense, interest_expense, currency, data_source, created_at
FROM income_statements;

-- Step 4: Drop old table and rename new one
DROP TABLE income_statements;
ALTER TABLE income_statements_new RENAME TO income_statements;

-- Step 5: Recreate indexes
CREATE INDEX idx_income_statements_stock_id ON income_statements(stock_id);
CREATE INDEX idx_income_statements_sec_filing_id ON income_statements(sec_filing_id);
CREATE INDEX idx_income_statements_period_type ON income_statements(period_type);
CREATE INDEX idx_income_statements_report_date ON income_statements(report_date);

-- Step 6: Repeat for balance_sheets and cash_flow_statements
-- (Similar process for other tables)
```

#### **Phase 2.2: Testing Procedures**
```bash
# 1. Run migration
sqlx migrate run --database-url "sqlite:db/stocks.db" --source db/migrations

# 2. Verify data integrity
sqlite3 db/stocks.db "
SELECT COUNT(*) FROM income_statements;
SELECT COUNT(*) FROM balance_sheets;
SELECT COUNT(*) FROM cash_flow_statements;
-- Should be same as before
"

# 3. Test application
cargo check
cargo run --bin refresh_data -- --status

# 4. Verify no duplicate metadata
sqlite3 db/stocks.db "
SELECT COUNT(*) FROM income_statements WHERE accession_number IS NOT NULL;
-- Should be 0 (no more duplicate metadata)
"
```

#### **Phase 2.3: Success Criteria**
- ✅ All migrations run without errors
- ✅ Duplicate EDGAR metadata columns removed
- ✅ All financial statements still linked to `sec_filings`
- ✅ No data loss
- ✅ Application works correctly

---

### **PHASE 3: Price Coverage SQL View**

**Risk Level**: 🟢 **ZERO RISK**  
**Impact**: 🟢 **HIGH IMPACT**  
**Dependencies**: ✅ **Phase 0 Complete**  
**Timeline**: **1-2 days**

#### **Phase 3.1: Create Price Coverage View**
```bash
sqlx migrate add create_price_coverage_view
```

**File**: `src-tauri/db/migrations/YYYYMMDDHHMMSS_create_price_coverage_view.sql`

```sql
-- Migration: Create price data coverage view
-- Risk: ZERO (view only, no data changes)
-- Impact: Replaces company_metadata table

CREATE VIEW v_price_data_coverage AS
SELECT 
    s.id as stock_id,
    s.symbol,
    s.company_name,
    s.is_sp500,
    s.sector,
    
    -- Price data coverage (computed from daily_prices)
    MIN(dp.date) as earliest_data_date,
    MAX(dp.date) as latest_data_date,
    COUNT(dp.date) as total_trading_days,
    COUNT(DISTINCT dp.date) as unique_trading_days,
    
    -- Additional useful metrics
    AVG(dp.close_price) as avg_close_price,
    MAX(dp.close_price) as highest_close_price,
    MIN(dp.close_price) as lowest_close_price,
    SUM(dp.volume) as total_volume,
    
    -- Data freshness
    MAX(dp.last_updated) as last_price_update,
    
    -- Coverage status
    CASE 
        WHEN COUNT(dp.date) = 0 THEN 'No Data'
        WHEN MAX(dp.date) >= DATE('now', '-7 days') THEN 'Current'
        WHEN MAX(dp.date) >= DATE('now', '-30 days') THEN 'Recent'
        ELSE 'Stale'
    END as coverage_status
    
FROM stocks s
LEFT JOIN daily_prices dp ON s.id = dp.stock_id
GROUP BY s.id, s.symbol, s.company_name, s.is_sp500, s.sector;
```

#### **Phase 3.2: Code Updates**
**Files to Update**:
- `src-tauri/src/bin/import-schwab-prices.rs` - Remove `update_company_metadata` function
- `src-tauri/src/tools/date_range_calculator.rs` - Update to use view

**Specific Changes**:

1. **Remove `update_company_metadata` function** (`src-tauri/src/bin/import-schwab-prices.rs`):
```rust
// REMOVE this entire function
async fn update_company_metadata(&self, stock_id: i64, _symbol: &str) -> Result<()> {
    // ... entire function body
}
```

2. **Update date range calculator** (`src-tauri/src/tools/date_range_calculator.rs`):
```rust
// BEFORE
let query = r#"
    SELECT earliest_data_date, latest_data_date, total_trading_days
    FROM company_metadata 
    WHERE stock_id = ?
"#;

// AFTER
let query = r#"
    SELECT earliest_data_date, latest_data_date, total_trading_days, coverage_status
    FROM v_price_data_coverage 
    WHERE stock_id = ?
"#;
```

#### **Phase 3.3: Remove `company_metadata` Table**
```bash
sqlx migrate add remove_company_metadata_table
```

**File**: `src-tauri/db/migrations/YYYYMMDDHHMMSS_remove_company_metadata_table.sql`

```sql
-- Migration: Remove company_metadata table
-- Risk: LOW (replaced by view)
-- Impact: Eliminates redundant computed data

-- Step 1: Create backup (safety measure)
CREATE TABLE company_metadata_backup AS SELECT * FROM company_metadata;

-- Step 2: Drop table
DROP TABLE company_metadata;
```

#### **Phase 3.4: Testing Procedures**
```bash
# 1. Run migrations
sqlx migrate run --database-url "sqlite:db/stocks.db" --source db/migrations

# 2. Test view
sqlite3 db/stocks.db "
SELECT 
    symbol, 
    earliest_data_date, 
    latest_data_date, 
    total_trading_days,
    coverage_status
FROM v_price_data_coverage 
WHERE is_sp500 = 1 
LIMIT 5;
"

# 3. Test application
cargo check
cargo run --bin refresh_data -- --status

# 4. Test price import (should not update company_metadata)
cargo run --bin import-schwab-prices --test-symbol AAPL
```

#### **Phase 3.5: Success Criteria**
- ✅ View created successfully
- ✅ `company_metadata` table removed
- ✅ Application works with view
- ✅ Price import works without updating metadata
- ✅ Coverage status calculated correctly

---

## **PHASE COMPLETION CHECKLIST**

### **After Each Phase:**
- [ ] All migrations run without errors
- [ ] Application compiles without warnings
- [ ] Basic functionality tested
- [ ] No data loss verified
- [ ] Performance improved (where applicable)
- [ ] User approval received before proceeding

### **Overall Success Metrics:**
- [ ] Database size reduced by 30-40%
- [ ] Query performance improved
- [ ] Code simplified and cleaner
- [ ] No data integrity issues
- [ ] All functionality preserved

## **ROLLBACK PROCEDURES**

### **Phase 0 Rollback:**
```sql
DROP TABLE stocks;
ALTER TABLE stocks_backup RENAME TO stocks;
```

### **Phase 1 Rollback:**
```sql
-- Restore EDGAR metadata columns
ALTER TABLE income_statements ADD COLUMN accession_number TEXT;
ALTER TABLE income_statements ADD COLUMN form_type TEXT;
ALTER TABLE income_statements ADD COLUMN filed_date DATE;
ALTER TABLE income_statements ADD COLUMN fiscal_period TEXT;

-- Restore data from sec_filings
UPDATE income_statements 
SET 
    accession_number = sf.accession_number,
    form_type = sf.form_type,
    filed_date = sf.filed_date,
    fiscal_period = sf.fiscal_period
FROM sec_filings sf 
WHERE income_statements.sec_filing_id = sf.id;

-- Drop sec_filings table
DROP TABLE sec_filings;
```

### **Phase 2 Rollback:**
```sql
-- Restore tables from backup
DROP TABLE income_statements;
ALTER TABLE income_statements_backup RENAME TO income_statements;
-- Repeat for other tables
```

### **Phase 3 Rollback:**
```sql
-- Drop view
DROP VIEW v_price_data_coverage;

-- Restore company_metadata table
ALTER TABLE company_metadata_backup RENAME TO company_metadata;
```

## **NEXT STEPS**

1. **Review this plan** with the user
2. **Get approval** to proceed with Phase 0
3. **Execute Phase 0** with testing
4. **Get approval** to proceed with Phase 1
5. **Continue** with remaining phases

---

*Last Updated: 2025-10-03*  
*Version: 1.0 - Detailed Implementation Plan*
