-- Migration: Add share_repurchases to balance_sheets
-- Purpose: Add share repurchases data for Shareholder Yield calculations.

ALTER TABLE balance_sheets ADD COLUMN share_repurchases REAL;

-- Add an index for efficient querying of share repurchases
CREATE INDEX IF NOT EXISTS idx_balance_sheets_share_repurchases ON balance_sheets(share_repurchases);
