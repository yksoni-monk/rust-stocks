# Transaction-Based Storage Implementation Checklist

## Overview
Implement ACID-compliant transaction-based storage for financial data to enforce the invariant: **Every sec_filing MUST have all 3 associated financial statements**.

## Current State Analysis

### Files to Modify
1. `src-tauri/src/tools/sec_edgar_client.rs` - Core storage functions
2. `src-tauri/src/tools/data_freshness_checker.rs` - Extraction and orchestration logic

### Current Storage Flow (BROKEN)
```
For each filed_date in missing_dates:
  1. Group data by filed_date → HashMap<filed_date, (report_date, data)>
  2. For each (filed_date, data) in grouped:
     a. Create BalanceSheetData struct
     b. Find matching metadata (or skip if not found)
     c. Call store_balance_sheet_data(pool, data, metadata)
        - Creates sec_filing record (or gets existing)
        - Inserts balance_sheet record
     d. Increment records_stored counter
  3. Same for income_statement_data
  4. Same for cash_flow_data
  5. Return total records_stored

PROBLEM: If step 2c succeeds but 3c or 4c fails, we have orphaned sec_filing!
```

### Current Function Signatures

**sec_edgar_client.rs**:
```rust
// Line 1009
pub async fn store_balance_sheet_data(
    &self,
    data: &BalanceSheetData,
    filing_metadata: Option<&FilingMetadata>
) -> Result<()>

// Similar for income_statement and cash_flow
pub async fn store_income_statement_data(...)
pub async fn store_cash_flow_data(...)

// Line ~950
async fn create_or_get_sec_filing(
    &self,
    stock_id: i64,
    metadata: &FilingMetadata,
    fiscal_year: i32,
    report_date_str: &str
) -> Result<i64>
```

**data_freshness_checker.rs**:
```rust
// Line 623
async fn extract_and_store_balance_sheet_data(
    edgar_client: &mut SecEdgarClient,
    json: &serde_json::Value,
    stock_id: i64,
    symbol: &str,
    missing_dates: &[String]
) -> Result<i64>

// Similar for income_statement and cash_flow
```

## Implementation Plan

### Phase 1: Add Transaction Support to Storage Functions

#### Step 1.1: Add Transaction Variants of Storage Functions
**File**: `src-tauri/src/tools/sec_edgar_client.rs`

Add new transaction-aware versions alongside existing functions:

```rust
// New: Transaction-aware version
pub async fn store_balance_sheet_data_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    data: &BalanceSheetData,
    sec_filing_id: i64
) -> Result<()> {
    let query = r#"
        INSERT INTO balance_sheets (
            stock_id, period_type, report_date, fiscal_year,
            total_assets, total_liabilities, total_equity,
            cash_and_equivalents, current_assets, current_liabilities,
            short_term_debt, long_term_debt, total_debt,
            share_repurchases, sec_filing_id
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    "#;

    sqlx::query(query)
        .bind(data.stock_id)
        .bind("Annual")
        .bind(&data.report_date)
        .bind(data.fiscal_year)
        .bind(data.total_assets)
        .bind(data.total_liabilities)
        .bind(data.total_equity)
        .bind(data.cash_and_equivalents)
        .bind(data.current_assets)
        .bind(data.current_liabilities)
        .bind(data.short_term_debt)
        .bind(data.long_term_debt)
        .bind(data.total_debt)
        .bind(data.share_repurchases)
        .bind(sec_filing_id)
        .execute(&mut **tx)  // Note: &mut **tx for transaction
        .await?;

    Ok(())
}

// Keep existing store_balance_sheet_data for backward compatibility
// It can now call the transaction version internally
```

**Action Items**:
- [ ] Add `store_balance_sheet_data_tx` function
- [ ] Add `store_income_statement_data_tx` function
- [ ] Add `store_cash_flow_data_tx` function
- [ ] Add `create_or_get_sec_filing_tx` function (transaction-aware)

#### Step 1.2: Update create_or_get_sec_filing for Transactions
**File**: `src-tauri/src/tools/sec_edgar_client.rs`

