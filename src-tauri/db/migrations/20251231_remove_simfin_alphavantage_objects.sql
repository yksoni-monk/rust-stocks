-- Migration: Remove SimFin and Alpha Vantage Database Objects
-- Created: 2025-12-31
-- Purpose: Clean up database objects for removed data sources
-- Safety: DROP operations only - no data preservation needed

-- ============================================================================
-- DROP SIMFIN RELATED OBJECTS
-- ============================================================================

-- Drop SimFin-specific tables
DROP TABLE IF EXISTS quarterly_financials;
DROP TABLE IF EXISTS simfin_import_log;

-- Drop SimFin-specific indexes
DROP INDEX IF EXISTS idx_quarterly_financials_stock_fiscal;
DROP INDEX IF EXISTS idx_quarterly_financials_simfin_id;
DROP INDEX IF EXISTS idx_stocks_simfin_id;

-- ============================================================================
-- DROP ALPHA VANTAGE RELATED OBJECTS
-- ============================================================================

-- Drop Alpha Vantage earnings table
DROP TABLE IF EXISTS earnings_data;

-- Drop Alpha Vantage indexes
DROP INDEX IF EXISTS idx_earnings_data_stock_fiscal;
DROP INDEX IF EXISTS idx_earnings_data_earnings_type;

-- ============================================================================
-- CLEAN UP DATA SOURCE REFERENCES
-- ============================================================================

-- Update data_source references from 'simfin' to 'unknown' in existing tables
UPDATE daily_prices SET data_source = 'unknown' WHERE data_source = 'simfin';
UPDATE income_statements SET data_source = 'unknown' WHERE data_source = 'simfin';
UPDATE balance_sheets SET data_source = 'unknown' WHERE data_source = 'simfin';

-- Update refresh configuration to remove SimFin/Alpha Vantage references
UPDATE refresh_configuration 
SET refresh_command = 'edgar-api-client' 
WHERE refresh_command IN ('concurrent-edgar-extraction', 'import-simfin', 'run_pe_calculation');

-- ============================================================================
-- VERIFICATION QUERIES
-- ============================================================================

-- Verify SimFin objects are removed
SELECT 'SimFin objects removed' as status
WHERE NOT EXISTS (
    SELECT 1 FROM sqlite_master 
    WHERE type IN ('table', 'index') 
    AND name LIKE '%simfin%'
);

-- Verify Alpha Vantage objects are removed
SELECT 'Alpha Vantage objects removed' as status
WHERE NOT EXISTS (
    SELECT 1 FROM sqlite_master 
    WHERE type IN ('table', 'index') 
    AND name LIKE '%earnings%'
);

-- Show remaining data sources
SELECT DISTINCT data_source, COUNT(*) as record_count
FROM (
    SELECT data_source FROM daily_prices
    UNION ALL
    SELECT data_source FROM income_statements
    UNION ALL
    SELECT data_source FROM balance_sheets
    UNION ALL
    SELECT data_source FROM cash_flow_statements
)
WHERE data_source IS NOT NULL
GROUP BY data_source
ORDER BY record_count DESC;
