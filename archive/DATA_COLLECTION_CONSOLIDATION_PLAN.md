# Data Collection Consolidation - Detailed Implementation Plan

## Overview
Consolidate duplicate single stock and concurrent stock fetching logic into a unified architecture.

## Pre-Implementation Checklist

### Test Compatibility Verification
- [x] Identify all test binaries that use DataCollector functions
- [ ] Document current test binary function calls  
- [ ] Ensure test coverage won't be broken
- [ ] Plan test binary updates

### Current State Analysis
- [x] Located duplicate batching logic in `run_single_stock_collection` (data_collection_new.rs:680-796)
- [x] Identified `fetch_stock_history()` (data_collector.rs:189-243) - simple, no batching
- [x] Identified `fetch_stock_history_with_batching_ref()` (data_collector.rs:249-332) - with batching  
- [x] Found concurrent fetcher uses batching version
- [x] Confirmed UI logging duplication between single/concurrent flows

## Step-by-Step Implementation

### Step 1: Create Unified Configuration ⏳ NEXT
**File:** `src/concurrent_fetcher.rs`
**Action:** Replace `ConcurrentFetchConfig` with `UnifiedFetchConfig`

```rust
#[derive(Debug, Clone)]
pub struct UnifiedFetchConfig {
    pub stocks: Vec<Stock>,           // Single=[selected], All=get_active_stocks()  
    pub date_range: DateRange,
    pub num_threads: usize,           // Single=1, Concurrent=5-10
    pub retry_attempts: u32,
    pub rate_limit_ms: u64,
    pub max_stocks: Option<usize>,    // For testing limits
}
```

**Changes Required:**
- Replace `ConcurrentFetchConfig` with `UnifiedFetchConfig` 
- Add `stocks: Vec<Stock>` field (replaces getting stocks inside function)
- Add `rate_limit_ms: u64` field
- Update all references throughout file

### Step 2: Rename & Update Core Function
**File:** `src/concurrent_fetcher.rs`
**Action:** Rename and update main function signature

```rust
// OLD
pub async fn fetch_stocks_concurrently_with_logging(
    database: Arc<DatabaseManagerSqlx>,
    config: ConcurrentFetchConfig,
    global_broadcast_sender: Option<Arc<broadcast::Sender<StateUpdate>>>,
) -> Result<FetchResult>

// NEW  
pub async fn fetch_stocks_unified_with_logging(
    database: Arc<DatabaseManagerSqlx>,
    config: UnifiedFetchConfig,
    global_broadcast_sender: Option<Arc<broadcast::Sender<StateUpdate>>>,
) -> Result<FetchResult>
```

**Implementation Changes:**
- Remove `database.get_active_stocks().await?` - use `config.stocks` instead
- Update `total_stocks` calculation to use `config.stocks.len()`
- Pass `config.stocks` to worker threads instead of loading stocks

### Step 3: Update Single Stock UI
**File:** `src/ui/data_collection_new.rs`
**Action:** Replace manual batching with unified fetcher call

**Remove (~120 lines):**
- Manual batch calculation: `TradingWeekBatchCalculator::calculate_batches()`
- Batch processing loop with custom TUI logging
- Individual `DataCollector::fetch_stock_history()` calls per batch
- Custom delay and progress tracking

**Replace with (~20 lines):**
```rust
async fn run_single_stock_collection(&mut self, stock: Stock, start_date: NaiveDate, end_date: NaiveDate) {
    let operation_id = format!("single_stock_{}", stock.symbol);
    let _ = self.state_manager.start_operation(operation_id.clone(), format!("Single Stock Collection: {}", stock.symbol), true);
    
    let mut state_manager = self.state_manager.clone();
    let global_broadcast_sender = self.global_broadcast_sender.clone().expect("Global broadcast sender not set");
    let database = self.database.clone().expect("Database not available");
    
    tokio::spawn(async move {
        use crate::concurrent_fetcher::{UnifiedFetchConfig, DateRange, fetch_stocks_unified_with_logging};
        
        let config = UnifiedFetchConfig {
            stocks: vec![stock.clone()],
            date_range: DateRange { start_date, end_date },
            num_threads: 1,
            retry_attempts: 3,
            rate_limit_ms: 500,
            max_stocks: None,
        };

        match fetch_stocks_unified_with_logging(database, config, Some(Arc::new(global_broadcast_sender))).await {
            Ok(result) => {
                let success_message = format!(
                    "✅ Single stock collection completed! Records: {}",
                    result.total_records_fetched
                );
                let _ = state_manager.complete_operation(&operation_id, Ok(success_message));
            }
            Err(e) => {
                let error_message = format!("❌ Single stock collection failed: {}", e);
                let _ = state_manager.complete_operation(&operation_id, Err(error_message));
            }
        }
    });
}
```

### Step 4: Update Concurrent Stock UI  
**File:** `src/ui/data_collection_new.rs`
**Action:** Update to use new unified function name and config