```rust
async fn create_or_get_sec_filing_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    stock_id: i64,
    metadata: &FilingMetadata,
    fiscal_year: i32,
    report_date_str: &str
) -> Result<i64> {
    // Try to get existing
    let existing: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM sec_filings WHERE stock_id = ? AND accession_number = ?"
    )
    .bind(stock_id)
    .bind(&metadata.accession_number)
    .fetch_optional(&mut **tx)
    .await?;

    if let Some(id) = existing {
        return Ok(id);
    }

    // Insert new
    let insert_query = r#"
        INSERT INTO sec_filings (
            stock_id, accession_number, form_type, filed_date,
            fiscal_period, fiscal_year, report_date
        ) VALUES (?, ?, ?, ?, ?, ?, ?)
    "#;

    let result = sqlx::query(insert_query)
        .bind(stock_id)
        .bind(&metadata.accession_number)
        .bind(&metadata.form_type)
        .bind(&metadata.filing_date)
        .bind(&metadata.fiscal_period)
        .bind(fiscal_year)
        .bind(report_date_str)
        .execute(&mut **tx)
        .await?;

    Ok(result.last_insert_rowid())
}
```

**Action Items**:
- [ ] Create `create_or_get_sec_filing_tx` function
- [ ] Handle UNIQUE constraint violations gracefully
- [ ] Add logging for new vs existing filings

### Phase 2: Create Atomic Storage Function

#### Step 2.1: Add store_filing_atomic Function
**File**: `src-tauri/src/tools/data_freshness_checker.rs`

Add new function to handle transaction-based storage:

