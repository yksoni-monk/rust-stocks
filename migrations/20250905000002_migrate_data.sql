-- Data migration from original tables to enhanced tables
-- Migration: 20250905000002_migrate_data.sql
-- Migrates existing data while preserving all information

-- Enable foreign keys
PRAGMA foreign_keys = ON;

-- Migrate stocks data from original table to enhanced table
INSERT OR IGNORE INTO stocks_enhanced (
    id,
    symbol,
    company_name,
    sector,
    industry,
    market_cap,
    created_at,
    updated_at
)
SELECT 
    id,
    symbol,
    company_name,
    sector,
    industry,
    market_cap,
    created_at,
    COALESCE(last_updated, created_at) as updated_at
FROM stocks;

-- Migrate daily prices data from original table to enhanced table
INSERT OR IGNORE INTO daily_prices_enhanced (
    id,
    stock_id,
    date,
    open_price,
    high_price,
    low_price,
    close_price,
    volume,
    pe_ratio,
    dividend_yield,
    created_at
)
SELECT 
    id,
    stock_id,
    date,
    open_price,
    high_price,
    low_price,
    close_price,
    volume,
    pe_ratio,
    dividend_yield,
    created_at
FROM daily_prices;

-- Update metadata to track migration
INSERT OR REPLACE INTO metadata (key, value) VALUES 
    ('data_migration_completed', CURRENT_TIMESTAMP),
    ('stocks_migrated', (SELECT COUNT(*) FROM stocks_enhanced)),
    ('prices_migrated', (SELECT COUNT(*) FROM daily_prices_enhanced)),
    ('migration_20250905000002', 'completed');