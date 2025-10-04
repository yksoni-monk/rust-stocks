-- Migration: Populate sec_filings from existing financial statement data
-- Risk: LOW (read-only operation, uses INSERT OR IGNORE)
-- Impact: Creates normalized filing data from existing metadata

-- Step 1: Insert from income_statements (handle duplicates gracefully)
INSERT OR IGNORE INTO sec_filings (stock_id, accession_number, form_type, filed_date, fiscal_period, fiscal_year, report_date)
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