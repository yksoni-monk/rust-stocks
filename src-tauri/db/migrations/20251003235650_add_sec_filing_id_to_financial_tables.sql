-- Migration: Add sec_filing_id references to financial statement tables
-- Risk: LOW (adding nullable foreign key columns)
-- Impact: Links financial records to normalized filing data

-- Add sec_filing_id column to income_statements
ALTER TABLE income_statements ADD COLUMN sec_filing_id INTEGER;

-- Add sec_filing_id column to balance_sheets  
ALTER TABLE balance_sheets ADD COLUMN sec_filing_id INTEGER;

-- Add sec_filing_id column to cash_flow_statements
ALTER TABLE cash_flow_statements ADD COLUMN sec_filing_id INTEGER;

-- Create indexes for the new foreign key columns
CREATE INDEX idx_income_statements_sec_filing_id ON income_statements(sec_filing_id);
CREATE INDEX idx_balance_sheets_sec_filing_id ON balance_sheets(sec_filing_id);
CREATE INDEX idx_cash_flow_statements_sec_filing_id ON cash_flow_statements(sec_filing_id);