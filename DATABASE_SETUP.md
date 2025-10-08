# Database Setup Guide

Complete guide for setting up a fresh database from scratch.

## Prerequisites

1. **Rust & Cargo** installed
2. **SQLx CLI** installed: `cargo install sqlx-cli`
3. **SQLite3** installed (for verification)
4. **Environment variables** set in `.env` file at project root

## Environment Setup

Create `.env` file at `/Users/yksoni/code/misc/rust-stocks/.env`:

```bash
# Project structure
PROJECT_ROOT=/Users/yksoni/code/misc/rust-stocks

# RUST root folder
RUST_ROOT=/Users/yksoni/code/misc/rust-stocks/src-tauri

# Data paths
DATABASE_PATH=db/stocks.db

# SQLx Database URL (required for sqlx compile-time verification)
DATABASE_URL=sqlite:db/stocks.db

# Migrations folder
MIGRATION_PATH=db/migrations
```

## Complete Setup Process

### Step 1: Set Environment Variables

```bash
export PROJECT_ROOT=/Users/yksoni/code/misc/rust-stocks
cd $PROJECT_ROOT/src-tauri
```

### Step 2: Delete Existing Database (If Starting Fresh)

If you have an old database file, delete it:

```bash
rm db/stocks.db
```

**Note:** This step is only needed if you're starting completely fresh. Skip if this is your first setup.

### Step 3: Verify Migration Files Exist

Check that migration files are present:

```bash
ls -la db/migrations/
```

You should see:
- `20251008212012_initial_schema.up.sql` - Creates all tables, views, indexes
- `20251008212012_initial_schema.down.sql` - Drops all schema objects

### Step 4: Create Empty Database File

Create the database file that migrations will populate:

```bash
sqlite3 db/stocks.db "SELECT 1;"
```

**Why this step is needed:** SQLx migrations require the database file to exist before they can run. This command creates an empty SQLite database.

**Verify database created:**
```bash
ls -lh db/stocks.db
# Should show: a small file (~8-12 KB)
```

### Step 5: Apply Initial Migration

```bash
cargo run --bin migrate -- run
```

**Expected output:**
```
✅ Loaded .env from: /path/to/.env
✅ Current directory: /path/to/src-tauri
✅ DATABASE_URL: sqlite:/path/to/db/stocks.db
✅ MIGRATION_PATH: db/migrations

🚀 Applying pending migrations...

Applied 20251008212012/migrate initial schema

✅ Migrations applied successfully
```

**Verify schema created:**
```bash
sqlite3 db/stocks.db ".tables"
```

You should see all tables created (stocks, daily_prices, income_statements, etc.)

### Step 6: Initialize S&P 500 Stock List

Download and populate the S&P 500 company list:

```bash
cargo run --bin init_sp500
```

**Expected output:**
```
🔄 Initializing S&P 500 Stock List
══════════════════════════════════

📥 Downloading S&P 500 list from GitHub...
📄 Parsing CSV data...
   ✅ Found 503 companies
💾 Inserting stocks into database...
   ✅ Inserted: 503, Updated: 0

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✅ Successfully initialized 503 S&P 500 companies!
📅 Last updated: 2025-10-08

💡 Next steps:
   1. Run: cargo run --bin fetch_ciks
   2. Run: cargo run --bin refresh_data -- all
```

**Verify stocks populated:**
```bash
sqlite3 db/stocks.db "SELECT COUNT(*) FROM stocks;"
# Should show: 503
```

### Step 7: Fetch CIK Numbers from SEC

Fetch Central Index Keys (CIKs) for all S&P 500 companies:

```bash
cargo run --bin fetch_ciks
```

This fetches CIK numbers from the SEC EDGAR API for each company. CIKs are required to download financial data.

**Expected output:**
```
🔍 CIK Fetcher for S&P 500 Stocks
==================================
📊 Found 503 S&P 500 stocks to process

🔍 Fetching CIKs from SEC EDGAR API...
[Progress messages for each stock...]

✅ CIK fetch complete! Successfully fetched 503 CIKs
```

**Verify CIKs populated:**
```bash
sqlite3 db/stocks.db "SELECT COUNT(*) FROM stocks WHERE cik IS NOT NULL;"
# Should show: ~500+ (most stocks should have CIKs)
```

### Step 8: Download All Financial Data

Download market data and financial statements for all S&P 500 companies:

```bash
cargo run --bin refresh_data -- all
```

