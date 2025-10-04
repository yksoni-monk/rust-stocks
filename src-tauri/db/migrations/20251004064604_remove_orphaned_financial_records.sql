-- Migration: Remove orphaned financial records (Clean Slate Approach)
-- Risk: HIGH RISK (permanent data deletion)
-- Impact: Removes 163,711 orphaned financial records (98.33% of total)
-- Benefit: Clean database, 100% SEC metadata coverage, 95% size reduction (5.3GB → 200MB)

-- Before cleanup stats:
-- Total financial records: 166,481
-- Records WITH SEC metadata: 2,776 (1.67%)
-- Records WITHOUT SEC metadata: 163,711 (98.33%)

-- Remove orphaned income statements (62,265 records)
DELETE FROM income_statements 
WHERE sec_filing_id IS NULL;

-- Remove orphaned balance sheets (61,550 records)  
DELETE FROM balance_sheets 
WHERE sec_filing_id IS NULL;

-- Remove orphaned cash flow statements (53,513 records)
DELETE FROM cash_flow_statements 
WHERE sec_filing_id IS NULL;

-- Clean up orphaned SEC filings (if any exist without linked records)
DELETE FROM sec_filings 
WHERE id NOT IN (
    SELECT DISTINCT sec_filing_id 
    FROM income_statements 
    WHERE sec_filing_id IS NOT NULL
    UNION
    SELECT DISTINCT sec_filing_id 
    FROM balance_sheets 
    WHERE sec_filing_id IS NOT NULL
    UNION
    SELECT DISTINCT sec_filing_id 
    FROM cash_flow_statements 
    WHERE sec_filing_id IS NOT NULL
);

-- Clean up orphaned data imports (if any exist without linked records)
DELETE FROM data_imports 
WHERE id NOT IN (
    SELECT DISTINCT import_id 
    FROM income_statements 
    WHERE import_id IS NOT NULL
    UNION
    SELECT DISTINCT import_id 
    FROM balance_sheets 
    WHERE import_id IS NOT NULL
    UNION
    SELECT DISTINCT import_id 
    FROM cash_flow_statements 
    WHERE import_id IS NOT NULL
);

-- After cleanup expected stats:
-- Total financial records: ~2,776 (100% with SEC metadata)
-- Database size: ~200MB (95% reduction)
-- SEC metadata coverage: 100%