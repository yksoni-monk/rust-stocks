# Database Normalization & Cleanup Plan

## Executive Summary

This document outlines a comprehensive plan to normalize the current database schema by eliminating data duplication and following SQL best practices. The current schema violates multiple normalization principles, leading to data redundancy, integrity issues, and maintenance overhead.

## Current Issues Analysis

### Major Data Duplication Problems

#### 1. **EDGAR Metadata Duplication** (CRITICAL)
- `accession_number`, `form_type`, `filed_date`, `fiscal_period` repeated in ALL 3 financial tables
- `edgar_accession`, `edgar_form`, `edgar_filed_date` (legacy) also duplicated
- **Violation**: 3NF - same filing metadata stored multiple times
- **Impact**: ~2,781 financial records × 4 metadata fields = 11,124 duplicate values

#### 2. **CIK Duplication**
- `stocks.cik` 
- `cik_mappings_sp500.cik`
- **Violation**: 2NF - CIK should be in one place only
- **Impact**: 497 S&P 500 CIKs stored twice

#### 3. **Company Information Duplication**
- `stocks.symbol`, `stocks.company_name`
- `cik_mappings_sp500.symbol`, `cik_mappings_sp500.company_name` 
- `company_metadata.symbol`, `company_metadata.company_name`
- **Violation**: 1NF - same company data in multiple tables
- **Impact**: 5,892+ stocks with duplicated company information

#### 4. **Computed Data Storage** (CRITICAL)
- `company_metadata.earliest_data_date`, `latest_data_date`, `total_trading_days`
- **Violation**: Storing computed data that can be derived from `daily_prices`
- **Impact**: Data inconsistency (stored values become outdated)
- **Example**: AAPL shows `latest_data_date = 2024-12-31` but actual data goes to `2025-10-01`

#### 6. **Obsolete Columns in `stocks` Table** (NEW)
- `simfin_id` (5,389 records) - **OBSOLETE**: No longer using SimFin data source
- `first_trading_date` (0 records) - **OBSOLETE**: Never populated, unused
- `gics_sector`, `gics_industry_group`, `gics_industry`, `gics_sub_industry` (503 records) - **OBSOLETE**: No code usage, duplicate `sector`/`industry`
- `currency` (5,892 records, all "USD") - **QUESTIONABLE**: Hardcoded to USD, no code usage
- `shares_outstanding` (0 records) - **OBSOLETE**: Never populated, belongs in financial statements
- `market_cap` (0 records) - **OBSOLETE**: Never populated, calculated field (price × shares)
- `industry` (0 records) - **OBSOLETE**: Never populated, unused
- `sector` (503 records) - **QUESTIONABLE**: Only S&P 500, used in screening algorithms
- `status` (5,892 records, all "active") - **OBSOLETE**: All stocks are active, no business logic to change status
- **Impact**: ~6,000 obsolete records and unused columns

## Proposed Normalized Schema Design

### Phase 1: EDGAR Filing Metadata Consolidation

#### New Table: `sec_filings`
```sql
CREATE TABLE sec_filings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    accession_number TEXT NOT NULL,
    form_type TEXT NOT NULL,           -- '10-K', '10-Q', etc.
    filed_date DATE NOT NULL,
    fiscal_period TEXT,                -- 'Q1', 'Q2', 'Q3', 'Q4', or NULL for annual
    fiscal_year INTEGER NOT NULL,
    report_date DATE NOT NULL,        -- End date of reporting period
    
    -- Filing metadata
    file_size_bytes INTEGER,
    document_count INTEGER,
    is_amended BOOLEAN DEFAULT 0,
    
    -- Audit fields
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, accession_number),
    UNIQUE(stock_id, form_type, report_date, fiscal_year) -- Prevent duplicate filings
);

-- Performance indexes
CREATE INDEX idx_sec_filings_stock_id ON sec_filings(stock_id);
CREATE INDEX idx_sec_filings_filed_date ON sec_filings(filed_date);
CREATE INDEX idx_sec_filings_form_type ON sec_filings(form_type);
CREATE INDEX idx_sec_filings_accession ON sec_filings(accession_number);
CREATE INDEX idx_sec_filings_report_date ON sec_filings(report_date);
```

