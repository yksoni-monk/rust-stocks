-- Migration: Remove daily_valuation_ratios table and related objects
-- Purpose: Clean up obsolete calculated ratios table in favor of pure SQL views
-- Architecture: Pure SQL view architecture eliminates need for calculated tables

-- Drop related indexes first
DROP INDEX IF EXISTS idx_daily_valuation_ratios_stock_date;
DROP INDEX IF EXISTS idx_daily_valuation_ratios_pe_ratio;
DROP INDEX IF EXISTS idx_daily_valuation_ratios_ps_ratio;
DROP INDEX IF EXISTS idx_daily_valuation_ratios_evs_ratio;
DROP INDEX IF EXISTS idx_daily_valuation_ratios_pb_ratio;
DROP INDEX IF EXISTS idx_daily_valuation_ratios_pcf_ratio;
DROP INDEX IF EXISTS idx_daily_valuation_ratios_ev_ebitda_ratio;
DROP INDEX IF EXISTS idx_daily_valuation_ratios_shareholder_yield;

-- Drop the daily_valuation_ratios table
DROP TABLE IF EXISTS daily_valuation_ratios;

-- Drop any related cache tables
DROP TABLE IF EXISTS sp500_pe_cache;
