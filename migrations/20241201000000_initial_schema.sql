-- Initial database schema for Rust Stocks TUI
-- Migration: 20241201000000_initial_schema.sql

-- Enable foreign keys
PRAGMA foreign_keys = ON;

-- Stocks table
CREATE TABLE IF NOT EXISTS stocks (
    id INTEGER PRIMARY KEY,
    symbol TEXT UNIQUE NOT NULL,
    company_name TEXT NOT NULL,
    sector TEXT,
    industry TEXT,
    market_cap REAL,
    status TEXT DEFAULT 'active',
    first_trading_date DATE,
    last_updated DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Daily prices table
CREATE TABLE IF NOT EXISTS daily_prices (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER NOT NULL,
    date DATE NOT NULL,
    open_price REAL NOT NULL,
    high_price REAL NOT NULL,
    low_price REAL NOT NULL,
    close_price REAL NOT NULL,
    volume INTEGER,
    pe_ratio REAL,
    market_cap REAL,
    dividend_yield REAL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, date)
);

-- Metadata table for application settings
CREATE TABLE IF NOT EXISTS metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_stocks_symbol ON stocks(symbol);
CREATE INDEX IF NOT EXISTS idx_stocks_company_name ON stocks(company_name);
CREATE INDEX IF NOT EXISTS idx_daily_prices_stock_date ON daily_prices(stock_id, date);
CREATE INDEX IF NOT EXISTS idx_daily_prices_date ON daily_prices(date);

-- Insert default metadata
INSERT OR IGNORE INTO metadata (key, value) VALUES 
    ('schema_version', '1.0'),
    ('last_updated', CURRENT_TIMESTAMP),
    ('total_stocks', '0'),
    ('total_prices', '0');