#### Updated Financial Tables (Remove EDGAR Metadata)
```sql
-- income_statements (cleaned)
CREATE TABLE income_statements (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    sec_filing_id INTEGER NOT NULL,    -- NEW: Reference to sec_filings
    period_type TEXT NOT NULL,         -- 'Annual', 'Quarterly', 'TTM'
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

-- balance_sheets (cleaned)
CREATE TABLE balance_sheets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    sec_filing_id INTEGER NOT NULL,    -- NEW: Reference to sec_filings
    period_type TEXT NOT NULL,
    report_date DATE NOT NULL,
    fiscal_year INTEGER,
    
    -- Financial data only
    cash_and_equivalents REAL,
    short_term_debt REAL,
    long_term_debt REAL,
    total_debt REAL,
    total_assets REAL,
    total_liabilities REAL,
    total_equity REAL,
    shares_outstanding REAL,
    current_assets REAL,
    current_liabilities REAL,
    inventory REAL,
    accounts_receivable REAL,
    accounts_payable REAL,
    working_capital REAL,
    share_repurchases REAL,
    
    -- Import metadata (keep minimal)
    currency TEXT DEFAULT 'USD',
    data_source TEXT DEFAULT 'sec_edgar',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    FOREIGN KEY (sec_filing_id) REFERENCES sec_filings(id),
    UNIQUE(stock_id, period_type, report_date)
);

-- cash_flow_statements (cleaned)
CREATE TABLE cash_flow_statements (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    sec_filing_id INTEGER NOT NULL,    -- NEW: Reference to sec_filings
    period_type TEXT NOT NULL,
    report_date DATE NOT NULL,
    fiscal_year INTEGER,
    
    -- Financial data only
    operating_cash_flow REAL,
    depreciation_amortization REAL,
    depreciation_expense REAL,
    amortization_expense REAL,
    investing_cash_flow REAL,
    capital_expenditures REAL,
    financing_cash_flow REAL,
    dividends_paid REAL,
    share_repurchases REAL,
    net_cash_flow REAL,
    
    -- Import metadata (keep minimal)
    data_source TEXT DEFAULT 'sec_edgar',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    FOREIGN KEY (sec_filing_id) REFERENCES sec_filings(id),
    UNIQUE(stock_id, period_type, report_date)
);
```

### Phase 2: Company Information Consolidation

#### Enhanced `stocks` Table (Single Source of Truth)
```sql
CREATE TABLE stocks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT UNIQUE NOT NULL,
    company_name TEXT NOT NULL,
    cik TEXT UNIQUE,                    -- Single CIK location
    
    -- Classification
    sector TEXT,
    industry TEXT,
    gics_sector TEXT,
    gics_industry_group TEXT,
    gics_industry TEXT,
    gics_sub_industry TEXT,
    
    -- Market data
    market_cap REAL,
    shares_outstanding INTEGER,
    currency TEXT DEFAULT 'USD',
    
    -- Status and dates
    status TEXT DEFAULT 'active',
    first_trading_date DATE,
    last_updated DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    -- Flags
    is_sp500 BOOLEAN DEFAULT 0,
    
    -- External IDs
    simfin_id INTEGER,
    
    -- Indexes
    INDEX idx_stocks_symbol ON stocks(symbol),
    INDEX idx_stocks_cik ON stocks(cik),
    INDEX idx_stocks_sp500 ON stocks(is_sp500),
    INDEX idx_stocks_sector ON stocks(sector),
    INDEX idx_stocks_industry ON stocks(industry)
);
```

