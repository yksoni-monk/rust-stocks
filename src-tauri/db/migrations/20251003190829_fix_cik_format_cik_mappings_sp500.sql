-- Migration: Fix CIK format in cik_mappings_sp500 table
-- Purpose: Convert CIK values to proper 10-digit format with leading zeros
-- Date: 2025-10-03
-- Issue: CIK values are stored without proper 10-digit padding (e.g., 320193 instead of 0000320193)

-- Update cik_mappings_sp500 table with correctly formatted CIKs
UPDATE cik_mappings_sp500 
SET cik = printf('%010d', CAST(cik AS INTEGER))
WHERE cik IS NOT NULL 
  AND cik != ''
  AND LENGTH(cik) < 10;

-- Verify the fix by showing some examples
-- SELECT symbol, cik FROM cik_mappings_sp500 WHERE symbol IN ('AAPL', 'MSFT', 'GOOGL') ORDER BY symbol;
