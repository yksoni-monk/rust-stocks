-- Migration: Add pcf_ratio_annual to daily_valuation_ratios
-- Purpose: Add the Price-to-Cash Flow (P/CF) ratio for O'Shaughnessy calculations using Annual data.

-- Column already exists, skip adding it
-- ALTER TABLE daily_valuation_ratios ADD COLUMN pcf_ratio_annual REAL;

-- Add an index for efficient querying of P/CF ratios
CREATE INDEX IF NOT EXISTS idx_daily_ratios_pcf_annual ON daily_valuation_ratios(pcf_ratio_annual);