#### Remove Redundant Tables
- **`cik_mappings_sp500`** → Data moved to `stocks` table
- **`company_metadata`** → Replaced with SQL view (computed data)

### Phase 2.5: Price Data Coverage SQL View

#### New View: `v_price_data_coverage`
```sql
CREATE VIEW v_price_data_coverage AS
SELECT 
    s.id as stock_id,
    s.symbol,
    s.company_name,
    s.is_sp500,
    s.sector,
    s.industry,
    
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
GROUP BY s.id, s.symbol, s.company_name, s.is_sp500, s.sector, s.industry;
```

#### Benefits of SQL View Approach
- **✅ Always Accurate**: Real-time data from `daily_prices` table
- **✅ No Sync Issues**: Cannot get out of sync because it's computed on-demand
- **✅ No Update Overhead**: No need to update coverage after price imports
- **✅ Better Performance**: Uses indexes on base tables, database can cache results
- **✅ More Flexible**: Easy to add new metrics, filtering, and calculations
- **✅ Simpler Code**: Single query gets all coverage info, no maintenance needed

### Phase 2.6: Market Cap SQL View

#### New View: `v_market_cap_calculated`
```sql
CREATE VIEW v_market_cap_calculated AS
SELECT 
    s.id as stock_id,
    s.symbol,
    s.company_name,
    s.is_sp500,
    s.sector,
    
    -- Latest price data
    dp.close_price as current_price,
    dp.date as price_date,
    
    -- Latest shares outstanding from balance sheet
    bs.shares_outstanding,
    
    -- Calculated market cap
    CASE 
        WHEN dp.close_price IS NOT NULL AND bs.shares_outstanding IS NOT NULL 
        THEN dp.close_price * bs.shares_outstanding
        ELSE NULL
    END as market_cap,
    
    -- Market cap category
    CASE 
        WHEN dp.close_price * bs.shares_outstanding >= 200_000_000_000 THEN 'Mega Cap'  -- $200B+
        WHEN dp.close_price * bs.shares_outstanding >= 10_000_000_000 THEN 'Large Cap'   -- $10B+
        WHEN dp.close_price * bs.shares_outstanding >= 2_000_000_000 THEN 'Mid Cap'     -- $2B+
        WHEN dp.close_price * bs.shares_outstanding >= 300_000_000 THEN 'Small Cap'    -- $300M+
        WHEN dp.close_price * bs.shares_outstanding >= 50_000_000 THEN 'Micro Cap'     -- $50M+
        ELSE 'Nano Cap'
    END as market_cap_category
    
FROM stocks s
LEFT JOIN daily_prices dp ON s.id = dp.stock_id 
    AND dp.date = (SELECT MAX(date) FROM daily_prices WHERE stock_id = s.id)
LEFT JOIN balance_sheets bs ON s.id = bs.stock_id 
    AND bs.report_date = (SELECT MAX(report_date) FROM balance_sheets WHERE stock_id = s.id)
WHERE s.status = 'active';
```

#### Benefits of Market Cap View
- **✅ Always Accurate**: Real-time calculation from current price and shares
- **✅ No Storage Overhead**: No duplicate data stored
- **✅ Market Cap Categories**: Automatic classification
- **✅ Flexible**: Easy to add new calculations (P/E, P/B, etc.)

### Phase 3: Import Metadata Consolidation

#### New Table: `data_imports`
```sql
CREATE TABLE data_imports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    import_type TEXT NOT NULL,          -- 'sec_edgar', 'schwab_prices', 'simfin'
    import_date DATE NOT NULL,
    source_file TEXT,
    records_imported INTEGER,
    status TEXT DEFAULT 'completed',    -- 'completed', 'failed', 'partial'
    
    -- Metadata
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    UNIQUE(import_type, import_date)
);
```

### Phase 0: Obsolete Columns Cleanup (SAFEST - Start Here)

