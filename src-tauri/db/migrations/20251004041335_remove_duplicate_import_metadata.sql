-- Migration: Remove duplicate import metadata columns
-- Risk: LOW RISK (removing redundant columns)
-- Impact: Eliminates import metadata duplication

-- Remove duplicate import metadata columns from financial tables
-- These are now tracked centrally in data_imports table

ALTER TABLE income_statements DROP COLUMN data_source;
ALTER TABLE income_statements DROP COLUMN created_at;

ALTER TABLE balance_sheets DROP COLUMN data_source;
ALTER TABLE balance_sheets DROP COLUMN created_at;

ALTER TABLE cash_flow_statements DROP COLUMN data_source;
ALTER TABLE cash_flow_statements DROP COLUMN created_at;