```rust
/// Store a complete filing (sec_filing + all 3 financial statements) atomically
async fn store_filing_atomic(
    pool: &SqlitePool,
    edgar_client: &SecEdgarClient,
    stock_id: i64,
    symbol: &str,
    filed_date: &str,
    report_date: &str,
    fiscal_year: i32,
    metadata: &FilingMetadata,
    balance_data: &HashMap<String, f64>,
    income_data: &HashMap<String, f64>,
    cashflow_data: &HashMap<String, f64>
) -> Result<i64> {
    // Start transaction
    let mut tx = pool.begin().await?;

    // 1. Create or get sec_filing
    let sec_filing_id = edgar_client.create_or_get_sec_filing_tx(
        &mut tx,
        stock_id,
        metadata,
        fiscal_year,
        report_date
    ).await?;

    println!("    📋 Working with sec_filing ID={} for filed_date={}", sec_filing_id, filed_date);

    // 2. Prepare balance sheet data
    let balance_sheet_data = BalanceSheetData {
        stock_id,
        symbol: symbol.to_string(),
        report_date: chrono::NaiveDate::parse_from_str(report_date, "%Y-%m-%d")?,
        fiscal_year,
        total_assets: balance_data.get("Assets").copied(),
        total_liabilities: balance_data.get("Liabilities").copied(),
        total_equity: balance_data.get("StockholdersEquity").copied(),
        cash_and_equivalents: balance_data.get("CashAndCashEquivalentsAtCarryingValue").copied(),
        short_term_debt: balance_data.get("ShortTermDebt").copied()
            .or_else(|| balance_data.get("DebtCurrent").copied()),
        long_term_debt: balance_data.get("LongTermDebt").copied()
            .or_else(|| balance_data.get("LongTermDebtAndCapitalLeaseObligations").copied()),
        total_debt: {
            let st = balance_data.get("ShortTermDebt").copied()
                .or_else(|| balance_data.get("DebtCurrent").copied());
            let lt = balance_data.get("LongTermDebt").copied()
                .or_else(|| balance_data.get("LongTermDebtAndCapitalLeaseObligations").copied());
            match (st, lt) {
                (Some(s), Some(l)) => Some(s + l),
                (Some(s), None) => Some(s),
                (None, Some(l)) => Some(l),
                _ => None,
            }
        },
        current_assets: balance_data.get("AssetsCurrent").copied(),
        current_liabilities: balance_data.get("LiabilitiesCurrent").copied(),
        share_repurchases: balance_data.get("ShareRepurchases").copied(),
    };

    // 3. Store balance sheet
    edgar_client.store_balance_sheet_data_tx(&mut tx, &balance_sheet_data, sec_filing_id).await?;

    // 4. Prepare income statement data
    let income_statement_data = IncomeStatementData {
        stock_id,
        symbol: symbol.to_string(),
        period_type: "Annual".to_string(),
        report_date: chrono::NaiveDate::parse_from_str(report_date, "%Y-%m-%d")?,
        fiscal_year,
        revenue: income_data.get("Revenues").copied()
            .or_else(|| income_data.get("RevenueFromContractWithCustomerExcludingAssessedTax").copied()),
        gross_profit: income_data.get("GrossProfit").copied(),
        operating_income: income_data.get("OperatingIncomeLoss").copied(),
        net_income: income_data.get("NetIncomeLoss").copied(),
        cost_of_revenue: income_data.get("CostOfGoodsAndServicesSold").copied(),
        interest_expense: income_data.get("InterestExpense").copied(),
        tax_expense: income_data.get("IncomeTaxExpenseBenefit").copied(),
        shares_basic: income_data.get("WeightedAverageNumberOfSharesOutstandingBasic").copied(),
        shares_diluted: income_data.get("WeightedAverageNumberOfSharesOutstandingDiluted").copied(),
    };

    // 5. Store income statement
    edgar_client.store_income_statement_data_tx(&mut tx, &income_statement_data, sec_filing_id).await?;

    // 6. Prepare cash flow data
    let cash_flow_data = CashFlowData {
        stock_id,
        symbol: symbol.to_string(),
        report_date: chrono::NaiveDate::parse_from_str(report_date, "%Y-%m-%d")?,
        fiscal_year,
        operating_cash_flow: cashflow_data.get("NetCashProvidedByUsedInOperatingActivities").copied(),
        depreciation_expense: cashflow_data.get("DepreciationDepletionAndAmortization").copied()
            .or_else(|| cashflow_data.get("DepreciationExpense").copied()),
        amortization_expense: cashflow_data.get("AmortizationExpense").copied(),
        investing_cash_flow: cashflow_data.get("NetCashProvidedByUsedInInvestingActivities").copied(),
        financing_cash_flow: cashflow_data.get("NetCashProvidedByUsedInFinancingActivities").copied(),
        dividends_paid: cashflow_data.get("PaymentsOfDividends").copied(),
        share_repurchases: cashflow_data.get("PaymentsForRepurchaseOfCommonStock").copied(),
    };

    // 7. Store cash flow
    edgar_client.store_cash_flow_data_tx(&mut tx, &cash_flow_data, sec_filing_id).await?;

    // 8. Commit transaction
    tx.commit().await?;

    println!("    ✅ Atomically stored filing {} with all 3 statements", filed_date);

    Ok(1)
}
```

**Action Items**:
- [ ] Create `store_filing_atomic` function
- [ ] Add proper error handling and logging
- [ ] Ensure transaction rollback on any failure

### Phase 3: Refactor Extraction Functions

#### Step 3.1: Update extract_and_store_balance_sheet_data
**File**: `src-tauri/src/tools/data_freshness_checker.rs`

**Current approach** (BROKEN):
- Processes one filed_date at a time
- Stores balance_sheet immediately
- No coordination with income_statement and cash_flow

**New approach** (TRANSACTIONAL):
- Parse all 3 statement types FIRST
- Group by filed_date for ALL 3 types
- For each filed_date, call `store_filing_atomic` with ALL 3 data sets