#### Step 0.1: Remove Obsolete Columns from `stocks` Table
```sql
-- Migration: 20251004000000_remove_obsolete_stocks_columns.sql
-- Note: SQLite doesn't support DROP COLUMN directly, need to recreate table

-- Create new stocks table without obsolete columns
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
    is_sp500 BOOLEAN DEFAULT 0,
    
    -- Indexes
    INDEX idx_stocks_symbol ON stocks_new(symbol),
    INDEX idx_stocks_cik ON stocks_new(cik),
    INDEX idx_stocks_sp500 ON stocks_new(is_sp500),
    INDEX idx_stocks_sector ON stocks_new(sector)
);

-- Copy data from old table (excluding obsolete columns)
INSERT INTO stocks_new 
SELECT 
    id, symbol, company_name, cik, sector, 
    last_updated, created_at, is_sp500
FROM stocks;

-- Drop old table and rename new one
DROP TABLE stocks;
ALTER TABLE stocks_new RENAME TO stocks;
```

#### Benefits of Phase 0 Cleanup
- **✅ Immediate Storage Reduction**: Remove ~6,000 obsolete records
- **✅ Zero Risk**: No data loss (obsolete columns are unused)
- **✅ Cleaner Schema**: Remove confusion from unused fields
- **✅ Better Performance**: Smaller table, faster queries
- **✅ Easier Maintenance**: Less columns to manage

## Incremental Migration Strategy

### Phase 1: EDGAR Metadata Consolidation (SAFEST)

#### Step 1.1: Create `sec_filings` Table
```sql
-- Migration: 20251004000001_create_sec_filings_table.sql
CREATE TABLE sec_filings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    accession_number TEXT NOT NULL,
    form_type TEXT NOT NULL,
    filed_date DATE NOT NULL,
    fiscal_period TEXT,
    fiscal_year INTEGER NOT NULL,
    report_date DATE NOT NULL,
    
    file_size_bytes INTEGER,
    document_count INTEGER,
    is_amended BOOLEAN DEFAULT 0,
    
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, accession_number),
    UNIQUE(stock_id, form_type, report_date, fiscal_year)
);

CREATE INDEX idx_sec_filings_stock_id ON sec_filings(stock_id);
CREATE INDEX idx_sec_filings_filed_date ON sec_filings(filed_date);
CREATE INDEX idx_sec_filings_form_type ON sec_filings(form_type);
CREATE INDEX idx_sec_filings_accession ON sec_filings(accession_number);
CREATE INDEX idx_sec_filings_report_date ON sec_filings(report_date);
```

#### Step 1.2: Populate `sec_filings` from Existing Data
```sql
-- Migration: 20251004000002_populate_sec_filings.sql
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

-- Handle balance_sheets and cash_flow_statements if they have different filings
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
```

#### Step 1.3: Add `sec_filing_id` to Financial Tables
```sql
-- Migration: 20251004000003_add_sec_filing_reference.sql
ALTER TABLE income_statements ADD COLUMN sec_filing_id INTEGER;
ALTER TABLE balance_sheets ADD COLUMN sec_filing_id INTEGER;
ALTER TABLE cash_flow_statements ADD COLUMN sec_filing_id INTEGER;

-- Note: SQLite doesn't support ALTER TABLE ADD CONSTRAINT
-- Foreign key constraints will be enforced at application level
```

#### Step 1.4: Update Financial Records with `sec_filing_id`
```sql
-- Migration: 20251004000004_link_financial_records.sql
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

#### Step 1.5: Remove Duplicate EDGAR Metadata
```sql
-- Migration: 20251004000005_remove_duplicate_edgar_metadata.sql
-- Note: SQLite doesn't support DROP COLUMN directly
-- We'll need to recreate tables without duplicate columns

-- Create new income_statements table
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

-- Copy data from old table
INSERT INTO income_statements_new 
SELECT 
    id, stock_id, sec_filing_id, period_type, report_date, fiscal_year,
    revenue, gross_profit, operating_income, net_income, shares_basic, shares_diluted,
    cost_of_revenue, research_development, selling_general_admin, depreciation_expense,
    amortization_expense, interest_expense, currency, data_source, created_at
