# Annual Data Standardization Plan

## Executive Summary

**Problem**: Inconsistent period_type values across financial tables causing screening methods to fail
**Root Cause**: Income statements use mix of 'Annual', 'FY', 'Quarterly', 'TTM' with data scattered across types
**Solution**: Standardize all data to 'Annual' period_type only, eliminate Quarterly/TTM data
**Impact**: Fix both Piotroski and O'Shaughnessy screening methods

## Current Data Analysis

### Data Distribution by Period Type

| Table | Period Type | Total Records | Records with Data | Coverage |
|-------|-------------|---------------|-------------------|----------|
| **Balance Sheets** | Annual | 65,496 | 64,355 | 98.3% ✅ |
| **Balance Sheets** | Quarterly | 410 | 395 | - |
| **Balance Sheets** | TTM | 34 | 0 | - |
| **Cash Flow** | Annual | 59,687 | 39,057 | 65.4% ✅ |
| **Cash Flow** | Quarterly | 348 | 323 | - |
| **Income Statements** | Annual | 18,996 | 538 | 2.8% ❌ |
| **Income Statements** | FY | 717 | 447 | 62.3% ✅ |
| **Income Statements** | Quarterly | 507 | 428 | - |
| **Income Statements** | TTM | 9 | 9 | - |

### Key Findings

1. **Balance Sheets**: Already 98% Annual data ✅
2. **Cash Flow**: Already 65% Annual data ✅
3. **Income Statements**: Data split between 'Annual' (empty) and 'FY' (populated) ❌

### Root Problem

Income statements have:
- 18,996 'Annual' records (mostly empty placeholders)
- 717 'FY' records (actual fiscal year data)
- Views filter for `period_type = 'Annual'` but data is in `period_type = 'FY'`

## Design Principles

1. **Single Source of Truth**: All annual financial data uses `period_type = 'Annual'`
2. **No Duplicates**: Eliminate Quarterly and TTM records from database
3. **Consistent Semantics**: 'Annual' means annual/fiscal year financial statements
4. **Proper Joins**: Use correct tables for each metric (shares from balance_sheets, not income_statements)

## Implementation Plan

### Phase 1: Data Migration - Consolidate Period Types

**Objective**: Merge 'FY' records into 'Annual', delete Quarterly/TTM

#### Step 1.1: Income Statements - Merge FY into Annual
```sql
-- For each stock with FY data but no Annual data, convert FY → Annual
UPDATE income_statements
SET period_type = 'Annual'
WHERE period_type = 'FY'
  AND (stock_id, report_date) NOT IN (
    SELECT stock_id, report_date
    FROM income_statements
    WHERE period_type = 'Annual' AND revenue IS NOT NULL
  );

-- Delete remaining FY records (duplicates where Annual exists)
DELETE FROM income_statements
WHERE period_type = 'FY';
```

#### Step 1.2: Delete Quarterly and TTM Data
```sql
-- Income statements
DELETE FROM income_statements
WHERE period_type IN ('Quarterly', 'TTM');

-- Balance sheets
DELETE FROM balance_sheets
WHERE period_type IN ('Quarterly', 'TTM');

-- Cash flow statements
DELETE FROM cash_flow_statements
WHERE period_type IN ('Quarterly', 'TTM');
```

#### Step 1.3: Delete Empty Annual Placeholder Records
```sql
-- Delete Annual records with no data
DELETE FROM income_statements
WHERE period_type = 'Annual'
  AND revenue IS NULL
  AND net_income IS NULL
  AND operating_income IS NULL;

DELETE FROM balance_sheets
WHERE period_type = 'Annual'
  AND total_assets IS NULL
  AND total_equity IS NULL;

DELETE FROM cash_flow_statements
WHERE period_type = 'Annual'
  AND operating_cash_flow IS NULL
  AND net_cash_flow IS NULL;
```

### Phase 2: Fix O'Shaughnessy View

**Objective**: Use correct Annual data sources with proper field mappings

#### Current Issues
1. Uses `shares_diluted` from income_statements (only 12 records have it)
2. Should use `shares_outstanding` from balance_sheets (420 records have it)
3. Filters for 'Annual' only (now correct after Phase 1)

