-- Migration: Cleanup backup tables
-- Risk: ZERO RISK (removing unused backup tables)
-- Impact: Reduces database size and clutter

-- Remove backup tables that were created during previous migration attempts
-- These are no longer needed since the migrations have been completed successfully

DROP TABLE IF EXISTS data_refresh_status_backup;
DROP TABLE IF EXISTS _sqlx_migrations_backup;
DROP TABLE IF EXISTS stocks_backup;
DROP TABLE IF EXISTS income_statements_backup;
DROP TABLE IF EXISTS balance_sheets_backup;
DROP TABLE IF EXISTS cash_flow_statements_backup;