FROM income_statements;

-- Drop old table and rename new one
DROP TABLE income_statements;
ALTER TABLE income_statements_new RENAME TO income_statements;

-- Recreate indexes
CREATE INDEX idx_income_statements_stock_id ON income_statements(stock_id);
CREATE INDEX idx_income_statements_sec_filing_id ON income_statements(sec_filing_id);
CREATE INDEX idx_income_statements_period_type ON income_statements(period_type);
CREATE INDEX idx_income_statements_report_date ON income_statements(report_date);

-- Similar process for balance_sheets and cash_flow_statements
```

### Phase 2: Company Information Consolidation

#### Step 2.1: Enhance `stocks` Table
```sql
-- Migration: 20251004000006_enhance_stocks_table.sql
ALTER TABLE stocks ADD COLUMN gics_sector TEXT;
ALTER TABLE stocks ADD COLUMN gics_industry_group TEXT;
ALTER TABLE stocks ADD COLUMN gics_industry TEXT;
ALTER TABLE stocks ADD COLUMN gics_sub_industry TEXT;

-- Add CIK if not exists
ALTER TABLE stocks ADD COLUMN cik TEXT;

-- Add indexes
CREATE INDEX idx_stocks_cik ON stocks(cik);
CREATE INDEX idx_stocks_sp500 ON stocks(is_sp500);
CREATE INDEX idx_stocks_sector ON stocks(sector);
CREATE INDEX idx_stocks_industry ON stocks(industry);
```

#### Step 2.2: Migrate Data from `cik_mappings_sp500`
```sql
-- Migration: 20251004000007_migrate_cik_mappings.sql
UPDATE stocks 
SET cik = (
    SELECT cik 
    FROM cik_mappings_sp500 
    WHERE cik_mappings_sp500.stock_id = stocks.id
)
WHERE EXISTS (
    SELECT 1 FROM cik_mappings_sp500 
    WHERE cik_mappings_sp500.stock_id = stocks.id
);
```

#### Step 2.3: Create Price Coverage SQL View
```sql
-- Migration: 20251004000008_create_price_coverage_view.sql
CREATE VIEW v_price_data_coverage AS
SELECT 
    s.id as stock_id,
    s.symbol,
    s.company_name,
    s.is_sp500,
    s.sector,
    s.industry,
    
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
GROUP BY s.id, s.symbol, s.company_name, s.is_sp500, s.sector, s.industry;
```

#### Step 2.4: Update Application Code to Use View
```rust
// Before: Query company_metadata table
let query = r#"
    SELECT earliest_data_date, latest_data_date, total_trading_days
    FROM company_metadata 
    WHERE stock_id = ?
"#;

// After: Query the view
let query = r#"
    SELECT earliest_data_date, latest_data_date, total_trading_days, coverage_status
    FROM v_price_data_coverage 
    WHERE stock_id = ?
"#;
```

#### Step 2.5: Remove `company_metadata` Table
```sql
-- Migration: 20251004000009_remove_company_metadata.sql
DROP TABLE company_metadata;
```

### Phase 3: Import Metadata Consolidation

#### Step 3.1: Create `data_imports` Table
```sql
-- Migration: 20251004000010_create_data_imports.sql
CREATE TABLE data_imports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    import_type TEXT NOT NULL,
    import_date DATE NOT NULL,
    source_file TEXT,
    records_imported INTEGER,
    status TEXT DEFAULT 'completed',
    
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    UNIQUE(import_type, import_date)
);

CREATE INDEX idx_data_imports_type ON data_imports(import_type);
CREATE INDEX idx_data_imports_date ON data_imports(import_date);
```

#### Step 3.2: Populate `data_imports` from Existing Data
```sql
-- Migration: 20251004000011_populate_data_imports.sql
INSERT INTO data_imports (import_type, import_date, records_imported, status)
SELECT 
    'sec_edgar' as import_type,
    DATE(created_at) as import_date,
    COUNT(*) as records_imported,
    'completed' as status
