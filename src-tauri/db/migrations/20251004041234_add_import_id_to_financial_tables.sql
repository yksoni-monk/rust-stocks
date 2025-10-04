-- Migration: Add import_id to financial tables
-- Risk: LOW RISK (adding nullable column)
-- Impact: Links financial records to import sessions

-- Add import_id column to all financial tables
ALTER TABLE income_statements ADD COLUMN import_id INTEGER;
ALTER TABLE balance_sheets ADD COLUMN import_id INTEGER;
ALTER TABLE cash_flow_statements ADD COLUMN import_id INTEGER;

-- Create indexes for better performance
CREATE INDEX idx_income_statements_import_id ON income_statements(import_id);
CREATE INDEX idx_balance_sheets_import_id ON balance_sheets(import_id);
CREATE INDEX idx_cash_flow_statements_import_id ON cash_flow_statements(import_id);