-- Create dedicated CIK mappings table for S&P 500 companies
-- Single source of truth for EDGAR data extraction

CREATE TABLE IF NOT EXISTS cik_mappings_sp500 (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    cik TEXT NOT NULL UNIQUE,
    stock_id INTEGER NOT NULL,
    symbol TEXT NOT NULL,
    company_name TEXT NOT NULL,
    edgar_file_path TEXT NOT NULL,
    file_exists BOOLEAN DEFAULT 1,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (stock_id) REFERENCES stocks(id)
);

-- Create indices for performance
CREATE INDEX IF NOT EXISTS idx_cik_mappings_sp500_cik ON cik_mappings_sp500(cik);
CREATE INDEX IF NOT EXISTS idx_cik_mappings_sp500_stock_id ON cik_mappings_sp500(stock_id);
CREATE INDEX IF NOT EXISTS idx_cik_mappings_sp500_symbol ON cik_mappings_sp500(symbol);
CREATE INDEX IF NOT EXISTS idx_cik_mappings_sp500_file_exists ON cik_mappings_sp500(file_exists);