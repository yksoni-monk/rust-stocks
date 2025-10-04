-- Migration: Remove unused tables
-- Risk: ZERO RISK (tables have 0 references in application code)
-- Impact: Reduces database size and clutter

-- Drop unused tables in correct order (child tables first due to foreign keys)
DROP TABLE IF EXISTS daily_prices_enhanced;
DROP TABLE IF EXISTS real_time_quotes;
DROP TABLE IF EXISTS intraday_prices;
DROP TABLE IF EXISTS option_chains;
DROP TABLE IF EXISTS stocks_enhanced;

-- Drop other unused tables
DROP TABLE IF EXISTS data_import_status;
DROP TABLE IF EXISTS dividend_history;
DROP TABLE IF EXISTS edgar_field_mappings;

-- Drop unused view that references unused table
DROP VIEW IF EXISTS v_stock_data_coverage;