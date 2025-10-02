-- Migration: Add filing metadata columns to financial statement tables
-- Purpose: Enable filing-based freshness checking using SEC Company Facts API metadata
-- Date: 2025-10-02

-- Add filing metadata columns to income_statements
ALTER TABLE income_statements ADD COLUMN accession_number TEXT;
ALTER TABLE income_statements ADD COLUMN form_type TEXT;
ALTER TABLE income_statements ADD COLUMN filed_date DATE;
ALTER TABLE income_statements ADD COLUMN fiscal_period TEXT; -- FY, Q1, Q2, Q3, Q4

-- Add filing metadata columns to balance_sheets
ALTER TABLE balance_sheets ADD COLUMN accession_number TEXT;
ALTER TABLE balance_sheets ADD COLUMN form_type TEXT;
ALTER TABLE balance_sheets ADD COLUMN filed_date DATE;
ALTER TABLE balance_sheets ADD COLUMN fiscal_period TEXT;

-- Add filing metadata columns to cash_flow_statements
ALTER TABLE cash_flow_statements ADD COLUMN accession_number TEXT;
ALTER TABLE cash_flow_statements ADD COLUMN form_type TEXT;
ALTER TABLE cash_flow_statements ADD COLUMN filed_date DATE;
ALTER TABLE cash_flow_statements ADD COLUMN fiscal_period TEXT;

-- Add indexes for efficient filing date lookups
CREATE INDEX IF NOT EXISTS idx_income_statements_filed_date ON income_statements(filed_date);
CREATE INDEX IF NOT EXISTS idx_balance_sheets_filed_date ON balance_sheets(filed_date);
CREATE INDEX IF NOT EXISTS idx_cash_flow_statements_filed_date ON cash_flow_statements(filed_date);

-- Add index for accession number lookups (unique filing identifiers)
CREATE INDEX IF NOT EXISTS idx_income_statements_accn ON income_statements(accession_number);
CREATE INDEX IF NOT EXISTS idx_balance_sheets_accn ON balance_sheets(accession_number);
CREATE INDEX IF NOT EXISTS idx_cash_flow_statements_accn ON cash_flow_statements(accession_number);

-- Add table to track latest SEC filings per stock
CREATE TABLE IF NOT EXISTS sec_filing_tracking (
    stock_id INTEGER PRIMARY KEY,
    cik TEXT NOT NULL,
    latest_filing_date TEXT,
    latest_10k_date TEXT,
    latest_10q_date TEXT,
    last_checked TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (stock_id) REFERENCES stocks (id)
);

-- Add indexes for efficient lookup
CREATE INDEX IF NOT EXISTS idx_sec_filing_tracking_cik ON sec_filing_tracking(cik);
CREATE INDEX IF NOT EXISTS idx_sec_filing_tracking_last_checked ON sec_filing_tracking(last_checked);

-- Initialize tracking for existing stocks with CIKs
INSERT OR IGNORE INTO sec_filing_tracking (stock_id, cik)
SELECT id, cik FROM stocks WHERE is_sp500 = 1 AND cik IS NOT NULL AND cik != '';
