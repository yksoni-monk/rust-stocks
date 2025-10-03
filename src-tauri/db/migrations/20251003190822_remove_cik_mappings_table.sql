-- Migration: Remove cik_mappings table
-- Purpose: Consolidate CIK mappings to use only cik_mappings_sp500 table
-- Date: 2025-10-03
-- Reason: cik_mappings table is redundant and only used by one utility script

-- Drop indexes first
DROP INDEX IF EXISTS idx_cik_mappings_cik;
DROP INDEX IF EXISTS idx_cik_mappings_stock_id;
DROP INDEX IF EXISTS idx_cik_mappings_symbol;

-- Drop the table
DROP TABLE IF EXISTS cik_mappings;
