-- Remove Company Facts API freshness tracking table
-- This table was used for individual Company Facts API freshness checking

DROP INDEX IF EXISTS idx_sec_filing_tracking_cik;
DROP INDEX IF EXISTS idx_sec_filing_tracking_last_checked;  
DROP TABLE IF EXISTS sec_filing_tracking;