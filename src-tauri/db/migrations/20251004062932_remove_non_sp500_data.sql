-- Migration: Remove all non-S&P 500 data
-- Risk: HIGH RISK (permanent data deletion)
-- Impact: Removes ~85% of database data, keeping only S&P 500 companies
-- Benefit: Focused database, 100% SEC metadata coverage, improved performance

-- Remove non-S&P 500 price data (5.6M records)
DELETE FROM daily_prices 
WHERE stock_id IN (
    SELECT id FROM stocks 
    WHERE is_sp500 = 0 OR is_sp500 IS NULL
);

-- Remove non-S&P 500 financial data (3.9K records)
DELETE FROM income_statements 
WHERE stock_id IN (
    SELECT id FROM stocks 
    WHERE is_sp500 = 0 OR is_sp500 IS NULL
);

DELETE FROM balance_sheets 
WHERE stock_id IN (
    SELECT id FROM stocks 
    WHERE is_sp500 = 0 OR is_sp500 IS NULL
);

DELETE FROM cash_flow_statements 
WHERE stock_id IN (
    SELECT id FROM stocks 
    WHERE is_sp500 = 0 OR is_sp500 IS NULL
);

-- Remove non-S&P 500 SEC filings
DELETE FROM sec_filings 
WHERE stock_id IN (
    SELECT id FROM stocks 
    WHERE is_sp500 = 0 OR is_sp500 IS NULL
);

-- Remove non-S&P 500 data imports
DELETE FROM data_imports 
WHERE id IN (
    SELECT di.id FROM data_imports di
    JOIN income_statements i ON di.id = i.import_id
    WHERE i.stock_id IN (
        SELECT id FROM stocks 
        WHERE is_sp500 = 0 OR is_sp500 IS NULL
    )
);

-- Remove non-S&P 500 stocks (5.4K records)
DELETE FROM stocks 
WHERE is_sp500 = 0 OR is_sp500 IS NULL;

-- Update remaining stocks to ensure is_sp500 = 1
UPDATE stocks SET is_sp500 = 1 WHERE is_sp500 IS NULL;