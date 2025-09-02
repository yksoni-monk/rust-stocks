# SQLX Migration - Import Fix Plan

## 🎯 Current Status
- ✅ SQLX dependencies added
- ✅ Rusqlite removed
- ❌ 8 files have unresolved imports for `crate::database::DatabaseManager`

## 📋 Files Requiring Import Fixes

### **Priority 1: Core Application Files**
1. `src/main.rs` - Main application entry point
2. `src/data_collector.rs` - Data collection workflows
3. `src/concurrent_fetcher.rs` - Concurrent operations

### **Priority 2: UI Components**
4. `src/ui/app.rs` - Main TUI application
5. `src/ui/data_collection.rs` - Data collection UI
6. `src/ui/dashboard.rs` - Dashboard UI
7. `src/ui/data_analysis.rs` - Analysis UI

### **Priority 3: Analysis & Utilities**
8. `src/analysis/mod.rs` - Analysis operations

## 🔧 Fix Strategy

### **Phase A: Temporary Import Fixes**
- Replace `use crate::database::DatabaseManager;` with `use crate::database_sqlx::DatabaseManagerSqlx;`
- Comment out all database operations temporarily
- Focus on getting compilation working first

### **Phase B: Method Signature Updates**
- Update all method calls to use async versions
- Add `.await` to all database operations
- Handle async context properly

### **Phase C: Full Integration**
- Implement proper error handling
- Add proper async context management
- Test all functionality

## 📝 Implementation Plan

### **Step 1: Fix Core Files First**
1. `src/main.rs` - Update database initialization
2. `src/data_collector.rs` - Update data collection workflows
3. `src/concurrent_fetcher.rs` - Update concurrent operations

### **Step 2: Fix UI Components**
1. `src/ui/app.rs` - Update main application
2. `src/ui/data_collection.rs` - Update data collection UI
3. `src/ui/dashboard.rs` - Update dashboard
4. `src/ui/data_analysis.rs` - Update analysis UI

### **Step 3: Fix Analysis**
1. `src/analysis/mod.rs` - Update analysis operations

## 🚨 Risk Mitigation
- Keep git commits after each file fix
- Test compilation after each file
- Maintain working state throughout process

## 📈 Success Criteria
- [ ] All files compile without import errors
- [ ] Basic functionality can be tested
- [ ] No runtime errors from import fixes
