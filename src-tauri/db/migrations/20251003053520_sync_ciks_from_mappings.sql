-- Migration: Sync CIK values from cik_mappings_sp500 to stocks table
-- Purpose: Populate CIK column in stocks table using existing mappings from cik_mappings_sp500
-- Date: 2025-10-03

-- Update stocks table with CIKs from cik_mappings_sp500
UPDATE stocks 
SET cik = '000' || LENGTH(cik_mappings_sp500.cik) || cik_mappings_sp500.cik
FROM cik_mappings_sp500 
WHERE stocks.id = cik_mappings_sp500.stock_id 
  AND stocks.is_sp500 = 1 
  AND cik_mappings_sp500.cik IS NOT NULL 
  AND cik_mappings_sp500.cik != '';

-- Add comments to show progress
-- SELECT COUNT(*) as stocks_with_ciks FROM stocks WHERE is_sp500 = 1 AND cik IS NOT NULL AND cik != '';