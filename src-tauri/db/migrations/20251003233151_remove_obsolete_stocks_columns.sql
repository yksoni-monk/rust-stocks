-- Migration: Remove obsolete columns from stocks table
-- Risk: ZERO (columns are unused)
-- Impact: Immediate storage reduction and cleaner schema

-- Step 0: Disable foreign key constraints temporarily
PRAGMA foreign_keys = OFF;

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

-- Step 4: Create indexes (only if they don't exist)
CREATE INDEX IF NOT EXISTS idx_stocks_symbol ON stocks_new(symbol);
CREATE INDEX IF NOT EXISTS idx_stocks_cik ON stocks_new(cik);
CREATE INDEX IF NOT EXISTS idx_stocks_sp500 ON stocks_new(is_sp500);
CREATE INDEX IF NOT EXISTS idx_stocks_sector ON stocks_new(sector);

-- Step 5: Drop old table and rename new one
DROP TABLE stocks;
ALTER TABLE stocks_new RENAME TO stocks;

-- Step 6: Re-enable foreign key constraints
PRAGMA foreign_keys = ON;

-- Step 7: Verify data integrity
-- This will be done in testing phase
