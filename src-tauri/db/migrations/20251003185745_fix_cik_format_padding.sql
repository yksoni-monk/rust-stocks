-- Migration: Fix CIK Format Padding
-- Purpose: Correct the CIK format in stocks table to use proper 10-digit padding
-- Date: 2025-10-03
-- Issue: CIKs were incorrectly padded using LENGTH() instead of proper zero-padding

-- Update stocks table with correctly formatted CIKs from cik_mappings_sp500
UPDATE stocks 
SET cik = printf('%010d', CAST(cik_mappings_sp500.cik AS INTEGER))
FROM cik_mappings_sp500 
WHERE stocks.id = cik_mappings_sp500.stock_id 
  AND stocks.is_sp500 = 1 
  AND cik_mappings_sp500.cik IS NOT NULL 
  AND cik_mappings_sp500.cik != '';

-- Verify the fix by showing some examples
-- SELECT symbol, cik FROM stocks WHERE symbol IN ('AAPL', 'MSFT', 'GOOGL') AND is_sp500 = 1;