#### Corrected View Definition
```sql
CREATE VIEW oshaughnessy_value_composite AS
SELECT
  s.id as stock_id,
  s.symbol,
  s.sector,

  -- Latest price data
  (SELECT close_price FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) as current_price,
  (SELECT market_cap FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) as market_cap,

  -- Latest Annual income statement data
  i.net_income,
  i.revenue,
  i.operating_income,

  -- Latest Annual balance sheet data
  b.total_equity,
  b.shares_outstanding,  -- ✅ CORRECT SOURCE for shares
  b.total_debt,
  b.cash_and_equivalents,

  -- Latest Annual cash flow data
  cf.dividends_paid,
  cf.share_repurchases,
  cf.depreciation_expense,
  cf.amortization_expense,

  -- Enterprise Value: Market Cap + Total Debt - Cash
  ((SELECT market_cap FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) +
   COALESCE(b.total_debt, 0) - COALESCE(b.cash_and_equivalents, 0)) as enterprise_value,

  -- EBITDA: Operating Income + Depreciation + Amortization
  (COALESCE(i.operating_income, 0) + COALESCE(cf.depreciation_expense, 0) +
   COALESCE(cf.amortization_expense, 0)) as ebitda,

  -- 1. P/E Ratio: Market Cap / Net Income
  CASE WHEN i.net_income > 0
       THEN (SELECT market_cap FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) / i.net_income
       ELSE NULL END as pe_ratio,

  -- 2. P/B Ratio: Market Cap / Total Equity
  CASE WHEN b.total_equity > 0
       THEN (SELECT market_cap FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) / b.total_equity
       ELSE NULL END as pb_ratio,

  -- 3. P/S Ratio: Market Cap / Revenue
  CASE WHEN i.revenue > 0
       THEN (SELECT market_cap FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) / i.revenue
       ELSE NULL END as ps_ratio,

  -- 4. EV/S Ratio: Enterprise Value / Revenue
  CASE WHEN i.revenue > 0
       THEN ((SELECT market_cap FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) +
             COALESCE(b.total_debt, 0) - COALESCE(b.cash_and_equivalents, 0)) / i.revenue
       ELSE NULL END as evs_ratio,

  -- 5. EV/EBITDA Ratio: Enterprise Value / EBITDA
  CASE WHEN (COALESCE(i.operating_income, 0) + COALESCE(cf.depreciation_expense, 0) +
             COALESCE(cf.amortization_expense, 0)) > 0
       THEN ((SELECT market_cap FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) +
             COALESCE(b.total_debt, 0) - COALESCE(b.cash_and_equivalents, 0)) /
            (COALESCE(i.operating_income, 0) + COALESCE(cf.depreciation_expense, 0) +
             COALESCE(cf.amortization_expense, 0))
       ELSE NULL END as ev_ebitda_ratio,

  -- 6. Shareholder Yield: (Dividends + Buybacks) / Market Cap
  CASE WHEN (SELECT market_cap FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) > 0
       THEN (COALESCE(cf.dividends_paid, 0) + COALESCE(cf.share_repurchases, 0)) /
            (SELECT market_cap FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1)
       ELSE NULL END as shareholder_yield,

  -- Data completeness score (0-100)
  ((CASE WHEN i.net_income > 0 THEN 1 ELSE 0 END) +
   (CASE WHEN b.total_equity > 0 THEN 1 ELSE 0 END) +
   (CASE WHEN i.revenue > 0 THEN 1 ELSE 0 END) +
   (CASE WHEN i.revenue > 0 THEN 1 ELSE 0 END) +
   (CASE WHEN (COALESCE(i.operating_income, 0) + COALESCE(cf.depreciation_expense, 0) +
               COALESCE(cf.amortization_expense, 0)) > 0 THEN 1 ELSE 0 END) +
   (CASE WHEN (SELECT market_cap FROM daily_prices WHERE stock_id = s.id ORDER BY date DESC LIMIT 1) > 0 THEN 1 ELSE 0 END)
  ) * 16.67 as data_completeness_score

FROM stocks s
LEFT JOIN (
  SELECT stock_id, net_income, revenue, operating_income, report_date,
         ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
  FROM income_statements
  WHERE period_type = 'Annual' AND revenue IS NOT NULL
) i ON s.id = i.stock_id AND i.rn = 1
LEFT JOIN (
  SELECT stock_id, total_equity, shares_outstanding, total_debt, cash_and_equivalents, report_date,
         ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
  FROM balance_sheets
  WHERE period_type = 'Annual' AND total_equity IS NOT NULL
) b ON s.id = b.stock_id AND b.rn = 1
LEFT JOIN (
  SELECT stock_id, dividends_paid, share_repurchases, depreciation_expense, amortization_expense, report_date,
         ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
  FROM cash_flow_statements
  WHERE period_type = 'Annual' AND operating_cash_flow IS NOT NULL
) cf ON s.id = cf.stock_id AND cf.rn = 1
WHERE s.is_sp500 = 1;
```

