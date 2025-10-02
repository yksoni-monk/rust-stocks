-- Migration: Add CIK column to stocks table for EDGAR API integration
-- Purpose: Enable EDGAR financial data extraction using SEC Company Facts API

-- Add CIK column to stocks table
ALTER TABLE stocks ADD COLUMN cik TEXT;

-- Add index for CIK lookups
CREATE INDEX IF NOT EXISTS idx_stocks_cik ON stocks(cik);

-- Update existing S&P 500 stocks with CIK values (common ones)
UPDATE stocks SET cik = '0000320193' WHERE symbol = 'AAPL';
UPDATE stocks SET cik = '0000789019' WHERE symbol = 'MSFT';
UPDATE stocks SET cik = '0001652044' WHERE symbol = 'GOOGL';
UPDATE stocks SET cik = '0001018724' WHERE symbol = 'AMZN';
UPDATE stocks SET cik = '0001318605' WHERE symbol = 'TSLA';
UPDATE stocks SET cik = '0001067983' WHERE symbol = 'META';
UPDATE stocks SET cik = '0001045810' WHERE symbol = 'NVDA';
UPDATE stocks SET cik = '0000078003' WHERE symbol = 'JPM';
UPDATE stocks SET cik = '0000004962' WHERE symbol = 'JNJ';
UPDATE stocks SET cik = '0000079038' WHERE symbol = 'V';
