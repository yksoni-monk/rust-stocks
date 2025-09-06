-- Add missing fundamental data columns to daily_prices table
-- This migration adds all the fundamental fields from Schwab API that we're currently missing

-- Add EPS (Earnings Per Share)
ALTER TABLE daily_prices ADD COLUMN eps REAL;

-- Add Beta (volatility measure)
ALTER TABLE daily_prices ADD COLUMN beta REAL;

-- Add 52-week high/low
ALTER TABLE daily_prices ADD COLUMN week_52_high REAL;
ALTER TABLE daily_prices ADD COLUMN week_52_low REAL;

-- Add Price ratios
ALTER TABLE daily_prices ADD COLUMN pb_ratio REAL;  -- Price-to-Book
ALTER TABLE daily_prices ADD COLUMN ps_ratio REAL;  -- Price-to-Sales

-- Add Shares information
ALTER TABLE daily_prices ADD COLUMN shares_outstanding REAL;

-- Add Margin and Return metrics
ALTER TABLE daily_prices ADD COLUMN profit_margin REAL;
ALTER TABLE daily_prices ADD COLUMN operating_margin REAL;
ALTER TABLE daily_prices ADD COLUMN return_on_equity REAL;
ALTER TABLE daily_prices ADD COLUMN return_on_assets REAL;

-- Add Debt ratios
ALTER TABLE daily_prices ADD COLUMN debt_to_equity REAL;

-- Add Dividend per share (different from dividend yield)
ALTER TABLE daily_prices ADD COLUMN dividend_per_share REAL;

-- Add indexes for commonly queried fundamental fields
CREATE INDEX idx_daily_prices_pe_ratio ON daily_prices(pe_ratio);
CREATE INDEX idx_daily_prices_market_cap ON daily_prices(market_cap);
CREATE INDEX idx_daily_prices_dividend_yield ON daily_prices(dividend_yield);
CREATE INDEX idx_daily_prices_eps ON daily_prices(eps);
CREATE INDEX idx_daily_prices_beta ON daily_prices(beta);