**Changes:**
- Replace `ConcurrentFetchConfig` with `UnifiedFetchConfig`
- Add `stocks: database.get_active_stocks().await?` to config
- Replace `fetch_stocks_concurrently_with_logging` with `fetch_stocks_unified_with_logging`
- Add `rate_limit_ms: 500` to config

### Step 5: Remove Obsolete DataCollector Function
**File:** `src/data_collector.rs` 
**Action:** Remove `fetch_stock_history()` function (lines 189-243)

**Verification:**
- Ensure no other code calls this function
- Update any remaining references to use the batching version

### Step 6: Update Test Binaries
**Files:** `tests/bin/data_collection_test.rs`, others
**Action:** Replace DataCollector function calls

```rust
// OLD
match DataCollector::fetch_stock_history(
    Arc::new(client),
    database.clone(),
    stock.clone(),
    start_date,
    end_date
).await

// NEW  
match fetch_stocks_unified_with_logging(
    database.clone(),
    UnifiedFetchConfig {
        stocks: vec![stock.clone()],
        date_range: DateRange { start_date, end_date },
        num_threads: 1,
        retry_attempts: 3,
        rate_limit_ms: 1000, // More conservative for tests
        max_stocks: None,
    },
    None, // No TUI logging in tests
).await
```

### Step 7: Update Import Statements
**Files:** All files importing concurrent_fetcher functions
**Action:** Update import statements

```rust
// OLD
use crate::concurrent_fetcher::{ConcurrentFetchConfig, DateRange, fetch_stocks_concurrently_with_logging};

// NEW
use crate::concurrent_fetcher::{UnifiedFetchConfig, DateRange, fetch_stocks_unified_with_logging};
```

### Step 8: Update Legacy Function Wrapper
**File:** `src/concurrent_fetcher.rs`
**Action:** Update `fetch_stocks_concurrently()` to use new unified function

```rust
pub async fn fetch_stocks_concurrently(
    database: Arc<DatabaseManagerSqlx>,
    config: UnifiedFetchConfig, // Changed from ConcurrentFetchConfig
) -> Result<FetchResult> {
    fetch_stocks_unified_with_logging(database, config, None).await
}
```

## Verification Steps

### Functional Testing
1. **Single Stock Collection:**
   - [ ] Select single stock from UI
   - [ ] Verify correct batching behavior  
   - [ ] Confirm TUI logs appear correctly
   - [ ] Check data is inserted properly

2. **Concurrent Collection:**
   - [ ] Select "all stocks" from UI
   - [ ] Verify multi-threading works
   - [ ] Confirm progress logs from different threads
   - [ ] Check performance is maintained

3. **Test Binaries:**
   - [ ] `cargo run --bin data_collection_test detailed -s 20240101 -e 20240102`
   - [ ] `cargo run --bin data_collection_test single AAPL 20240101 20240102`
   - [ ] Verify output matches previous behavior

### Performance Testing
- [ ] Compare single stock collection time before/after
- [ ] Compare concurrent collection time before/after  
- [ ] Verify memory usage is similar
- [ ] Check no regression in throughput

### Error Handling Testing
- [ ] Test with invalid stock symbol
- [ ] Test with network failures
- [ ] Test with date range issues
- [ ] Verify error messages are consistent

## Risk Mitigation

### Backup Strategy
- [ ] Create git commit before starting consolidation
- [ ] Test each step incrementally
- [ ] Keep rollback plan ready

### Testing Strategy  
- [ ] Run full test suite after each major step
- [ ] Test both UI flows manually
- [ ] Verify test binaries still work
- [ ] Check edge cases and error conditions

## Success Criteria

### Code Reduction
- [ ] ~200 lines removed from `run_single_stock_collection`
- [ ] ~60 lines removed from `fetch_stock_history()`
- [ ] Single unified function handles all fetching

### Functional Equivalence
- [ ] UI behavior identical to users
- [ ] Same performance characteristics
- [ ] Same error handling and recovery
- [ ] Test binaries continue to work

### Architecture Improvement
- [ ] Clear separation of concerns
- [ ] No code duplication between single/concurrent
- [ ] Consistent logging and error handling
- [ ] Easier to maintain and extend

## Expected Timeline
- **Step 1-2:** 30 minutes (Config and function updates)
- **Step 3-4:** 45 minutes (UI updates and testing)
- **Step 5-6:** 30 minutes (Cleanup and test binary updates)  
- **Step 7-8:** 15 minutes (Import and wrapper updates)
- **Verification:** 45 minutes (Testing both UI flows)
- **Total:** ~3 hours

## Post-Implementation
- [ ] Update architecture.md with new simplified flow
- [ ] Document new unified configuration options
- [ ] Consider adding integration tests for unified fetcher
- [ ] Plan future enhancements now that code is consolidated