FROM income_statements 
WHERE data_source = 'sec_edgar'
GROUP BY DATE(created_at);

-- Similar for balance_sheets and cash_flow_statements
```

## Benefits of Normalized Design

### 1. Data Integrity
- **Single Source of Truth**: Each piece of data stored once
- **Referential Integrity**: Foreign key constraints prevent orphaned records
- **Consistency**: No more sync issues between duplicated data
- **Validation**: Centralized validation rules

### 2. Storage Efficiency
- **Reduced Database Size**: Eliminate duplicate EDGAR metadata (~30% reduction expected)
- **Better Compression**: Normalized data compresses better
- **Faster Backups**: Less data to backup and restore
- **Memory Usage**: Smaller tables use less RAM

### 3. Query Performance
- **Faster Joins**: Smaller tables with proper indexes
- **Better Caching**: Database can cache smaller normalized tables
- **Optimized Queries**: Can query only needed data
- **Index Efficiency**: Better index utilization on smaller tables

### 4. Maintenance Benefits
- **Easier Updates**: Update filing metadata in one place
- **Simplified Code**: No need to sync duplicate data
- **Better Testing**: Easier to validate data integrity
- **Reduced Bugs**: Fewer places for data inconsistencies

### 5. Scalability
- **Future Growth**: Schema can handle more data efficiently
- **New Features**: Easier to add new financial statement types
- **API Integration**: Cleaner data model for external APIs
- **Reporting**: Better foundation for analytics and reporting

## Code Changes Required

### Freshness Checker Updates
```rust
// Before: Query multiple tables for EDGAR metadata
let query = r#"
    SELECT s.cik, MAX(i.filed_date) as latest_filed_date
    FROM stocks s
    INNER JOIN income_statements i ON s.id = i.stock_id
    WHERE s.is_sp500 = 1 AND i.filed_date IS NOT NULL
    GROUP BY s.cik
"#;

// After: Query normalized sec_filings table
let query = r#"
    SELECT s.cik, MAX(sf.filed_date) as latest_filed_date
    FROM stocks s
    INNER JOIN sec_filings sf ON s.id = sf.stock_id
    WHERE s.is_sp500 = 1 AND sf.filed_date IS NOT NULL
    GROUP BY s.cik
"#;
```

### Price Coverage Updates
```rust
// Before: Update company_metadata after price import
async fn update_company_metadata(&self, stock_id: i64, _symbol: &str) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE company_metadata
        SET
            earliest_data_date = (SELECT MIN(date) FROM daily_prices WHERE stock_id = ?),
            latest_data_date = (SELECT MAX(date) FROM daily_prices WHERE stock_id = ?),
            total_trading_days = (SELECT COUNT(*) FROM daily_prices WHERE stock_id = ?),
            updated_at = CURRENT_TIMESTAMP
        WHERE stock_id = ?
        "#
    )
    .bind(stock_id)
    .bind(stock_id)
    .bind(stock_id)
    .bind(stock_id)
    .execute(&self.pool)
    .await?;
    Ok(())
}

// After: No update needed - view computes automatically
// Simply query the view when needed
let query = r#"
    SELECT earliest_data_date, latest_data_date, total_trading_days, coverage_status
    FROM v_price_data_coverage 
    WHERE stock_id = ?
"#;
```

### Data Population Updates
```rust
// Before: Insert EDGAR metadata into each financial table
// After: Insert into sec_filings once, reference from financial tables

// New pattern:
let filing_id = insert_sec_filing(stock_id, accession_number, form_type, filed_date).await?;
insert_income_statement(stock_id, filing_id, financial_data).await?;
insert_balance_sheet(stock_id, filing_id, financial_data).await?;
insert_cash_flow(stock_id, filing_id, financial_data).await?;
```

### Application Code Updates
```rust
// New struct for SEC filings
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