### Phase 3: Fix Piotroski View

**Objective**: Ensure all 9 criteria use Annual data only

The Piotroski view should already work correctly after Phase 1 since it filters for `period_type = 'Annual'`. Just need to verify the view definition is correct.

### Phase 4: Update Schema Constraints

**Objective**: Enforce Annual-only data at database level

```sql
-- Add CHECK constraint to ensure only Annual period_type
ALTER TABLE income_statements ADD CONSTRAINT check_period_type_annual
  CHECK (period_type = 'Annual');

ALTER TABLE balance_sheets ADD CONSTRAINT check_period_type_annual
  CHECK (period_type = 'Annual');

ALTER TABLE cash_flow_statements ADD CONSTRAINT check_period_type_annual
  CHECK (period_type = 'Annual');
```

### Phase 5: Update Data Import Code

**Objective**: Ensure future data imports only create Annual records

Files to update:
- `src-tauri/src/bin/refresh_data.rs`
- `src-tauri/src/tools/sec_edgar_client.rs`
- Any EDGAR extraction code

Changes:
1. Remove all Quarterly/TTM period type creation
2. Standardize FY → Annual during import
3. Add validation to reject non-Annual data

## Expected Results

### Data Coverage After Migration

| Metric | Before | After | Source |
|--------|--------|-------|--------|
| **Income Statements (Annual)** | 538 | 985+ | Merge FY + Annual |
| **Balance Sheets (Annual)** | 64,355 | 64,355 | Already good |
| **Cash Flow (Annual)** | 39,057 | 39,057 | Already good |
| **O'Shaughnessy Coverage** | 0% | 85%+ | Fixed view + data |
| **Piotroski Coverage** | 99.4% | 99.4% | Already working |

### O'Shaughnessy Metrics Coverage (Estimated)

After fixing the view to use shares_outstanding from balance_sheets:
- PE Ratio: ~85% (limited by net_income availability)
- PB Ratio: ~98% (balance sheets are good)
- PS Ratio: ~95% (revenue coverage)
- EV/EBITDA: ~70% (limited by cash flow coverage)
- EV/S: ~95% (revenue coverage)
- Shareholder Yield: ~65% (limited by cash flow coverage)

## Migration Files

### Migration 1: Consolidate Period Types
File: `2025123105_consolidate_period_types_to_annual.sql`

### Migration 2: Fix O'Shaughnessy View
File: `2025123106_fix_oshaughnessy_annual_data.sql`

### Migration 3: Add Schema Constraints
File: `2025123107_enforce_annual_only_constraint.sql`

## Testing Strategy

1. **Pre-Migration Verification**: Count records by period_type
2. **Post-Migration Verification**: Confirm only Annual records remain
3. **O'Shaughnessy Test**: Query view and verify metrics populate
4. **Piotroski Test**: Verify all 9 criteria still work
5. **Data Integrity**: Ensure no data loss during consolidation

## Rollback Plan

Before running migrations:
1. Backup database: `cp stocks.db stocks.db.backup`
2. If issues occur: `mv stocks.db.backup stocks.db`

## Timeline

- **Phase 1**: 5 minutes (data migration)
- **Phase 2**: 5 minutes (fix O'Shaughnessy view)
- **Phase 3**: 2 minutes (verify Piotroski view)
- **Phase 4**: 2 minutes (add constraints)
- **Phase 5**: TBD (code updates for future imports)

**Total Time**: ~15 minutes for database fixes, separate task for code updates
