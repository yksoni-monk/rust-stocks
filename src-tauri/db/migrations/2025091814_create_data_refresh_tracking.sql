-- Migration: Create data refresh tracking tables
-- Created: 2024-09-18
-- Purpose: Track data refresh operations and freshness status for unified refresh system

-- Table to track the status and history of data refresh operations
CREATE TABLE IF NOT EXISTS data_refresh_status (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    data_source TEXT NOT NULL UNIQUE,           -- 'daily_prices', 'pe_ratios', 'financial_statements', etc.
    last_refresh_start DATETIME,
    last_refresh_complete DATETIME,
    last_successful_refresh DATETIME,
    next_recommended_refresh DATETIME,
    refresh_status TEXT DEFAULT 'unknown',      -- 'current', 'stale', 'refreshing', 'error', 'missing'
    records_updated INTEGER DEFAULT 0,
    latest_data_date DATE,
    error_message TEXT,
    refresh_duration_seconds INTEGER DEFAULT 0,
    refresh_type TEXT,                          -- 'incremental', 'full', 'backfill'
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Table to track individual refresh sessions and their progress
CREATE TABLE IF NOT EXISTS refresh_progress (
    session_id TEXT PRIMARY KEY,
    operation_type TEXT NOT NULL,               -- 'quick', 'standard', 'full'
    start_time DATETIME DEFAULT CURRENT_TIMESTAMP,
    end_time DATETIME,
    total_steps INTEGER NOT NULL,
    completed_steps INTEGER DEFAULT 0,
    current_step_name TEXT,
    current_step_progress REAL DEFAULT 0.0,    -- 0.0 to 100.0
    estimated_completion DATETIME,
    status TEXT DEFAULT 'running',              -- 'running', 'completed', 'error', 'cancelled'
    error_details TEXT,
    initiated_by TEXT,                          -- 'user', 'scheduled', 'auto'
    data_sources_refreshed TEXT,                -- JSON array of data sources included
    total_records_processed INTEGER DEFAULT 0,
    performance_metrics TEXT                    -- JSON object with performance data
);

-- Table to store refresh configuration and scheduling
CREATE TABLE IF NOT EXISTS refresh_configuration (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    data_source TEXT NOT NULL UNIQUE,
    refresh_frequency_hours INTEGER DEFAULT 24, -- How often to refresh (in hours)
    max_staleness_days INTEGER DEFAULT 7,       -- Maximum acceptable staleness
    auto_refresh_enabled BOOLEAN DEFAULT TRUE,
    refresh_priority INTEGER DEFAULT 5,         -- 1-10 priority for ordering
    dependency_sources TEXT,                     -- JSON array of required predecessor sources
    refresh_command TEXT,                        -- Command/function to execute refresh
    estimated_duration_minutes INTEGER DEFAULT 10,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_refresh_status_source ON data_refresh_status(data_source);
CREATE INDEX IF NOT EXISTS idx_refresh_status_last_successful ON data_refresh_status(last_successful_refresh);
CREATE INDEX IF NOT EXISTS idx_refresh_status_status ON data_refresh_status(refresh_status);
CREATE INDEX IF NOT EXISTS idx_refresh_progress_status ON refresh_progress(status);
CREATE INDEX IF NOT EXISTS idx_refresh_progress_start_time ON refresh_progress(start_time);
CREATE INDEX IF NOT EXISTS idx_refresh_config_source ON refresh_configuration(data_source);
CREATE INDEX IF NOT EXISTS idx_refresh_config_priority ON refresh_configuration(refresh_priority);

-- Insert default configuration for known data sources (skip if already exists)
INSERT OR IGNORE INTO refresh_configuration (data_source, refresh_frequency_hours, max_staleness_days, refresh_priority, estimated_duration_minutes, refresh_command) VALUES
('daily_prices', 24, 7, 1, 15, 'import-schwab-prices'),
('pe_ratios', 24, 2, 2, 25, 'run_pe_calculation'),
('ps_evs_ratios', 168, 14, 3, 8, 'calculate-ratios'),
('financial_statements', 720, 120, 4, 45, 'edgar-api-client'),
('company_metadata', 8760, 365, 5, 2, 'update-company-metadata');

-- Insert initial status records for tracking (skip if already exists)
INSERT OR IGNORE INTO data_refresh_status (data_source, refresh_status) VALUES
('daily_prices', 'unknown'),
('pe_ratios', 'unknown'),
('ps_evs_ratios', 'unknown'),
('financial_statements', 'unknown'),
('company_metadata', 'unknown');

-- Create view for easy monitoring of refresh status
CREATE VIEW IF NOT EXISTS v_data_freshness_summary AS
SELECT
    drs.data_source,
    drs.refresh_status,
    drs.latest_data_date,
    drs.last_successful_refresh,
    drs.records_updated,
    drs.refresh_duration_seconds,
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

-- Create view for active refresh operations
CREATE VIEW IF NOT EXISTS v_active_refresh_operations AS
SELECT
    session_id,
    operation_type,
    start_time,
    total_steps,
    completed_steps,
    current_step_name,
    current_step_progress,
    ROUND((CAST(completed_steps AS REAL) / CAST(total_steps AS REAL)) * 100, 1) as overall_progress_percent,
    estimated_completion,
    status,
    initiated_by,
    CAST((JULIANDAY('now') - JULIANDAY(start_time)) * 24 * 60 AS INTEGER) as elapsed_minutes
FROM refresh_progress
WHERE status IN ('running', 'error')
ORDER BY start_time DESC;