```rust
/// Extract and store ALL financial statements for missing dates (TRANSACTIONAL)
async fn extract_and_store_all_statements_atomic(
    edgar_client: &mut SecEdgarClient,
    pool: &SqlitePool,
    json: &serde_json::Value,
    stock_id: i64,
    symbol: &str,
    missing_dates: &[String]
) -> Result<i64> {
    println!("    📊 Parsing all financial statement types for {}...", symbol);

    // 1. Parse all 3 statement types
    let balance_data = edgar_client.parse_company_facts_json(json, symbol)?;
    let income_data = edgar_client.parse_income_statement_json(json, symbol)?;
    let cashflow_data = edgar_client.parse_cash_flow_json(json, symbol)?;

    // 2. Extract filing metadata (comprehensive)
    let filing_metadata_vec = edgar_client.extract_filing_metadata(json, symbol)?;

    // 3. Group by filed_date for each statement type
    let mut balance_by_filed: HashMap<String, (String, HashMap<String, f64>)> = HashMap::new();
    let mut income_by_filed: HashMap<String, (String, HashMap<String, f64>)> = HashMap::new();
    let mut cashflow_by_filed: HashMap<String, (String, HashMap<String, f64>)> = HashMap::new();

    for (field_name, value, report_date, filed_date) in balance_data {
        if missing_dates.contains(&filed_date) {
            let entry = balance_by_filed.entry(filed_date.clone())
                .or_insert_with(|| (report_date.clone(), HashMap::new()));
            if entry.0.is_empty() && !report_date.is_empty() { entry.0 = report_date.clone(); }
            entry.1.insert(field_name, value);
        }
    }

    for (field_name, value, report_date, filed_date) in income_data {
        if missing_dates.contains(&filed_date) {
            let entry = income_by_filed.entry(filed_date.clone())
                .or_insert_with(|| (report_date.clone(), HashMap::new()));
            if entry.0.is_empty() && !report_date.is_empty() { entry.0 = report_date.clone(); }
            entry.1.insert(field_name, value);
        }
    }

    for (field_name, value, report_date, filed_date) in cashflow_data {
        if missing_dates.contains(&filed_date) {
            let entry = cashflow_by_filed.entry(filed_date.clone())
                .or_insert_with(|| (report_date.clone(), HashMap::new()));
            if entry.0.is_empty() && !report_date.is_empty() { entry.0 = report_date.clone(); }
            entry.1.insert(field_name, value);
        }
    }

    println!("    📈 Found {} balance sheet dates, {} income dates, {} cashflow dates",
        balance_by_filed.len(), income_by_filed.len(), cashflow_by_filed.len());

    // 4. Store each filing atomically (all 3 statements in one transaction)
    let mut records_stored = 0;

    for filed_date in missing_dates {
        // Get data for this filed_date (may not exist in all 3 types)
        let balance_entry = balance_by_filed.get(filed_date);
        let income_entry = income_by_filed.get(filed_date);
        let cashflow_entry = cashflow_by_filed.get(filed_date);

        // Skip if we don't have at least one statement type
        if balance_entry.is_none() && income_entry.is_none() && cashflow_entry.is_none() {
            println!("    ⚠️ Skipping {} - no data in any statement type", filed_date);
            continue;
        }

        // Determine report_date (use first non-empty from any statement)
        let report_date = balance_entry.map(|(rd, _)| rd)
            .or_else(|| income_entry.map(|(rd, _)| rd))
            .or_else(|| cashflow_entry.map(|(rd, _)| rd))
            .ok_or_else(|| anyhow::anyhow!("No report_date found for {}", filed_date))?;

        let fiscal_year = report_date.split('-').next().unwrap().parse::<i32>().unwrap_or(0);

        // Find matching metadata
        let meta = filing_metadata_vec.iter().find(|m| &m.filing_date == filed_date);

        if let Some(metadata) = meta {
            println!("    💾 Storing filing for {} (report_date={}, fiscal_year={})",
                filed_date, report_date, fiscal_year);

            // Store atomically
            match store_filing_atomic(
                pool,
                edgar_client,
                stock_id,
                symbol,
                filed_date,
                report_date,
                fiscal_year,
                metadata,
                &balance_entry.map(|(_, data)| data.clone()).unwrap_or_default(),
                &income_entry.map(|(_, data)| data.clone()).unwrap_or_default(),
                &cashflow_entry.map(|(_, data)| data.clone()).unwrap_or_default()
            ).await {
                Ok(count) => records_stored += count,
                Err(e) => {
                    println!("🔴 ERROR: Failed to store filing {} atomically: {}", filed_date, e);
                    // Transaction already rolled back automatically
                    // Continue processing other filings
                }
            }
        } else {
            println!("🔴 WARNING: No matching metadata for {} filed_date={}", symbol, filed_date);
            println!("   Available metadata filing_dates: {:?}",
                filing_metadata_vec.iter().map(|m| &m.filing_date).collect::<Vec<_>>());
        }
    }

    Ok(records_stored)
}
```

