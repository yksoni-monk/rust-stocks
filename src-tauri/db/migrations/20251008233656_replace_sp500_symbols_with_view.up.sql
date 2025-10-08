-- Replace sp500_symbols table with a view based on stocks.is_sp500

-- Drop the redundant sp500_symbols table
DROP TABLE IF EXISTS sp500_symbols;

-- Create a view that provides the same interface
CREATE VIEW sp500_symbols AS
SELECT
    id,
    symbol,
    created_at
FROM stocks
WHERE is_sp500 = 1;