// Updated financial statement structs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomeStatement {
    pub id: i64,
    pub stock_id: i64,
    pub sec_filing_id: i64,  // NEW: Reference to sec_filings
    pub period_type: String,
    pub report_date: String,
    pub fiscal_year: Option<i32>,
    
    // Financial data
    pub revenue: Option<f64>,
    pub gross_profit: Option<f64>,
    pub operating_income: Option<f64>,
    pub net_income: Option<f64>,
    // ... other financial fields
    
    // Import metadata
    pub currency: String,
    pub data_source: String,
    pub created_at: String,
}
```

## Migration Risks & Mitigation

### Risks
1. **Data Loss**: Accidental deletion during migration
2. **Application Breakage**: Code expecting old schema
3. **Performance Impact**: Large data migration operations
4. **Rollback Complexity**: Hard to undo normalized changes
5. **Downtime**: Application unavailable during migration

### Mitigation Strategies
1. **Comprehensive Backups**: Full database backup before each phase
2. **Incremental Testing**: Test each phase thoroughly before proceeding
3. **Parallel Schema**: Keep old columns during transition period
4. **Rollback Scripts**: Prepare rollback migrations for each phase
5. **Application Compatibility**: Update code gradually to use new schema
6. **Staged Deployment**: Deploy changes in small, testable increments
7. **Data Validation**: Verify data integrity after each migration step

### Rollback Procedures
```sql
-- Rollback for Phase 1: Restore EDGAR metadata columns
ALTER TABLE income_statements ADD COLUMN accession_number TEXT;
ALTER TABLE income_statements ADD COLUMN form_type TEXT;
ALTER TABLE income_statements ADD COLUMN filed_date DATE;
ALTER TABLE income_statements ADD COLUMN fiscal_period TEXT;

-- Restore data from sec_filings table
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

## Recommended Approach

### Start with Phase 0 (Obsolete Columns Cleanup)
- **Lowest Risk**: Obsolete columns are unused, zero data loss risk
- **Immediate Impact**: Cleaner schema, better performance
- **Zero Dependencies**: No application code changes needed
- **Quick Win**: Can be done immediately

### Then Phase 1 (EDGAR Metadata)
- **Low Risk**: EDGAR metadata is relatively isolated
- **High Impact**: Eliminates most duplication
- **Clear Benefits**: Immediate storage and performance improvements

### Timeline
- **Week 1**: Create `sec_filings` table and populate
- **Week 2**: Add `sec_filing_id` references to financial tables
- **Week 3**: Update application code to use new schema
- **Week 4**: Remove duplicate EDGAR metadata columns
- **Week 5**: Testing and validation

### Success Metrics
- **Database Size Reduction**: Target 20-30% reduction
- **Query Performance**: Faster freshness checks and financial queries
- **Code Simplification**: Fewer duplicate data handling paths
- **Data Integrity**: Zero orphaned records or sync issues
- **Maintenance Overhead**: Reduced time spent on data synchronization

### Testing Strategy
1. **Unit Tests**: Test each migration step individually
2. **Integration Tests**: Verify application functionality with new schema
3. **Performance Tests**: Measure query performance improvements
4. **Data Integrity Tests**: Verify no data loss during migration
5. **Rollback Tests**: Ensure rollback procedures work correctly

## Conclusion

This normalized design follows **3NF (Third Normal Form)** principles and will significantly improve:
- **Data Integrity**: Single source of truth for all data
- **Performance**: Faster queries and reduced storage
- **Maintainability**: Easier code maintenance and fewer bugs
- **Scalability**: Better foundation for future growth

The incremental approach minimizes risks while maximizing benefits, starting with the safest and most impactful changes first.

---

*Last Updated: 2025-10-03*
*Version: 1.0 - Initial Database Normalization Plan*
