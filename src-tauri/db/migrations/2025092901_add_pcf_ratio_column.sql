-- Migration: Add pcf_ratio_ttm to daily_valuation_ratios
-- Purpose: Add the Price-to-Cash Flow (P/CF) ratio for O'Shaughnessy calculations.

ALTER TABLE daily_valuation_ratios ADD COLUMN pcf_ratio_ttm REAL;

-- Add an index for efficient querying of P/CF ratios
CREATE INDEX IF NOT EXISTS idx_daily_ratios_pcf_ttm ON daily_valuation_ratios(pcf_ratio_ttm);
