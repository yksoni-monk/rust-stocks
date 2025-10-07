# UI Cleanup and Screening Fix Plan

**Date**: 2025-10-07
**Status**: Ready to Execute

---

## 🎯 Objectives

1. Remove Data Management tab from UI (stops auto-refresh on launch)
2. Remove all data refresh backend commands exposed to UI
3. Fix Piotroski F-Score screening (broken view)
4. Fix O'Shaughnessy Value screening (returns no results)
5. Update TypeScript bindings

---

## 🔍 Root Cause Analysis

### Issue 1: Auto-refresh on Launch
**Symptom**: Backend starts `refresh_data` when UI launches
**Root Cause**:
- File: `src/src/App.tsx:24-29`
- `createEffect` triggers when `data-management` tab is active
- Calls `dataRefreshStore.checkDataFreshness()` which hits backend
- Backend command `get_data_freshness_status` runs queries

**Fix**: Remove entire data-management tab and associated code

### Issue 2: Piotroski Screening Broken
**Symptom**: "Analysis Error: Failed to load recommendations"
**Root Cause**:
- File: `src-tauri/src/commands/piotroski_screening.rs:112`
- Query references: `FROM piotroski_screening_results_new`
- **View name mismatch**: Migration creates `piotroski_screening_results` but code expects `piotroski_screening_results_new`
- Database has views: `piotroski_screening_results`, `piotroski_f_score_complete`, `piotroski_multi_year_data`

**Fix**: Change code to use `piotroski_screening_results` (1-line fix!)

### Issue 3: O'Shaughnessy Screening Returns No Results
**Symptom**: "No S&P 500 stocks currently meet the OSHAUGHNESSY screening criteria"
**Root Cause**: (To be investigated)
- Views exist: `oshaughnessy_value_composite`, `oshaughnessy_ranking`
- Need to check if views have data
- May need to recalculate ratios

**Fix**: Verify data population and fix query if needed

---

## 📋 Implementation Plan

### Phase 1: Remove Data Management Tab & Backend (30 min)

#### 1.1 Frontend Cleanup

**Files to Delete**:
- [ ] `src/src/components/SimpleDataManagement.tsx`
- [ ] `src/src/stores/dataRefreshStore.ts`

**Files to Modify**:
- [ ] `src/src/App.tsx`
  - Remove import: `SimpleDataManagement`
  - Remove import: `dataRefreshStore`
  - Remove effect at lines 24-29 (checkDataFreshness)
  - Remove data-management tab button (lines 54-70)
  - Remove data-management tab content (lines 94-97)

- [ ] `src/src/stores/uiStore.ts`
  - Remove `openDataManagement()` method
  - Remove 'data-management' from `activeTab` type

**Expected Result**: UI has only "🔍 Stock Screening" tab

#### 1.2 Backend Cleanup

**Files to Delete**:
- [ ] `src-tauri/src/commands/data_refresh.rs`

**Files to Modify**:
- [ ] `src-tauri/src/commands/mod.rs`
  - Remove: `pub mod data_refresh;`

- [ ] `src-tauri/src/lib.rs`
  - Remove imports for data_refresh commands
  - Remove from `.invoke_handler()`:
    - `commands::data_refresh::get_data_freshness_status`
    - `commands::data_refresh::check_screening_readiness`
    - `commands::data_refresh::start_data_refresh`
    - `commands::data_refresh::get_refresh_progress`
    - `commands::data_refresh::cancel_refresh`

**Files to Keep**:
- ✅ `src-tauri/src/tools/data_refresh_orchestrator.rs` (used by CLI)
- ✅ `src-tauri/src/tools/data_freshness_checker.rs` (used by CLI)
- ✅ `src-tauri/src/bin/refresh_data.rs` (CLI binary)

**Expected Result**: UI cannot trigger backend refresh, only CLI can

---

### Phase 2: Fix Piotroski F-Score Screening (5 min)

