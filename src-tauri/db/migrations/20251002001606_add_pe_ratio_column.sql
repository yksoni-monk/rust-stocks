-- Migration: Add missing pe_ratio column to daily_valuation_ratios
-- Purpose: Fix P/E ratio calculation failures in data refresh
-- Issue: data_refresh_orchestrator.rs tries to insert pe_ratio but column doesn't exist

-- Add the missing pe_ratio column
ALTER TABLE daily_valuation_ratios ADD COLUMN pe_ratio REAL;

-- Add index for better performance on P/E ratio queries
CREATE INDEX IF NOT EXISTS idx_daily_valuation_ratios_pe_ratio ON daily_valuation_ratios(pe_ratio);

-- Update existing records with P/E ratio calculation where possible
UPDATE daily_valuation_ratios 
SET pe_ratio = CASE
    WHEN price > 0 AND market_cap > 0 AND revenue_annual > 0 
    THEN price / (revenue_annual / (market_cap / price))
    ELSE NULL
END
WHERE pe_ratio IS NULL;
