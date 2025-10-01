-- Add P/B ratio column to daily_valuation_ratios table for O'Shaughnessy screening
-- Migration: 20250928_add_pb_ratio_column.sql

-- Add P/B ratio column
ALTER TABLE daily_valuation_ratios ADD COLUMN pb_ratio_ttm REAL;

-- Add index for P/B ratio queries
CREATE INDEX IF NOT EXISTS idx_daily_ratios_pb_ttm 
ON daily_valuation_ratios(pb_ratio_ttm);

-- Update the multi-period index to include P/B ratio
DROP INDEX IF EXISTS idx_daily_ratios_multi_period;
CREATE INDEX IF NOT EXISTS idx_daily_ratios_multi_period 
ON daily_valuation_ratios(stock_id, date, ps_ratio_ttm, evs_ratio_ttm, pb_ratio_ttm);
