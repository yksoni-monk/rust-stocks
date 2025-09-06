-- Database Migration v2.0: Enhanced Data Fetching Architecture
-- Date: 2025-09-06
-- Description: Add earnings data table, processing status, and enhance daily_prices for Alpha Vantage integration

-- 1. Backup existing data (should be done before running this)
-- .backup stocks_backup_$(date +%Y%m%d).db

BEGIN TRANSACTION;

-- 2. Create earnings_data table for caching quarterly EPS
CREATE TABLE IF NOT EXISTS earnings_data (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER NOT NULL,
    fiscal_date_ending DATE NOT NULL,
    reported_date DATE,
    reported_eps REAL NOT NULL,
    estimated_eps REAL,
    surprise REAL,
    surprise_percentage REAL,
    report_time TEXT,
    earnings_type TEXT NOT NULL CHECK (earnings_type IN ('quarterly', 'annual')),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (stock_id) REFERENCES stocks(id) ON DELETE CASCADE,
    UNIQUE(stock_id, fiscal_date_ending, earnings_type)
);

-- 3. Create processing_status table for tracking bulk operations
CREATE TABLE IF NOT EXISTS processing_status (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER NOT NULL,
    data_type TEXT NOT NULL CHECK (data_type IN ('prices', 'earnings', 'fundamentals')),
    status TEXT NOT NULL CHECK (status IN ('pending', 'processing', 'completed', 'failed')),
    fetch_mode TEXT CHECK (fetch_mode IN ('compact', 'full')),
    records_processed INTEGER DEFAULT 0,
    total_records INTEGER DEFAULT 0,
    error_message TEXT,
    started_at DATETIME,
    completed_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (stock_id) REFERENCES stocks(id) ON DELETE CASCADE,
    UNIQUE(stock_id, data_type)
);

-- 4. Add new columns to daily_prices table
-- Check if columns already exist before adding
PRAGMA table_info(daily_prices);

-- Add data_source column (will be 'alpha_vantage' or 'schwab')
ALTER TABLE daily_prices ADD COLUMN data_source TEXT DEFAULT 'alpha_vantage';

-- Add last_updated column for tracking when data was fetched
ALTER TABLE daily_prices ADD COLUMN last_updated DATETIME DEFAULT CURRENT_TIMESTAMP;

-- 5. Create indexes for performance optimization
CREATE INDEX IF NOT EXISTS idx_earnings_stock_date ON earnings_data(stock_id, fiscal_date_ending);
CREATE INDEX IF NOT EXISTS idx_earnings_type ON earnings_data(earnings_type);
CREATE INDEX IF NOT EXISTS idx_earnings_stock_type ON earnings_data(stock_id, earnings_type);

CREATE INDEX IF NOT EXISTS idx_processing_stock_type ON processing_status(stock_id, data_type);
CREATE INDEX IF NOT EXISTS idx_processing_status ON processing_status(status);
CREATE INDEX IF NOT EXISTS idx_processing_stock_status ON processing_status(stock_id, status);

CREATE INDEX IF NOT EXISTS idx_daily_prices_source ON daily_prices(data_source);
CREATE INDEX IF NOT EXISTS idx_daily_prices_updated ON daily_prices(last_updated);
CREATE INDEX IF NOT EXISTS idx_daily_prices_source_date ON daily_prices(data_source, date);

-- 6. Clean existing data (remove old price data to start fresh)
DELETE FROM daily_prices WHERE 1=1;

-- 7. Clean metadata related to old fetching
DELETE FROM metadata WHERE key LIKE 'last_fetch_%';
DELETE FROM metadata WHERE key LIKE 'initialization_%';

-- 8. Add metadata for new system version
INSERT OR REPLACE INTO metadata (key, value, updated_at) VALUES 
    ('schema_version', '2.0', CURRENT_TIMESTAMP),
    ('migration_date', DATE('now'), CURRENT_TIMESTAMP),
    ('data_source_primary', 'alpha_vantage', CURRENT_TIMESTAMP),
    ('data_source_backup', 'schwab', CURRENT_TIMESTAMP);

-- 9. Create trigger to update last_updated timestamps
CREATE TRIGGER IF NOT EXISTS update_earnings_timestamp 
    AFTER UPDATE ON earnings_data
    FOR EACH ROW
BEGIN
    UPDATE earnings_data SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_processing_timestamp 
    AFTER UPDATE ON processing_status
    FOR EACH ROW
BEGIN
    UPDATE processing_status SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_daily_prices_timestamp 
    AFTER UPDATE ON daily_prices
    FOR EACH ROW
BEGIN
    UPDATE daily_prices SET last_updated = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

-- 10. Verify the schema
PRAGMA integrity_check;

COMMIT;

-- Display the new schema
.schema earnings_data
.schema processing_status
PRAGMA table_info(daily_prices);

-- Show statistics
SELECT 'Stocks Count' as table_name, COUNT(*) as records FROM stocks
UNION ALL
SELECT 'Daily Prices Count' as table_name, COUNT(*) as records FROM daily_prices
UNION ALL
SELECT 'Earnings Data Count' as table_name, COUNT(*) as records FROM earnings_data
UNION ALL
SELECT 'Processing Status Count' as table_name, COUNT(*) as records FROM processing_status;

SELECT 'Migration completed successfully at ' || CURRENT_TIMESTAMP as status;