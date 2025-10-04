-- Migration: Remove duplicate EDGAR metadata columns (Simple Approach)
-- Risk: LOW (dropping columns only)
-- Impact: Eliminates data duplication, reduces storage
-- Dependencies: Phase 1 complete (sec_filings table exists)

-- Step 1: Create backup tables (safety measure)
CREATE TABLE income_statements_backup AS SELECT * FROM income_statements;
CREATE TABLE balance_sheets_backup AS SELECT * FROM balance_sheets;
CREATE TABLE cash_flow_statements_backup AS SELECT * FROM cash_flow_statements;

-- Step 2: Remove duplicate EDGAR metadata columns from income_statements
-- These columns are now available in sec_filings table via sec_filing_id
ALTER TABLE income_statements DROP COLUMN accession_number;
ALTER TABLE income_statements DROP COLUMN form_type;
ALTER TABLE income_statements DROP COLUMN filed_date;
ALTER TABLE income_statements DROP COLUMN fiscal_period;
ALTER TABLE income_statements DROP COLUMN edgar_accession;
ALTER TABLE income_statements DROP COLUMN edgar_form;
ALTER TABLE income_statements DROP COLUMN edgar_filed_date;

-- Step 3: Remove duplicate EDGAR metadata columns from balance_sheets
ALTER TABLE balance_sheets DROP COLUMN accession_number;
ALTER TABLE balance_sheets DROP COLUMN form_type;
ALTER TABLE balance_sheets DROP COLUMN filed_date;
ALTER TABLE balance_sheets DROP COLUMN fiscal_period;
ALTER TABLE balance_sheets DROP COLUMN edgar_accession;
ALTER TABLE balance_sheets DROP COLUMN edgar_form;
ALTER TABLE balance_sheets DROP COLUMN edgar_filed_date;

-- Step 4: Remove duplicate EDGAR metadata columns from cash_flow_statements
ALTER TABLE cash_flow_statements DROP COLUMN accession_number;
ALTER TABLE cash_flow_statements DROP COLUMN form_type;
ALTER TABLE cash_flow_statements DROP COLUMN filed_date;
ALTER TABLE cash_flow_statements DROP COLUMN fiscal_period;
ALTER TABLE cash_flow_statements DROP COLUMN edgar_accession;
ALTER TABLE cash_flow_statements DROP COLUMN edgar_form;
ALTER TABLE cash_flow_statements DROP COLUMN edgar_filed_date;

-- Step 5: Verify data integrity (will be done in testing phase)
-- This migration removes duplicate EDGAR metadata columns:
-- - accession_number (now in sec_filings)
-- - form_type (now in sec_filings)  
-- - filed_date (now in sec_filings)
-- - fiscal_period (now in sec_filings)
-- - edgar_accession (duplicate of accession_number)
-- - edgar_form (duplicate of form_type)
-- - edgar_filed_date (duplicate of filed_date)
--
-- All EDGAR metadata is now centralized in sec_filings table
-- and accessible via sec_filing_id foreign key