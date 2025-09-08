# TODO - S&P 500 Offline Support

## Database Migration Issue
- **Problem**: Need to create `sp500_symbols` table for offline S&P 500 support
- **Current Status**: Migration system has issues (sector column error)
- **Action Required**: Fix migration system or create proper migration for `sp500_symbols` table

## Required Database Schema
```sql
CREATE TABLE IF NOT EXISTS sp500_symbols (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL UNIQUE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_sp500_symbols_symbol ON sp500_symbols(symbol);
```

## Implementation Status
- ✅ **Backend Code**: `get_sp500_symbols()` function updated with offline support
- ✅ **Timeout Logic**: 10-second timeout for GitHub fetch
- ✅ **Fallback Logic**: Uses database cache when GitHub fails
- ❌ **Database Table**: `sp500_symbols` table needs to be created via migration
- ❌ **Migration**: Migration system needs to be fixed

## Features Implemented
1. **Online Mode**: Fetches fresh S&P 500 list from GitHub
2. **Offline Mode**: Falls back to cached database symbols
3. **Timeout Protection**: 10-second timeout prevents hanging
4. **Auto-Update**: Updates database cache when online
5. **Error Handling**: Graceful fallback with console logging

## Next Steps
1. Fix migration system (sector column error)
2. Create proper migration for `sp500_symbols` table
3. Test offline functionality
4. Verify S&P 500 filter works with real ~500 symbols

## Code Changes Made
- Modified `src-tauri/src/commands/stocks.rs`
- Added `fetch_sp500_from_github_with_timeout()`
- Added `update_sp500_in_database()`
- Added `get_sp500_from_database()`
- Created migration file: `migrations/20250908000004_add_sp500_symbols.sql`