#### 2.1 Investigation ✅ COMPLETE

**Discovery**:
- Migration `2025092531_complete_net_margin_piotroski.sql` creates view: `piotroski_screening_results`
- Code expects: `piotroski_screening_results_new` ❌
- Views exist in database: `piotroski_screening_results`, `piotroski_f_score_complete`, `piotroski_multi_year_data`

#### 2.2 Solution: Fix View Name (1-line change)

**Option**: Update code to use correct view name ⭐ RECOMMENDED
- Change `piotroski_screening_results_new` → `piotroski_screening_results`
- View already has all 9 criteria with net_margin
- No database changes needed

#### 2.3 Implementation

**File**: `src-tauri/src/commands/piotroski_screening.rs`

**Required Columns**:
```rust
// From user requirement:
- criterion_improving_net_margin  // Use net_margin (not gross_margin)

// 9 Piotroski Criteria:
1. criterion_positive_net_income
2. criterion_positive_operating_cash_flow
3. criterion_improving_roa
4. criterion_cash_flow_quality
5. criterion_decreasing_debt_ratio
6. criterion_improving_current_ratio
7. criterion_no_dilution
8. criterion_improving_net_margin  // MODIFIED
9. criterion_improving_asset_turnover
```

**New Query Structure**:
```sql
WITH latest_financials AS (
  SELECT
    s.id as stock_id,
    s.symbol,
    s.sector,
    -- Get 2 most recent years for year-over-year comparisons
    -- Calculate net_margin = net_income / revenue
    -- Calculate ROA, debt_ratio, current_ratio, asset_turnover
    ...
  FROM stocks s
  WHERE s.is_sp500 = 1
),
piotroski_calc AS (
  SELECT
    *,
    -- Calculate each criterion (0 or 1)
    CASE WHEN current_net_income > 0 THEN 1 ELSE 0 END as criterion_positive_net_income,
    ...
    CASE WHEN current_net_margin > prior_net_margin THEN 1 ELSE 0 END as criterion_improving_net_margin,
    ...
  FROM latest_financials
)
SELECT
  *,
  (criterion_positive_net_income + criterion_positive_operating_cash_flow +
   criterion_improving_roa + criterion_cash_flow_quality +
   criterion_decreasing_debt_ratio + criterion_improving_current_ratio +
   criterion_no_dilution + criterion_improving_net_margin +
   criterion_improving_asset_turnover) as f_score_complete
FROM piotroski_calc
ORDER BY f_score_complete DESC
```

#### 2.4 Testing

**Test Query**:
```bash
cd src-tauri
cargo run --bin refresh_data financials --only-ticker AAPL  # Ensure fresh data
```

**Test Screening**:
```bash
# Launch UI
npm run tauri dev

# Click "Run Piotroski Screen"
# Should see results without "Analysis Error"
```

**Expected Result**: Piotroski screening returns top stocks with F-Score 6-9

---

### Phase 3: Fix O'Shaughnessy Value Screening (30 min)

#### 3.1 Investigation

**Check Views**:
```bash
sqlite3 src-tauri/db/stocks.db "SELECT * FROM oshaughnessy_value_composite LIMIT 5;"
sqlite3 src-tauri/db/stocks.db "SELECT * FROM oshaughnessy_ranking LIMIT 5;"
```

**Possible Issues**:
1. Views exist but have no data (ratios not calculated)
2. Query has wrong filters (no stocks pass criteria)
3. View schema doesn't match code expectations

#### 3.2 Check Data Population

**File**: `src-tauri/src/commands/oshaughnessy_screening.rs:76`

**Current Query**:
```rust
// Reads from: oshaughnessy_value_composite or oshaughnessy_ranking
```

**Verify**:
- [ ] Views have data for S&P 500 stocks
- [ ] Composite scores are calculated
- [ ] Ranking percentiles are assigned

#### 3.3 Fix Query or Recalculate Ratios

