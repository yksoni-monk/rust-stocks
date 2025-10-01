-- Migration: Add shareholder_yield_annual to daily_valuation_ratios
-- Purpose: Add the Shareholder Yield for O'Shaughnessy calculations using Annual data.

ALTER TABLE daily_valuation_ratios ADD COLUMN shareholder_yield_annual REAL;

-- Add an index for efficient querying of Shareholder Yield
CREATE INDEX IF NOT EXISTS idx_daily_ratios_shareholder_yield_annual ON daily_valuation_ratios(shareholder_yield_annual);
