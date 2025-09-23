# Database Migration Strategy & Standards

## Overview
This document defines our database migration strategy, naming conventions, and safety procedures for the rust-stocks project.

## Migration Naming Convention

### Standard Format
```
yyyymmdd##_descriptive_name.sql
```

**Components:**
- `yyyy` - 4-digit year
- `mm` - 2-digit month
- `dd` - 2-digit day
- `##` - 2-digit counter for same-day migrations
- `descriptive_name` - Short, clear description using snake_case

### Examples
```
2025090501_initial_schema.sql
2025090502_enhanced_schema.sql
2025091810_create_cik_mappings.sql
2025092228_create_improved_piotroski_views.sql
```

### Counter Rules
- Start at `01` for first migration of the day
- Increment by 1 for each additional migration same day: `01`, `02`, `03`, etc.
- Allows up to 99 migrations per day
- Ensures chronological ordering when sorted alphabetically

## Migration Directory Structure

```
src-tauri/db/
├── stocks.db                  # Production database (2.5GB)
├── migrations/                # All SQL migration files
│   ├── 2025090501_initial_schema.sql
│   ├── 2025090502_enhanced_schema.sql
│   └── ...
└── backups/                   # Automatic backups
    ├── migrations_backup_YYYYMMDD_HHMMSS/
    └── stocks_backup_YYYYMMDD_HHMMSS.db
```

## Migration Safety Strategy

### 1. Automatic Backups
- **Before any migration**: Automatic backup created
- **Location**: `src-tauri/db/` (root of db directory)
- **Naming**: `stocks.db.backup.YYYYMMDD_HHMMSS`
- **Verification**: File size and integrity checks
- **Auto-cleanup**: Keeps only the 2 most recent backups to save disk space
- **Cleanup reporting**: Shows freed disk space when old backups are removed

### 2. Production Database Protection
**Database Stats Monitoring:**
- Stocks: 5,892
- Daily prices: 6.5M+ records
- TTM financials: 142K+ records
- Size: 2.5GB

**Safety Checks:**
- Data loss detection (>50% record reduction triggers alert)
- Size verification after migrations
- Automatic rollback capability

### 3. Migration Workflow

```bash
# 1. Create migration file with proper naming
touch src-tauri/db/migrations/$(date +%Y%m%d)01_description.sql

# 2. Write migration SQL
# - Include DROP IF EXISTS for views/tables being recreated
# - Use transactions where possible
# - Add descriptive comments

# 3. Test migration (uses safety system)
cargo run --bin db_admin migrate

# 4. Verify results
sqlite3 src-tauri/db/stocks.db "SELECT COUNT(*) FROM stocks;"
```

## Migration Best Practices

### 1. File Content Standards
```sql
-- Migration: Description of what this migration does
-- Created: YYYY-MM-DD
-- Purpose: Detailed explanation of why this change is needed

-- Drop existing objects if recreating
DROP VIEW IF EXISTS existing_view;
DROP TABLE IF EXISTS temp_table;

-- Main migration logic
CREATE TABLE new_table (
    id INTEGER PRIMARY KEY,
    -- ... columns with comments
);

-- Create indexes for performance
CREATE INDEX idx_table_column ON new_table(column);

-- Add verification queries as comments
-- Verification: SELECT COUNT(*) FROM new_table;
```

### 2. Naming Guidelines
- **Views**: `{purpose}_{data_type}_view` → `piotroski_screening_view`
- **Tables**: `{entity}_{type}` → `income_statements`, `balance_sheets`
- **Indexes**: `idx_{table}_{column}` → `idx_stocks_symbol`
- **Temp objects**: `temp_{purpose}` → `temp_migration_data`

### 3. Migration Types
- **Schema changes**: New tables, columns, indexes
- **Data migrations**: Moving/transforming existing data
- **View updates**: New screening views, analytics views
- **Performance**: Index additions, optimizations

## Common Migration Patterns

### 1. Adding New Table
```sql
-- Migration: Add new cash flow statements table
-- Created: 2025-09-18
-- Purpose: Support cash flow analysis for Piotroski F-Score

CREATE TABLE cash_flow_statements (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER NOT NULL,
    fiscal_year INTEGER NOT NULL,
    fiscal_period TEXT NOT NULL,
    operating_cash_flow REAL,
    investing_cash_flow REAL,
    financing_cash_flow REAL,
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, fiscal_year, fiscal_period)
);

CREATE INDEX idx_cash_flow_stock_year ON cash_flow_statements(stock_id, fiscal_year);
```

