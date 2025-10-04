-- Migration: Remove cik_mappings_sp500 table
-- Risk: ZERO RISK (data is duplicated in stocks table)
-- Impact: Eliminates duplicate CIK data, simplifies architecture

-- Drop the cik_mappings_sp500 table since CIK data is now in stocks table
-- All CIKs from cik_mappings_sp500 are already in stocks.cik column
DROP TABLE IF EXISTS cik_mappings_sp500;