**Action Items**:
- [ ] Create new `extract_and_store_all_statements_atomic` function
- [ ] Replace the 3 separate extract functions with this unified one
- [ ] Update call sites in `get_all_sec_filings_for_cik_and_extract_data`

### Phase 4: Add Defensive Cleanup

#### Step 4.1: Add Orphan Detection and Cleanup
**File**: `src-tauri/src/tools/data_freshness_checker.rs`

```rust
/// Clean up any orphaned sec_filings (defensive - should never find any with transactions)
pub async fn cleanup_orphaned_sec_filings(pool: &SqlitePool) -> Result<i64> {
    let query = r#"
        DELETE FROM sec_filings
        WHERE id IN (
            SELECT sf.id
            FROM sec_filings sf
            WHERE NOT EXISTS (
                SELECT 1 FROM balance_sheets bs WHERE bs.sec_filing_id = sf.id
            )
            OR NOT EXISTS (
                SELECT 1 FROM income_statements inc WHERE inc.sec_filing_id = sf.id
            )
            OR NOT EXISTS (
                SELECT 1 FROM cash_flow_statements cf WHERE cf.sec_filing_id = sf.id
            )
        )
    "#;

    let result = sqlx::query(query).execute(pool).await?;
    let deleted = result.rows_affected();

    if deleted > 0 {
        println!("🧹 Cleaned up {} orphaned sec_filings", deleted);
    }

    Ok(deleted as i64)
}

/// Verify data integrity - should always return 0 orphans
pub async fn verify_data_integrity(pool: &SqlitePool) -> Result<i64> {
    let query = r#"
        SELECT COUNT(*) as orphan_count
        FROM sec_filings sf
        WHERE NOT EXISTS (SELECT 1 FROM balance_sheets bs WHERE bs.sec_filing_id = sf.id)
           OR NOT EXISTS (SELECT 1 FROM income_statements inc WHERE inc.sec_filing_id = sf.id)
           OR NOT EXISTS (SELECT 1 FROM cash_flow_statements cf WHERE cf.sec_filing_id = sf.id)
    "#;

    let row = sqlx::query(query).fetch_one(pool).await?;
    let orphan_count: i64 = row.get("orphan_count");

    Ok(orphan_count)
}
```

**Action Items**:
- [ ] Add `cleanup_orphaned_sec_filings` function
- [ ] Add `verify_data_integrity` function
- [ ] Call verification after each stock processing (optional)
- [ ] Add startup integrity check in refresh_data.rs

### Phase 5: Testing Plan

#### Test 1: Clean Up Existing Orphans from AAPL Test
```bash
# Check current orphans
source /Users/yksoni/code/misc/rust-stocks/.env && sqlite3 "$DATABASE_PATH" "
SELECT COUNT(*) FROM sec_filings WHERE id >= 26010 AND id <= 26014;
"

# Delete them
source /Users/yksoni/code/misc/rust-stocks/.env && sqlite3 "$DATABASE_PATH" "
DELETE FROM sec_filings WHERE id >= 26010 AND id <= 26014;
"
```

#### Test 2: Success Path (WMT)
```bash
cd /Users/yksoni/code/misc/rust-stocks/src-tauri
cargo run --bin refresh_data financials --only-ticker WMT
```

**Expected**:
- ✅ "Atomically stored filing..." messages
- ✅ "Transaction committed successfully"
- ✅ All 3 statements stored for each filing