### 2. Creating/Updating Views
```sql
-- Migration: Improved Piotroski F-Score screening view
-- Created: 2025-09-22
-- Purpose: Fix data quality issues and add missing criteria

-- Always drop existing views first
DROP VIEW IF EXISTS piotroski_screening_results;
DROP VIEW IF EXISTS piotroski_multi_year_data;

-- Create new view with proper logic
CREATE VIEW piotroski_screening_results AS
WITH financial_data AS (
    -- Complex view logic here
)
SELECT * FROM financial_data;
```

### 3. Data Migration
```sql
-- Migration: Migrate legacy data to new format
-- Created: 2025-09-05
-- Purpose: Move data from old schema to enhanced schema

-- Create temporary table for data transformation
CREATE TEMP TABLE temp_migration_data AS
SELECT id, symbol, old_column as new_column
FROM legacy_table;

-- Insert transformed data
INSERT INTO new_table (id, symbol, new_column)
SELECT id, symbol, new_column FROM temp_migration_data;

-- Verification
-- Expected count: SELECT COUNT(*) FROM temp_migration_data;
-- Actual count: SELECT COUNT(*) FROM new_table;
```

## Rollback Strategy

### 1. Automatic Backup Restoration
```bash
# List available backups (sorted by newest first)
ls -t src-tauri/db/stocks.db.backup.*

# Restore from most recent backup
cp src-tauri/db/stocks.db.backup.LATEST_TIMESTAMP src-tauri/db/stocks.db

# Verify restoration worked
cargo run --bin db_admin -- status
```

### 2. Migration Rollback Files
- For complex migrations, create corresponding rollback SQL
- Name: `rollback_YYYYMMDD##_description.sql`
- Test rollback before deploying migration

## Tools and Commands

### Database Administration
```bash
# Check database status (always run first)
cargo run --bin db_admin -- status

# Create manual backup
cargo run --bin db_admin -- backup

# Run migrations with safety checks (REQUIRES --confirm for production)
cargo run --bin db_admin -- migrate --confirm

# Verify data integrity
cargo run --bin db_admin -- verify
```

### Complete Migration Workflow Example

```bash
# 1. Navigate to project root
cd /Users/yksoni/code/misc/rust-stocks/src-tauri

# 2. Check current database status
cargo run --bin db_admin -- status
# Expected Output:
# 📊 Database Status: db/stocks.db
#    📈 Stocks: 5,892
#    💹 Price records: 6,565,513
#    🏢 Financial records: 50,673
#    💾 Size: 4651.54 MB
#    🚨 PRODUCTION DATABASE - Extra safeguards active

# 3. Review pending migrations
ls db/migrations/ | tail -5

# 4. Apply migrations with confirmation (required for production)
cargo run --bin db_admin -- migrate --confirm
# Expected Output:
# ✅ Database backup created: db/stocks.db.backup.20250923_045710
# ✅ Backup verified: 4877496320 bytes (original: 4877496320 bytes)
# 🧹 Cleaned up 4 old backup files, freed 18.2 GB of disk space
# ⚠️  PRODUCTION DATABASE DETECTED:
#    📊 Stocks: 5892
#    📈 Price records: 6565513
#    💾 Database size: 4651.54 MB
#    🔒 Additional safeguards active
# ✅ Database backup created: db/stocks.db.backup.20250923_045720
# 🔒 Migration backup: db/stocks.db.backup.20250923_045720
# ✅ Migrations completed successfully
# ✅ Data integrity verified after migration

# 5. Verify final status
cargo run --bin db_admin -- status
```

### Manual Operations
```bash
# Count records in key tables
sqlite3 src-tauri/db/stocks.db "
SELECT
    'stocks' as table_name, COUNT(*) as count FROM stocks
UNION ALL
SELECT 'daily_prices', COUNT(*) FROM daily_prices
UNION ALL
SELECT 'income_statements', COUNT(*) FROM income_statements;
"

# Check migration history (if tracking table exists)
sqlite3 src-tauri/db/stocks.db "SELECT * FROM schema_migrations ORDER BY version;"
```

## Migration Cleanup Guidelines

### When to Clean Up Migrations
- **Duplicate files**: Multiple versions of same migration
- **Obsolete migrations**: Superseded by newer versions
- **Wrong naming**: Files not following convention
- **Wrong location**: Files in incorrect directories

