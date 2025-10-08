-- Revert: Replace view with original sp500_symbols table

-- Drop the view
DROP VIEW IF EXISTS sp500_symbols;

-- Recreate the original table
CREATE TABLE sp500_symbols (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL UNIQUE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Recreate indexes
CREATE INDEX idx_sp500_symbols_symbol ON sp500_symbols(symbol);
CREATE INDEX idx_sp500_symbol ON sp500_symbols(symbol);

-- Repopulate with current S&P 500 stocks
INSERT INTO sp500_symbols (id, symbol, created_at)
SELECT id, symbol, created_at
FROM stocks
WHERE is_sp500 = 1;
