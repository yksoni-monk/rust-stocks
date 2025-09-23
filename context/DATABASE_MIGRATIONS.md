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
- **Location**: `src-tauri/db/backups/`
- **Naming**: `stocks_backup_YYYYMMDD_HHMMSS.db`
- **Verification**: File size and integrity checks

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
# List available backups
ls -la src-tauri/db/backups/

# Restore from backup
cp src-tauri/db/backups/stocks_backup_YYYYMMDD_HHMMSS.db src-tauri/db/stocks.db
```

### 2. Migration Rollback Files
- For complex migrations, create corresponding rollback SQL
- Name: `rollback_YYYYMMDD##_description.sql`
- Test rollback before deploying migration

## Tools and Commands

### Database Administration
```bash
# Run migrations with safety checks
cargo run --bin db_admin migrate

# Check database status
cargo run --bin db_admin status

# Create manual backup
cargo run --bin db_admin backup

# Verify data integrity
cargo run --bin db_admin verify
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
- Backup success rate
- Migration execution time

---

**Last Updated**: 2025-09-22
**Next Review**: 2025-10-22