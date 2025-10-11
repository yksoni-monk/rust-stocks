# 🚨 CLAUDE: QUICK REFERENCE

## PRODUCTION DATABASE (NEVER FORGET!)
- **Location**: `src-tauri/db/stocks.db` (2.5GB PRODUCTION DATABASE)
- **Size**: 2.5GB with S&P 500 financial data
- **Status**: PRODUCTION - contains all valuable data

## WORKING DIRECTORY 
**ALWAYS**: `/Users/yksoni/code/misc/rust-stocks` (ROOT)

## PROJECT STRUCTURE
```
/Users/yksoni/code/misc/rust-stocks/     ← PROJECT_ROOT
├── src/                                 ← SOLIDJS FRONTEND (TypeScript)
├── src-tauri/                          ← RUST BACKEND
│   └── db/                             ← Database
│       ├── stocks.db                   ← 2.5GB PRODUCTION DATABASE
│       ├── migrations/                 ← Database migrations
│       └── backups/                    ← Database backups
└── context/                            ← Project documentation
```

## CURRENT STATUS
- **Frontend**: ✅ SolidJS with TypeScript, signal-based state management
- **Backend**: ✅ Rust with Tauri framework
- **Database**: ✅ 2.5GB production database with S&P 500 data
- **Screening**: ✅ Piotroski F-Score and O'Shaughnessy Value algorithms
- **Data Source**: ✅ SEC EDGAR API integration

## AVAILABLE COMMANDS (run from ROOT)
```bash
# Desktop application
npm run tauri dev

# Database admin
cargo run --bin db_admin -- status
cargo run --bin db_admin -- backup

# Data refresh
cargo run --bin refresh_data -- --help
```

## 🔧 DATABASE MIGRATIONS (CRITICAL)
**⚠️ We use a CUSTOM Rust migration tool, NOT sqlx directly!**

**Environment setup (REQUIRED):**
```bash
export PROJECT_ROOT=/Users/yksoni/code/misc/rust-stocks
cd $PROJECT_ROOT/src-tauri
```

**Migration commands:**
```bash
# Create new migration
cargo run --bin migrate -- create <description>

# Apply migrations
cargo run --bin migrate -- run

# Check status
cargo run --bin migrate -- status

# Revert last migration
cargo run --bin migrate -- revert
```

**Database admin:**
```bash
# Check database status (ALWAYS run first)
cargo run --bin db_admin -- status

# Run migrations with safety (production)
cargo run --bin db_admin -- migrate --confirm
```

## CRITICAL REMINDERS
- Working directory is ROOT: `/Users/yksoni/code/misc/rust-stocks`
- Frontend is in `src/` (SolidJS TypeScript components)
- Backend is in `src-tauri/src/` (Rust code)
- Database is in `src-tauri/db/stocks.db`
- Migrations are in `src-tauri/db/migrations/`
- Backups are in `src-tauri/db/backups/`

## NEVER DO THESE THINGS
- ❌ Look for database in ROOT - it's in `src-tauri/db/stocks.db`
- ❌ Put migrations in root - they belong in `src-tauri/db/migrations/`
- ❌ Confuse frontend (src/) with backend (src-tauri/src/)
- ❌ Create databases or migrations outside proper directories
- ❌ Change API contracts without checking both sides
- ❌ Use different field names between database and API models
- ❌ Use old `sqlx migrate` commands - we have a custom tool now
- ❌ Run migrations without setting PROJECT_ROOT environment variable
- ❌ Modify migration files after they've been applied

## 📚 DETAILED DOCUMENTATION
- **Architecture**: `context/ARCHITECTURE.md`
- **Project Rules**: `.cursor/rules/projectrule.mdc`
- **README**: `README.md` (comprehensive guide)