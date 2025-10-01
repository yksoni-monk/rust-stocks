# 🚨 CLAUDE: START HERE EVERY TIME

## PRODUCTION DATABASE (NEVER FORGET THIS!)
- **Location**: `src-tauri/db/stocks.db` (in src-tauri/db/ directory)
- **DATABASE_URL**: `sqlite:/Users/yksoni/code/misc/rust-stocks/src-tauri/db/stocks.db` (ABSOLUTE PATH REQUIRED)
- **Size**: 2.5GB
- **Data**: 5,892 stocks, 6.2M daily prices, 54K TTM financials
- **Status**: PRODUCTION - contains all your valuable data

## WORKING DIRECTORY 
**ALWAYS**: `/Users/yksoni/code/misc/rust-stocks` (ROOT)

## PROJECT STRUCTURE (Standard Tauri)
```
/Users/yksoni/code/misc/rust-stocks/     ← PROJECT_ROOT (Environment Variable)
├── src/                                 ← SOLIDJS FRONTEND (TypeScript/SolidJS)
│   ├── src/
│   │   ├── App.tsx, main.tsx           ← SolidJS entry points
│   │   ├── components/                 ← UI components (TSX)
│   │   ├── stores/                     ← Signal-based state management
│   │   ├── services/                   ← API & data services layer (TypeScript)
│   │   └── utils/                      ← TypeScript types and utilities
├── src-tauri/                          ← RUST BACKEND (All Rust code)
│   ├── Cargo.toml                      ← Backend Cargo.toml
│   ├── src/                            ← Rust backend code
│   │   ├── commands/, analysis/, tools/, bin/
│   └── db/                             ← Database organization
│       ├── stocks.db                   ← 2.5GB PRODUCTION DATABASE
│       ├── migrations/                 ← Database migrations
│       └── backups/                    ← Database backups
├── edgar_data/                         ← EDGAR SEC DATA
│   └── companyfacts/                   ← EDGAR company JSON files (CIK*.json)
└── context/                            ← Project documentation
```

## PROJECT PATHS (Critical for File Access)
- **PROJECT_ROOT**: `/Users/yksoni/code/misc/rust-stocks` (in .env file)
- **RUST_BASE**: `$PROJECT_ROOT/src-tauri`
- **DATABASE**: `$PROJECT_ROOT/src-tauri/db/stocks.db`
- **EDGAR_DATA**: `$PROJECT_ROOT/edgar_data/companyfacts/` (CIK*.json files)

## 📚 BROADER PROJECT CONTEXT
**For detailed project architecture, design decisions, and comprehensive documentation:**
👉 **Check the `context/` folder** - contains full project context, architecture plans, and historical documentation

## 🚀 FRONTEND MIGRATION (SEPTEMBER 2025)
✅ **MIGRATED FROM REACT TO SOLIDJS** - Successfully solved infinite re-rendering issues
- **Previous Problem**: React RecommendationsPanel had infinite loops, GARP screening broken
- **Solution**: Migrated entire frontend to SolidJS with signal-based reactivity
- **Result**: 50% smaller bundle, fine-grained updates, GARP screening works perfectly
- **Architecture**: Store-based state management with TypeScript throughout
- **Documentation**: `context/SOLIDJS_FRONTEND_ARCHITECTURE.md` and `context/FRONTEND_MIGRATION_HISTORY.md`

## 🧪 BACKEND TESTING STRATEGY
**Backend testing with production database:**
- ✅ **13 Commands**: All frontend API calls identified and tested
- ✅ **Self-Contained Tests**: Focus only on code called by UI
- ✅ **Test Database**: Isolated test DB with production data copy
- ✅ **All Tests Passing**: 16/16 backend tests pass with 2.7GB database

## P/S & EV/S RATIO SYSTEM STATUS
✅ **FULLY OPERATIONAL**
- 54,160 TTM financial statements imported
- 3,294 P/S and EV/S ratios calculated  
- 96.4% data completeness
- Solves the problem: P/E ratios invalid when earnings are negative

