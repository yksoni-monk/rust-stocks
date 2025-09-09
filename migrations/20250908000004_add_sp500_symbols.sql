-- Add S&P 500 symbols table for offline support
-- This migration is ADDITIVE only - no data destruction

CREATE TABLE IF NOT EXISTS sp500_symbols (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL UNIQUE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_sp500_symbols_symbol ON sp500_symbols(symbol);

-- Add metadata entry to track S&P 500 updates
INSERT OR IGNORE INTO metadata (key, value) 
VALUES ('sp500_table_created', datetime('now'));