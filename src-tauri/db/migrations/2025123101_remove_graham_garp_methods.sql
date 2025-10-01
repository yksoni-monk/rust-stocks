-- Migration: Remove Graham Value and GARP P/E Screening Methods
-- Created: 2025-12-31
-- Purpose: Clean up database objects for removed screening methods
-- Safety: DROP operations only - no data preservation needed

-- ============================================================================
-- DROP GRAHAM VALUE SCREENING OBJECTS
-- ============================================================================

-- Drop Graham screening views
DROP VIEW IF EXISTS v_graham_screening_stats;
DROP VIEW IF EXISTS v_latest_graham_screening;

-- Drop Graham screening tables
DROP TABLE IF EXISTS graham_screening_presets;
DROP TABLE IF EXISTS graham_screening_results;

-- Drop Graham screening indexes (automatically dropped with tables)
-- Note: Indexes are automatically dropped when tables are dropped

-- ============================================================================
-- DROP GARP P/E SCREENING OBJECTS
-- ============================================================================

-- Drop GARP P/E screening views
DROP VIEW IF EXISTS garp_pe_screening_data;

-- Drop GARP P/E specific indexes
DROP INDEX IF EXISTS idx_income_statements_eps_growth;
DROP INDEX IF EXISTS idx_daily_ratios_pe_garp;
DROP INDEX IF EXISTS idx_balance_sheets_debt_equity;

-- Note: peg_ratio_analysis view is not dropped as it might be used by other methods
-- If it's GARP-specific, it will be identified and dropped separately

-- ============================================================================
-- VERIFICATION QUERIES
-- ============================================================================

-- Verify Graham objects are removed
SELECT 'Graham objects removed' as status
WHERE NOT EXISTS (
    SELECT 1 FROM sqlite_master 
    WHERE type IN ('table', 'view') 
    AND name LIKE '%graham%'
);

-- Verify GARP objects are removed (except peg_ratio_analysis which might be shared)
SELECT 'GARP objects removed' as status
WHERE NOT EXISTS (
    SELECT 1 FROM sqlite_master 
    WHERE type IN ('table', 'view') 
    AND name LIKE '%garp%'
    AND name != 'peg_ratio_analysis'
);

-- Show remaining screening-related objects
SELECT 
    type,
    name,
    sql
FROM sqlite_master 
WHERE type IN ('table', 'view') 
AND (
    name LIKE '%piotroski%' OR 
    name LIKE '%oshaughnessy%' OR
    name LIKE '%screening%' OR
    name LIKE '%ratio%'
)
ORDER BY type, name;