This will:
1. **Download market data** (prices, market cap, P/E ratios) from Schwab API
2. **Download financial statements** (income statements, balance sheets, cash flows) from SEC EDGAR
3. **Calculate ratios** for screening algorithms (Piotroski F-Score, O'Shaughnessy Value)

**Note:** This process can take 30-60 minutes depending on network speed and API rate limits.

**Expected output:**
```
🔄 Stock Data Refresh
📅 2025-10-08
══════════════════════════════════

🚀 Starting data refresh in All mode...
🔄 Step 1/4: Update market data
💰 Refreshing market data from Schwab...
📊 Found 503 S&P 500 stocks to update
[Progress messages...]

🔄 Step 2/4: Extract EDGAR financial data
📈 Refreshing EDGAR financial data...
[Progress messages...]

✅ All screening features should now be ready with current data!
```

**Verify data populated:**
```bash
# Check market data
sqlite3 db/stocks.db "SELECT COUNT(*) FROM daily_prices;"

# Check financial statements
sqlite3 db/stocks.db "SELECT COUNT(*) FROM income_statements;"
sqlite3 db/stocks.db "SELECT COUNT(*) FROM balance_sheets;"
sqlite3 db/stocks.db "SELECT COUNT(*) FROM cash_flow_statements;"
```

### Step 9: Verify Setup Complete

Run status check:

```bash
cargo run --bin migrate -- status
```

Check database size (should be ~2-3 GB when fully populated):
```bash
ls -lh db/stocks.db
```

## Database Management Commands

### View Migration Status

```bash
cargo run --bin migrate -- status
```

### Create New Migration

```bash
cargo run --bin migrate -- create add_new_feature
```

This creates:
- `db/migrations/TIMESTAMP_add_new_feature.up.sql`
- `db/migrations/TIMESTAMP_add_new_feature.down.sql`

### Apply Pending Migrations

```bash
cargo run --bin migrate -- run
```

### Revert Last Migration

```bash
cargo run --bin migrate -- revert
```

### Show Migration Info

```bash
cargo run --bin migrate -- info
```

## Troubleshooting

### "No S&P 500 stocks found"

**Problem:** `refresh_data` reports 0 stocks to process.

**Solution:** Run Step 6 again:
```bash
cargo run --bin init_sp500
```

### "CIK not found" errors

**Problem:** Some stocks don't have CIKs from SEC.

**Solution:** This is normal - not all companies have publicly available CIKs. The system will skip these stocks.

### "Database locked" errors

**Problem:** Another process is accessing the database.

**Solution:**
1. Close any SQLite browser tools
2. Stop the Tauri dev server if running
3. Try the command again

### "Failed to apply migration"

**Problem:** Migration file has SQL syntax errors.

**Solution:**
1. Check the migration file for syntax errors
2. Test SQL manually: `sqlite3 db/stocks.db < db/migrations/FILE.up.sql`
3. Fix errors and try again

### Migration files missing

**Problem:** After running `migrate_fresh`, migration files are in the wrong location.

**Solution:** Migration files should be in `src-tauri/db/migrations/`, not `src-tauri/migrations/`.

## Starting Fresh (Reset Everything)

If you need to completely reset the database:

### Option 1: Using Migrations (Recommended)

```bash
# Revert all migrations
cargo run --bin migrate -- revert

# Re-apply migrations
cargo run --bin migrate -- run

# Re-populate data (Steps 6-8)
cargo run --bin init_sp500
cargo run --bin fetch_ciks
cargo run --bin refresh_data -- all
```

### Option 2: Manual Reset

```bash
# Delete database
rm db/stocks.db

# Create empty database
sqlite3 db/stocks.db "SELECT 1;"

# Re-run all setup steps (Steps 5-8)
```

### Option 3: Use migrate_fresh Tool (Nuclear Option)

**WARNING:** This deletes ALL data and creates fresh migration history.

```bash
cargo run --bin migrate_fresh
```

This will:
1. Backup current database to `db/stocks.db.backup`
2. Backup migrations to `db/migrations_backup/`
3. Extract schema and create single initial migration
4. Create fresh empty database

## Quick Reference

```bash
# Full setup from scratch
export PROJECT_ROOT=/path/to/rust-stocks
cd $PROJECT_ROOT/src-tauri
rm -f db/stocks.db                          # Delete old database (if exists)
sqlite3 db/stocks.db "SELECT 1;"            # Create empty database
cargo run --bin migrate -- run              # Apply schema
cargo run --bin init_sp500                  # Populate stocks
cargo run --bin fetch_ciks                  # Fetch CIKs
cargo run --bin refresh_data -- all         # Download all data

# Check status
cargo run --bin migrate -- status           # Migration status
sqlite3 db/stocks.db "SELECT COUNT(*) FROM stocks;"
sqlite3 db/stocks.db "SELECT COUNT(*) FROM daily_prices;"

# Maintenance
cargo run --bin refresh_data -- market      # Update market data only
cargo run --bin refresh_data -- financials  # Update financial data only
```

## Architecture Notes

- **Database**: SQLite (production database ~2.5 GB)
- **Migrations**: Managed by SQLx with reversible .up.sql/.down.sql files
- **Data Sources**:
  - S&P 500 list: GitHub (datasets/s-and-p-500-companies)
  - CIKs: SEC EDGAR API
  - Market data: Schwab API
  - Financial statements: SEC EDGAR API
- **Location**: All database files in `src-tauri/db/`

## Next Steps

After completing the setup:

1. **Run the desktop app**: `npm run tauri dev` (from project root)
2. **Test screening features**:
   - Piotroski F-Score screening
   - O'Shaughnessy Value composite screening
3. **Keep data fresh**: Run `refresh_data` weekly/monthly

---

**Last Updated:** 2025-10-08
