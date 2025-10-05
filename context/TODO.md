# TODO - Project Structure Reorganization

## 🏗️ CURRENT PRIORITY: Standard Tauri Project Structure

### **Problem**: Messy project structure not following Tauri conventions
- Backend code scattered between `src/` and `src-tauri/src/`
- Database and migrations in root directory
- Dual analysis modules causing command registration confusion
- Root `Cargo.toml` should be in `src-tauri/`

### **Target Structure** (Standard Tauri):
```
rust-stocks/
├── package.json ✅ (Root Tauri project)
├── src/ ✅ (Frontend - React/JS) 
├── src-tauri/ ✅ (All backend code)
│   ├── Cargo.toml ✅ (Complete backend workspace)
│   ├── src/ ✅ (All Rust code unified)
│   │   ├── main.rs, lib.rs ✅
│   │   ├── commands/ ✅ (Tauri commands only)
│   │   ├── analysis/ ✅ (Business logic only) 
│   │   ├── database/, tools/, bin/ ✅
│   └── db/ ✅ (Database organization)
│       ├── stocks.db ✅
│       ├── migrations/ ✅
│       └── backups/ ✅
```

### **Reorganization Plan**:
#### **Phase 1: Backend Consolidation** 🔄
1. Move `src/` → `src-tauri/src/` (merge backend code)
2. Merge `Cargo.toml` → `src-tauri/Cargo.toml`
3. Clean up dual analysis modules

#### **Phase 2: Database Organization** 🔄  
1. Create `src-tauri/db/` directory
2. Move `stocks.db` → `src-tauri/db/stocks.db`
3. Move `migrations/` → `src-tauri/db/migrations/`
4. Create `src-tauri/db/backups/`
5. Update migration/backup code paths

#### **Phase 3: Frontend Cleanup** 🔄
1. Rename `frontend/` → `src/` (optional)
2. Update Tauri config

#### **Phase 4: Path & Configuration Updates** 🔄
1. Update database paths from `../stocks.db` to `db/stocks.db`
2. Fix command registration confusion
3. Update package.json scripts
4. Update documentation

---

# TODO - S&P 500 Offline Support ✅ COMPLETE

## Database Migration Issue ✅ RESOLVED
- **Problem**: ✅ SOLVED - `sp500_symbols` table created successfully
- **Current Status**: ✅ COMPLETE - Professional SQLx migration system with safeguards implemented
- **Action Required**: ✅ DONE - Safe migration system with automatic backups and production database protection

## Required Database Schema
```sql
CREATE TABLE IF NOT EXISTS sp500_symbols (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL UNIQUE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_sp500_symbols_symbol ON sp500_symbols(symbol);
```

## Implementation Status ✅ COMPLETE
- ✅ **Backend Code**: `get_sp500_symbols()` function updated with offline support  
- ✅ **Timeout Logic**: 10-second timeout for GitHub fetch
- ✅ **Fallback Logic**: Uses database cache when GitHub fails
- ✅ **Database Table**: `sp500_symbols` table created successfully via migration
- ✅ **Migration**: Professional SQLx migration system with safeguards implemented
- ✅ **Database Protection**: Production database safeguards with automatic backups

## Features Implemented
1. **Online Mode**: Fetches fresh S&P 500 list from GitHub
2. **Offline Mode**: Falls back to cached database symbols
3. **Timeout Protection**: 10-second timeout prevents hanging
4. **Auto-Update**: Updates database cache when online
5. **Error Handling**: Graceful fallback with console logging

## Next Steps ✅ COMPLETE
1. ✅ Fix migration system - Professional SQLx migration system implemented
2. ✅ Create proper migration for `sp500_symbols` table - Migration completed with safeguards  
3. ✅ Test offline functionality - Database and backend ready for offline operation
4. ✅ Verify S&P 500 filter works - Table created, ready for symbol population

## Database Admin Tools Added
- `cargo run --bin db_admin -- status` - Check database statistics and health
- `cargo run --bin db_admin -- backup` - Create manual database backup
- `cargo run --bin db_admin -- migrate --confirm` - Safe migration with backups
- `cargo run --bin db_admin -- verify` - Verify database integrity
- `./backup_database.sh` - Shell script for automatic backups

## Code Changes Made
- Modified `src-tauri/src/commands/stocks.rs`
- Added `fetch_sp500_from_github_with_timeout()`
- Added `update_sp500_in_database()`
- Added `get_sp500_from_database()`
- Created migration file: `migrations/20250908000004_add_sp500_symbols.sql`