**Verify**:
```bash
source /Users/yksoni/code/misc/rust-stocks/.env && sqlite3 "$DATABASE_PATH" "
SELECT COUNT(*) as orphans FROM sec_filings sf
WHERE NOT EXISTS (SELECT 1 FROM balance_sheets bs WHERE bs.sec_filing_id = sf.id)
   OR NOT EXISTS (SELECT 1 FROM income_statements inc WHERE inc.sec_filing_id = sf.id)
   OR NOT EXISTS (SELECT 1 FROM cash_flow_statements cf WHERE cf.sec_filing_id = sf.id);
"
```
Should return: 0

#### Test 3: Failure Path (AAPL - has duplicates)
```bash
cd /Users/yksoni/code/misc/rust-stocks/src-tauri
cargo run --bin refresh_data financials --only-ticker AAPL
```

**Expected**:
- ✅ Process some filings successfully
- ❌ Hit duplicate constraint on one filing
- ✅ "ERROR: Failed to store filing, rolling back transaction"
- ✅ Previous successful transactions remain committed
- ✅ Failed transaction fully rolled back
- ✅ NO orphaned sec_filings created

**Verify**:
```bash
source /Users/yksoni/code/misc/rust-stocks/.env && sqlite3 "$DATABASE_PATH" "
SELECT COUNT(*) as orphans FROM sec_filings sf
WHERE NOT EXISTS (SELECT 1 FROM balance_sheets bs WHERE bs.sec_filing_id = sf.id)
   OR NOT EXISTS (SELECT 1 FROM income_statements inc WHERE inc.sec_filing_id = sf.id)
   OR NOT EXISTS (SELECT 1 FROM cash_flow_statements cf WHERE cf.sec_filing_id = sf.id);
"
```
Should return: 0

## Implementation Order

1. **Phase 1.1**: Add transaction variants of storage functions (sec_edgar_client.rs)
2. **Phase 1.2**: Add create_or_get_sec_filing_tx (sec_edgar_client.rs)
3. **Phase 2.1**: Add store_filing_atomic (data_freshness_checker.rs)
4. **Phase 3.1**: Add extract_and_store_all_statements_atomic (data_freshness_checker.rs)
5. **Phase 3.2**: Update call sites to use new extraction function
6. **Phase 4.1**: Add cleanup and verification functions
7. **Phase 5**: Test thoroughly

## Estimated Time

- Phase 1: 1-1.5 hours (storage function refactoring)
- Phase 2: 30 minutes (atomic storage function)
- Phase 3: 45 minutes (extraction logic refactoring)
- Phase 4: 20 minutes (cleanup functions)
- Phase 5: 30 minutes (testing)
- **Total**: ~3-3.5 hours

## Success Criteria

- [ ] All storage operations use transactions
- [ ] Automatic rollback on any failure
- [ ] Zero orphaned sec_filings after any operation (verified via SQL)
- [ ] Clear error messages for failures
- [ ] Tests pass for both success and failure scenarios
- [ ] Existing WMT test still works (no regressions)
- [ ] AAPL test creates no orphans (even when hitting duplicates)

## Rollback Strategy

If issues are found:
1. Git stash changes
2. Run cleanup script to remove any orphaned records
3. Review implementation
4. Fix issues and re-test
5. Git stash pop to continue

## Questions to Resolve Before Implementation

1. ✅ Should we fail the entire stock if one filing fails? **Yes - but continue with other stocks**
2. ✅ Should we log every transaction boundary? **Yes - helpful for debugging**
3. ✅ Should we add a --verify flag to check integrity? **Nice to have, not critical**
4. ✅ Should cleanup run automatically on startup? **Yes - defensive**

## Notes

- SQLite transactions are local (no distributed transactions needed)
- Transaction rollback is automatic when tx is dropped without commit
- UNIQUE constraints will still trigger errors - we just rollback cleanly now
- This fixes the data corruption issue but doesn't solve duplicate detection (that's a separate issue)