### Cleanup Process
1. **Backup first**: Always backup before cleanup
2. **Identify duplicates**: Find multiple files doing same thing
3. **Keep latest**: Retain most recent/complete version
4. **Test integrity**: Verify database still works
5. **Document changes**: Update this file with changes

### Recent Cleanup (2025-09-22)
- Cleaned 29 migrations → 23 clean migrations
- Fixed naming: `20241201000000_*` → `2025090501_*` (correct dates)
- Removed 5 duplicate piotroski migrations
- Established proper chronological order
- Backup created: `migrations_backup_20250922_185321/`

## Troubleshooting

### Common Issues and Solutions

#### 1. "Database or disk is full"
**Cause**: Insufficient disk space for backup creation
**Solution**:
```bash
# Check disk space
df -h

# The auto-cleanup system now handles this automatically, but if needed:
# List backup files
ls -la src-tauri/db/*.backup*

# Remove old backups manually (system keeps 2 newest automatically)
rm src-tauri/db/stocks.db.backup.OLD_TIMESTAMP
```

#### 2. "Migration XXXXXX was previously applied but has been modified"
**Cause**: Migration file checksum doesn't match what was previously applied
**Solution**: This indicates inconsistent migration tracking. Contact development team.

#### 3. "Duplicate column name" or "Table already exists"
**Cause**: Migration trying to create objects that already exist
**Solution**: Migration system may be inconsistent. The object exists but isn't tracked.
```bash
# Check what objects exist
sqlite3 src-tauri/db/stocks.db ".schema TABLE_NAME"
sqlite3 src-tauri/db/stocks.db "PRAGMA table_info(TABLE_NAME);"
```

#### 4. Migration hangs or takes too long
**Cause**: Large table alterations on production database (2.5GB+)
**Solution**:
- Monitor system resources: `top` or Activity Monitor
- Consider running during low-activity periods
- Check for table locks: `cargo run --bin db_admin -- verify`

#### 5. Out of disk space during migration
**Cause**: Multiple large backups filling disk
**Solution**: The system now auto-cleans old backups, but you can also:
```bash
# Check current backups (should only see 2 newest)
ls -lah src-tauri/db/*.backup*

# Check total disk usage
du -sh src-tauri/db/
```

### Emergency Procedures

#### Complete Database Restore
If migrations cause serious issues:

1. **Stop the application immediately**
2. **Restore from most recent backup**:
   ```bash
   cd /Users/yksoni/code/misc/rust-stocks/src-tauri/db
   # List available backups (newest first)
   ls -t stocks.db.backup.* | head -3

   # Restore from newest backup
   cp stocks.db.backup.LATEST_TIMESTAMP stocks.db
   ```
3. **Verify restore**: `cargo run --bin db_admin -- status`
4. **Contact development team** with error details

#### Backup Verification
To verify a backup is valid before restoring:
```bash
# Check backup file size (should be similar to original)
ls -lah stocks.db*

# Test backup database connection and basic query
sqlite3 stocks.db.backup.TIMESTAMP "SELECT COUNT(*) FROM stocks;"

# Compare stock counts between original and backup
sqlite3 stocks.db "SELECT COUNT(*) FROM stocks;"
sqlite3 stocks.db.backup.TIMESTAMP "SELECT COUNT(*) FROM stocks;"
```

### Migration System Recovery

If the migration tracking system becomes completely inconsistent:

1. **Contact development team** - don't attempt manual fixes
2. **Preserve current database** - ensure you have working backups
3. **Document the exact error messages** and steps that led to the issue
4. **Check recent git history** for migration-related changes

## Monitoring and Maintenance

### Regular Checks
- **Monthly**: Review migration file organization
- **After major features**: Verify no duplicate migrations
- **Before releases**: Ensure all migrations follow standards
- **Database growth**: Monitor backup sizes and cleanup old backups

### Key Metrics to Track
- Total migration count
- Database size growth
- Query performance after schema changes
- Backup success rate and auto-cleanup efficiency
- Migration execution time
- Disk space usage (now automatically managed)

### Recent Improvements (2025-09-23)
- **Automatic Backup Cleanup**: System now keeps only 2 most recent backups
- **Disk Space Management**: Automatically frees disk space after each backup
- **Production Safety**: Enhanced safety checks for large databases
- **Clear Error Messages**: Improved troubleshooting information
- **Auto-cleanup Reporting**: Shows how much disk space was freed

---

**Last Updated**: 2025-09-23
**Next Review**: 2025-10-23