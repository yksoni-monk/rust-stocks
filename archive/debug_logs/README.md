# Debug Logs Archive

This directory contains debug log files from the data collection process during development and testing.

## Archived Files

- **debug_collection_A_2025-08-03_2025-09-02.log** - Debug logs for stock A
- **debug_collection_ABBV_2025-08-03_2025-09-02.log** - Debug logs for stock ABBV
- **debug_collection_ACGL_2025-08-03_2025-09-02.log** - Debug logs for stock ACGL
- **debug_collection_ADBE_2025-08-03_2025-09-02.log** - Debug logs for stock ADBE
- **debug_collection_ADP_2025-08-03_2025-09-02.log** - Debug logs for stock ADP
- **debug_collection_AIG_2025-08-03_2025-09-02.log** - Debug logs for stock AIG

## Purpose

These logs were generated during the development and testing of the data collection functionality. They contain detailed information about API calls, data processing, and error handling.

## Date Range

- **Start**: August 3, 2025
- **End**: September 2, 2025

## Current Usage

**NEW**: Starting from the SQLX migration, all new debug log files from the TUI data collection process are automatically created in this directory. The format is:
- `debug_collection_{SYMBOL}_{START_DATE}_{END_DATE}.log`

These files are kept for historical reference and debugging purposes.