## AVAILABLE COMMANDS (run from ROOT)
```bash
# From ROOT directory only:
cargo run --bin import-ttm                             # TTM Financial Data Import
cargo run --bin calculate-ratios                       # Calculate all P/S & EV/S ratios
cargo run --bin calculate-ratios --report              # Generate report only
cargo run --bin calculate-ratios --negative-earnings   # Focus on negative earnings stocks

# SolidJS Frontend Development
cd src && npm run dev                                   # SolidJS development server
cd src && npm run build                                 # Production build

# Tauri Desktop App (from ROOT)
npm run tauri dev                                       # Desktop app with SolidJS frontend
```

## DATABASE STRUCTURE
- `stocks`: 5,892 companies
- `daily_prices`: 6.2M price records  
- `income_statements`: 54K TTM financial records
- `balance_sheets`: 54K TTM balance sheets
- `daily_valuation_ratios`: 3,294 calculated P/S and EV/S ratios

## CRITICAL REMINDERS
- Working directory is ROOT: `/Users/yksoni/code/misc/rust-stocks`
- Frontend is in `src/` (SolidJS TypeScript components)
- Backend is in `src-tauri/src/` (Rust code)
- Database is in `src-tauri/db/stocks.db`
- Migrations are in `src-tauri/db/migrations/`
- Backups are in `src-tauri/db/backups/`

## 🛡️ ENGINEERING DISCIPLINE RULES (MANDATORY)
**After major API inconsistency bugs, these rules are NON-NEGOTIABLE:**

### **API Contract Discipline**
1. **NEVER** modify backend function signatures without checking all frontend callers
2. **ALWAYS** use consistent naming: `snake_case` in Rust, map to `camelCase` in frontend
3. **ALWAYS** validate parameter names match between frontend invoke() and backend function
4. **ALWAYS** verify return types match between frontend expectations and backend reality

### **Testing Requirements**
5. **ALWAYS** run integration tests for API changes: `cargo test && cd src && npm test`
6. **NEVER** claim something is "fixed" without testing the actual user flow
7. **ALWAYS** test frontend-backend communication end-to-end

### **Schema Consistency**
8. **ALWAYS** make database field names match API struct field names exactly
9. **NEVER** use different field names between database schema and API models
10. **ALWAYS** use proper serde mapping for camelCase conversion

### **Before Any API Change Checklist**
- [ ] Check frontend API calls match backend function signatures
- [ ] Verify parameter names are consistent (startDate → start_date mapping)
- [ ] Confirm return types match frontend expectations
- [ ] Test the actual user flow, not just unit tests
- [ ] Update TypeScript interfaces if backend structs change

**Violation of these rules wastes user time and is unacceptable.**

## NEVER DO THESE THINGS
- ❌ Look for database in ROOT - it's in `src-tauri/db/stocks.db`
- ❌ Put migrations in root - they belong in `src-tauri/db/migrations/`
- ❌ Confuse frontend (src/) with backend (src-tauri/src/)
- ❌ Create databases or migrations outside proper directories
- ❌ Change API contracts without checking both sides
- ❌ Use different field names between database and API models

## RECENT WORK COMPLETED
1. ✅ Multi-period database schema (income_statements, balance_sheets, daily_valuation_ratios)
2. ✅ TTM data import system (54K records imported successfully)  
3. ✅ P/S and EV/S ratio calculation engine (3,294 ratios calculated)
4. ✅ Root-level binary organization (clean structure)
5. ✅ Production database migration completed
6. ✅ **Frontend migration to SolidJS** (September 2025) - Solved React infinite loops
7. ✅ **GARP screening fully functional** - All screening algorithms working

## CURRENT STATUS
- **Frontend**: ✅ SolidJS with TypeScript, signal-based state management
- **Backend**: ✅ All 16 tests passing, 13 Tauri commands operational
- **Database**: ✅ 2.5GB production database with 96.4% data completeness
- **Screening**: ✅ GARP, P/S, P/E algorithms all working perfectly
- **Performance**: ✅ 50% smaller bundle, eliminated re-rendering issues
- Stock screening tools using P/S < 1.0 for undervalued stocks
- Historical ratio trend analysis
- Enhanced negative earnings stock analysis