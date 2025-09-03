# SQLX Migration - Batch Import Fix Script

## 🎯 Purpose
This script will systematically fix all import issues by replacing `crate::database::DatabaseManager` with `crate::database_sqlx::DatabaseManagerSqlx` in all affected files.

## 📋 Files to Fix
1. src/analysis/mod.rs
2. src/concurrent_fetcher.rs
3. src/data_collector.rs
4. src/ui/dashboard.rs
5. src/ui/data_analysis.rs
6. src/ui/app.rs
7. src/ui/data_collection.rs

## 🔧 Fix Strategy
- Replace import statements
- Comment out database operations temporarily
- Add TODO comments for async conversion
- Focus on compilation first, functionality later

## 📝 Implementation Notes
- Keep git commits after each file
- Test compilation after each fix
- Maintain working state throughout process
