-- Migration: Enhance balance_sheets table with EDGAR data fields
-- Purpose: Add missing balance sheet fields for complete O'Shaughnessy and Piotroski implementations

-- Add current assets and liabilities for current ratio calculation
ALTER TABLE balance_sheets ADD COLUMN current_assets REAL; -- AssetsCurrent
ALTER TABLE balance_sheets ADD COLUMN current_liabilities REAL; -- LiabilitiesCurrent

-- Add detailed asset and liability breakdowns
ALTER TABLE balance_sheets ADD COLUMN inventory REAL; -- InventoryNet
ALTER TABLE balance_sheets ADD COLUMN accounts_receivable REAL; -- AccountsReceivableNetCurrent
ALTER TABLE balance_sheets ADD COLUMN accounts_payable REAL; -- AccountsPayableCurrent

-- Add calculated working capital
ALTER TABLE balance_sheets ADD COLUMN working_capital REAL; -- current_assets - current_liabilities

-- Add EDGAR metadata for traceability
ALTER TABLE balance_sheets ADD COLUMN edgar_accession TEXT;
ALTER TABLE balance_sheets ADD COLUMN edgar_form TEXT; -- '10-K', '10-Q'
ALTER TABLE balance_sheets ADD COLUMN edgar_filed_date DATE;

-- Update existing records to set data_source
UPDATE balance_sheets SET data_source = 'simfin' WHERE data_source IS NULL;