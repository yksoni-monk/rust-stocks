-- Migration: Remove company_metadata table
-- Risk: ZERO RISK (replaced by v_price_data_coverage view)
-- Impact: HIGH IMPACT (eliminates redundant computed data)

-- Drop the company_metadata table since it's replaced by v_price_data_coverage view
DROP TABLE IF EXISTS company_metadata;