**Option A**: If views are empty
```bash
# Recalculate O'Shaughnessy ratios
cd src-tauri
cargo run --bin calculate-oshaughnessy-ratios
```

**Option B**: If query is broken
- Update query in `oshaughnessy_screening.rs`
- Fix any column name mismatches
- Adjust filters to return results

#### 3.4 Testing

**Test Screening**:
```bash
# Launch UI
npm run tauri dev

# Click O'Shaughnessy tab
# Should see "Top 10 Value Stocks"
```

**Expected Result**: O'Shaughnessy screening returns top 10 undervalued stocks

---

### Phase 4: Update TypeScript Bindings (10 min)

#### 4.1 Regenerate Bindings

**Command**:
```bash
cd src-tauri
cargo test --features ts-rs -- --nocapture
```

**Files Updated**:
- `src/bindings/*.ts` (auto-generated)

#### 4.2 Verify Removals

**Check that these types are removed**:
- [ ] `RefreshRequestDto`
- [ ] `RefreshProgressDto`
- [ ] `RefreshCompletedEvent`
- [ ] `SystemFreshnessReport`

**Keep these types**:
- ✅ `PiotoskiFScoreResult`
- ✅ `OShaughnessyResult`
- ✅ All screening-related types

---

## ✅ Acceptance Criteria

### Phase 1 Success Criteria:
- [ ] UI launches without backend refresh calls
- [ ] No "Data Management" tab visible
- [ ] No console errors about missing data refresh functions
- [ ] Backend log shows NO refresh_data activity on UI launch

### Phase 2 Success Criteria:
- [ ] Piotroski screening button works
- [ ] Returns top 10 stocks with F-Score ≥ 6
- [ ] Shows all 9 criteria scores
- [ ] Uses net_margin (not gross_margin)
- [ ] No "Analysis Error"

### Phase 3 Success Criteria:
- [ ] O'Shaughnessy screening button works
- [ ] Returns top 10 undervalued stocks
- [ ] Shows composite score and ranking
- [ ] No "No Stocks Found" error

### Phase 4 Success Criteria:
- [ ] TypeScript bindings compile without errors
- [ ] No import errors for removed types
- [ ] Frontend builds successfully

---

## 🚨 Rollback Plan

If any phase fails:

**Phase 1**: Restore files from git
```bash
git restore src/src/components/SimpleDataManagement.tsx
git restore src/src/stores/dataRefreshStore.ts
git restore src/src/App.tsx
git restore src-tauri/src/commands/data_refresh.rs
```

**Phase 2**: Revert Piotroski query changes
```bash
git restore src-tauri/src/commands/piotroski_screening.rs
```

**Phase 3**: Revert O'Shaughnessy changes
```bash
git restore src-tauri/src/commands/oshaughnessy_screening.rs
```

**Phase 4**: Regenerate bindings from clean state
```bash
git restore src/bindings/
cargo test --features ts-rs
```

---

## 📊 Estimated Timeline

- Phase 1 (Remove Data Management): 30 minutes
- Phase 2 (Fix Piotroski): 45 minutes
- Phase 3 (Fix O'Shaughnessy): 30 minutes
- Phase 4 (Update Bindings): 10 minutes
- **Total**: ~2 hours

---

## 📝 Notes

### User Requirements:
1. **Net Margin vs Gross Margin**: User specifically noted "We use net margin instead of gross margin because many companies don't have gross margin data"
2. **Auto-refresh Issue**: User frustrated that backend starts refresh on UI launch
3. **Data Management Removal**: User wants complete removal of data management UI

### Technical Considerations:
1. Keep CLI `refresh_data` binary intact (user needs this)
2. Keep all `tools/*` files (used by CLI)
3. Only remove UI-facing commands
4. Piotroski view was never created (probably lost in migration history)
5. O'Shaughnessy views exist but may need data refresh

---

**Ready to Execute**: ✅
**Backup Created**: Will create git stash before changes
**Review Completed**: Plan approved by user
