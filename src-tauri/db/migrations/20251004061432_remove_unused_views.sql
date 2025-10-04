-- Migration: Remove unused views
-- Risk: ZERO RISK (views have 0 references in application code)
-- Impact: Reduces database clutter

-- Drop unused views that have no references in application code
DROP VIEW IF EXISTS v_active_refresh_operations;
DROP VIEW IF EXISTS v_data_freshness_summary;
DROP VIEW IF EXISTS v_price_data_coverage;
DROP VIEW IF EXISTS v_market_cap_calculated;