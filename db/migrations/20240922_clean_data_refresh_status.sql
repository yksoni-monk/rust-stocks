-- Migration: Clean up data_refresh_status table structure
-- Created: 2024-09-22
-- Purpose: Remove redundant fields and simplify the table structure

-- First, create a backup of the current data
CREATE TABLE data_refresh_status_backup AS SELECT * FROM data_refresh_status;

-- Create the new clean table structure
CREATE TABLE data_refresh_status_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    data_source TEXT NOT NULL UNIQUE,
    latest_data_date DATE,
    last_successful_refresh DATETIME,
    refresh_status TEXT DEFAULT 'unknown',
    records_updated INTEGER DEFAULT 0,
    error_message TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Migrate data from old table to new table
INSERT INTO data_refresh_status_new (
    data_source, 
    latest_data_date, 
    last_successful_refresh, 
    refresh_status, 
    records_updated, 
    error_message,
    created_at,
    updated_at
)
SELECT 
    data_source,
    latest_data_date,
    last_successful_refresh,
    refresh_status,
    records_updated,
    error_message,
    created_at,
    updated_at
FROM data_refresh_status;

-- Drop the old table
DROP TABLE data_refresh_status;

-- Rename the new table
ALTER TABLE data_refresh_status_new RENAME TO data_refresh_status;

-- Recreate the indexes
CREATE INDEX idx_refresh_status_source ON data_refresh_status(data_source);
CREATE INDEX idx_refresh_status_last_successful ON data_refresh_status(last_successful_refresh);
CREATE INDEX idx_refresh_status_status ON data_refresh_status(refresh_status);

-- Update the view to work with the new structure
DROP VIEW IF EXISTS v_data_freshness_summary;

CREATE VIEW v_data_freshness_summary AS
SELECT
    drs.data_source,
    drs.refresh_status,
    drs.latest_data_date,
    drs.last_successful_refresh,
    drs.records_updated,
    drs.error_message,
    rc.max_staleness_days,
    rc.refresh_frequency_hours,
    rc.auto_refresh_enabled,
    rc.refresh_priority,

    -- Calculate staleness in days
    CASE
        WHEN drs.latest_data_date IS NOT NULL THEN
            CAST((JULIANDAY('now') - JULIANDAY(drs.latest_data_date)) AS INTEGER)
        ELSE NULL
    END as staleness_days,

    -- Determine if refresh is needed
    CASE
        WHEN drs.refresh_status = 'error' THEN 'critical'
        WHEN drs.refresh_status = 'missing' THEN 'critical'
        WHEN drs.latest_data_date IS NULL THEN 'critical'
        WHEN CAST((JULIANDAY('now') - JULIANDAY(drs.latest_data_date)) AS INTEGER) > rc.max_staleness_days THEN 'needed'
        WHEN CAST((JULIANDAY('now') - JULIANDAY(drs.latest_data_date)) AS INTEGER) > (rc.max_staleness_days * 0.7) THEN 'recommended'
        ELSE 'current'
    END as refresh_recommendation,

    -- Calculate next recommended refresh time
    CASE
        WHEN drs.last_successful_refresh IS NOT NULL THEN
            DATETIME(drs.last_successful_refresh, '+' || rc.refresh_frequency_hours || ' hours')
        ELSE DATETIME('now')
    END as next_recommended_refresh

FROM data_refresh_status drs
LEFT JOIN refresh_configuration rc ON drs.data_source = rc.data_source
ORDER BY rc.refresh_priority, drs.data_source;