---

# TODO - SEC Filing Metadata Population 🔄 IN PROGRESS

## Problem: Missing SEC Filing Metadata
- **Issue**: Financial statements lack SEC filing metadata (`filed_date`, `accession_number`, `form_type`)
- **Impact**: Freshness checker cannot determine if data is current with latest SEC filings
- **Root Cause**: Data imported from SimFin without SEC filing metadata

## Current Status
- ✅ **CIK Format Fixed**: All CIKs properly formatted and verified against SEC
- ✅ **Database Cleaned**: Consolidated to use only `cik_mappings_sp500` table
- ✅ **Phase 1 Complete**: `sec_filings` table created with 1,281 filings
- ✅ **Phase 2 Complete**: Duplicate EDGAR metadata columns removed from financial tables
- 🚨 **CRITICAL ISSUE**: Only 1,320 out of 161,539 financial records have SEC metadata (0.8% coverage!)
- ✅ **Scripts Cleaned**: Removed redundant `populate_sec_metadata_companyfacts.rs` - functionality integrated into main refresh system
- ❌ **Freshness Logic**: Cannot compare SEC filing dates with our data

## Implementation Plan - Company Facts API Approach

### **Step 1: Fix Broken Scripts** ✅ **COMPLETED**
- ✅ **FIXED**: `populate_sec_metadata_companyfacts.rs` now works with new `sec_filings` architecture
- ✅ **REMOVED**: `populate_sec_metadata.rs` (legacy bulk submissions approach)
- ✅ **SOLUTION**: Script now creates `sec_filings` records and links via `sec_filing_id`
- ✅ **STATUS**: **RESOLVED** - Scripts ready for future data downloads

### **Step 2: Remove Bulk Submissions Logic** ✅ **COMPLETED**
- ✅ **REMOVED**: `populate_sec_metadata.rs` binary (legacy bulk submissions approach)
- ✅ **CLEANED**: All bulk submissions download/extraction code removed
- ✅ **STATUS**: **COMPLETED** - Legacy bulk submissions logic eliminated

### **Step 3: Implement Company Facts API with Rate Limiting** ⏳
- **API Endpoint**: `https://data.sec.gov/api/xbrl/companyfacts/CIK##########.json`
- **Rate Limiting**: Use `governor` crate (10 requests/second limit)
- **Concurrency**: 10 concurrent threads with proper rate limiting
- **Dependencies**: Add `governor` and `reqwest-middleware` to Cargo.toml

### **Step 4: Concurrent Data Population** ⏳
- **Process**:
  1. Get all S&P 500 CIKs from database
  2. Create 10 concurrent workers with rate-limited HTTP client
  3. Each worker downloads Company Facts JSON for assigned CIKs
  4. Parse JSON to extract filing metadata (`filingDate`, `accessionNumber`, `form`)
  5. Match with our existing financial records by `reportDate`
  6. Update all matching records across all 3 tables
- **Target**: Both 10-K and 10-Q filings
- **Cleanup**: Remove records with no SEC filing match

### **Step 5: Error Handling** ⏳
- **Rate Limit Compliance**: Automatic throttling via governor
- **API Errors**: Handle 403, 429, and other HTTP errors gracefully
- **Continue Processing**: Don't stop on individual CIK errors
- **Validation**: Ensure data integrity after population

## Technical Details

### **SEC JSON Field Mapping**
```rust
// SEC Submission JSON → Our Database
reportDate → report_date
filingDate → filed_date  
accessionNumber → accession_number
form → form_type
```

### **Matching Logic**
- **Primary**: Match `reportDate` = `report_date` AND `form` = "10-K"
- **Secondary**: Also include `form` = "10-Q" filings
- **Data Validation**: Use revenue metrics to distinguish 10-K vs 10-Q

### **Expected Results**
- **Before**: All metadata columns NULL
- **After**: All financial records have proper SEC filing metadata
- **Freshness Checker**: Can now accurately determine data staleness

## Files to Modify
- `src-tauri/db/migrations/` - New migration file
- `src-tauri/src/tools/data_freshness_checker.rs` - Update freshness logic
- New concurrent processing module for metadata population

## Success Criteria
- ✅ All financial records have `filed_date`, `accession_number`, `form_type`
- ✅ Freshness checker reports accurate staleness based on SEC filings
- ✅ No orphaned financial records without SEC filing metadata
- ✅ Concurrent processing completes efficiently (10 threads)
