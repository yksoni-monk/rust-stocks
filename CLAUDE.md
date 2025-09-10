# 🚨 CLAUDE: START HERE EVERY TIME

## PRODUCTION DATABASE (NEVER FORGET THIS!)
- **Location**: `src-tauri/db/stocks.db` (in src-tauri/db/ directory)
- **Size**: 2.5GB 
- **Data**: 5,892 stocks, 6.2M daily prices, 54K TTM financials
- **Status**: PRODUCTION - contains all your valuable data

## WORKING DIRECTORY 
**ALWAYS**: `/Users/yksoni/code/misc/rust-stocks` (ROOT)

## PROJECT STRUCTURE (Standard Tauri)
```
/Users/yksoni/code/misc/rust-stocks/     ← YOU ARE HERE (ROOT)
├── src/                                 ← REACT FRONTEND (React/JS components)
│   ├── src/
│   │   ├── App.jsx, main.jsx
│   │   ├── components/
│   │   └── services/                    ← API & data services layer
├── src-tauri/                          ← RUST BACKEND (All Rust code)
│   ├── Cargo.toml                      ← Backend Cargo.toml
│   ├── src/                            ← Rust backend code
│   │   ├── commands/, analysis/, tools/, bin/
│   └── db/                             ← Database organization
│       ├── stocks.db                   ← 2.5GB PRODUCTION DATABASE
│       ├── migrations/                 ← Database migrations
│       └── backups/                    ← Database backups
└── context/                            ← Project documentation
```

## 📚 BROADER PROJECT CONTEXT
**For detailed project architecture, design decisions, and comprehensive documentation:**
👉 **Check the `context/` folder** - contains full project context, architecture plans, and historical documentation

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

# Tauri Desktop App
npm run tauri dev  # (from src-tauri/ directory)
```

## DATABASE STRUCTURE
- `stocks`: 5,892 companies
- `daily_prices`: 6.2M price records  
- `income_statements`: 54K TTM financial records
- `balance_sheets`: 54K TTM balance sheets
- `daily_valuation_ratios`: 3,294 calculated P/S and EV/S ratios

## CRITICAL REMINDERS
- Working directory is ROOT: `/Users/yksoni/code/misc/rust-stocks`
- Frontend is in `src/` (React components)
- Backend is in `src-tauri/src/` (Rust code)
- Database is in `src-tauri/db/stocks.db`
- Migrations are in `src-tauri/db/migrations/`
- Backups are in `src-tauri/db/backups/`

## NEVER DO THESE THINGS
- ❌ Look for database in ROOT - it's in `src-tauri/db/stocks.db`
- ❌ Put migrations in root - they belong in `src-tauri/db/migrations/`
- ❌ Confuse frontend (src/) with backend (src-tauri/src/)
- ❌ Create databases or migrations outside proper directories

## RECENT WORK COMPLETED
1. ✅ Multi-period database schema (income_statements, balance_sheets, daily_valuation_ratios)
2. ✅ TTM data import system (54K records imported successfully)  
3. ✅ P/S and EV/S ratio calculation engine (3,294 ratios calculated)
4. ✅ Root-level binary organization (clean structure)
5. ✅ Production database migration completed

## NEXT STEPS AVAILABLE
- Frontend integration to display P/S/EV/S ratios in UI
- Stock screening tools using P/S < 1.0 for undervalued stocks
- Historical ratio trend analysis
- Enhanced negative earnings stock analysis