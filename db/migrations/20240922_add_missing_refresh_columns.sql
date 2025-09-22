-- Migration: Add missing refresh tracking columns
-- Created: 2024-09-22
-- Purpose: Add back columns that were accidentally removed but are still needed by refresh orchestrator

-- Add the missing columns that the refresh orchestrator needs
ALTER TABLE data_refresh_status ADD COLUMN last_refresh_start DATETIME;
ALTER TABLE data_refresh_status ADD COLUMN last_refresh_complete DATETIME;

-- Update the view to include these columns if needed
-- (The view should still work as these are not part of the SELECT)
