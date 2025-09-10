# Backend Test Plan

## Overview
This document outlines comprehensive testing for all backend functions called by the frontend. Tests focus only on code paths used by the UI to avoid testing dead code.

## Frontend API Analysis

### ✅ **IMPLEMENTED & USED BY FRONTEND** (13 commands)

#### **Stock Operations** (4 commands)
1. **`get_stocks_paginated(limit, offset)`** - Core pagination for main stock list
   - **Used by**: `App.jsx` → `stockDataService.loadInitialStockData()`, `loadMoreStocks()`
   - **Test Priority**: HIGH - Critical for main UI functionality
   - **Test Scenarios**: Empty DB, pagination boundaries, invalid limits

2. **`get_stocks_with_data_status()`** - Get all stocks with data availability flags  
   - **Used by**: `App.jsx` → `stockDataService.loadInitialStockData()` for total count
   - **Test Priority**: HIGH - Used for stock count display
   - **Test Scenarios**: Various data completeness scenarios

3. **`search_stocks(query)`** - Real-time stock search functionality
   - **Used by**: `App.jsx` → `stockDataService.searchStocks()` 
   - **Test Priority**: MEDIUM - Search feature
   - **Test Scenarios**: Partial matches, case insensitivity, empty results

4. **`get_sp500_symbols()`** - S&P 500 filtering support
   - **Used by**: `App.jsx` → `stockDataService.loadSp500Symbols()`, filtering
   - **Test Priority**: HIGH - Core filtering feature
   - **Test Scenarios**: Online/offline modes, timeout handling

#### **Analysis Operations** (5 commands)  
5. **`get_stock_date_range(symbol)`** - Date range for stock data
   - **Used by**: `AnalysisPanel.jsx` → `analysisDataService.loadStockAnalysis()`
   - **Test Priority**: MEDIUM - Analysis panel functionality
   - **Test Scenarios**: Valid symbols, non-existent symbols

6. **`get_price_history(symbol, start_date, end_date)`** - Historical price data
   - **Used by**: `AnalysisPanel.jsx` → `analysisDataService.loadStockAnalysis()`
   - **Test Priority**: HIGH - Core analysis functionality  
   - **Test Scenarios**: Various date ranges, invalid dates, missing data

7. **`get_valuation_ratios(symbol)`** - P/S, EV/S ratio display
   - **Used by**: `AnalysisPanel.jsx` → `analysisDataService.loadStockAnalysis()`
   - **Test Priority**: HIGH - Key valuation metrics feature
   - **Test Scenarios**: Stocks with/without ratios, data completeness

8. **`get_ps_evs_history(symbol, start_date, end_date)`** - Historical P/S & EV/S data
   - **Used by**: `AnalysisPanel.jsx` → `analysisDataService.loadPsEvsHistory()`
   - **Test Priority**: HIGH - Advanced analysis charts
   - **Test Scenarios**: Date range validation, chart data formats

9. **`export_data(symbol, format)`** - Data export functionality  
   - **Used by**: `AnalysisPanel.jsx` → `analysisDataService.exportStockData()`
   - **Test Priority**: LOW - Export feature
   - **Test Scenarios**: CSV/JSON formats, export validation

#### **Recommendations Operations** (2 commands)
10. **`get_undervalued_stocks_by_ps(max_ps_ratio, limit)`** - P/S ratio screening
    - **Used by**: `RecommendationsPanel.jsx` → `recommendationsDataService.loadUndervaluedStocksByPs()`
    - **Test Priority**: HIGH - Key value screening feature
    - **Test Scenarios**: Various P/S thresholds, result limits

11. **`get_value_recommendations_with_stats(limit)`** - P/E based recommendations
    - **Used by**: `RecommendationsPanel.jsx` → `recommendationsDataService.loadValueRecommendations()`
    - **Test Priority**: HIGH - Core recommendations functionality
    - **Test Scenarios**: Recommendation scoring, statistics calculation

#### **System Operations** (2 commands)
12. **`get_initialization_status()`** - System status for UI
    - **Used by**: `App.jsx` → `systemDataService.loadInitializationStatus()`
    - **Test Priority**: LOW - Status display
    - **Test Scenarios**: Various system states

13. **`get_database_stats()`** - Database statistics display
    - **Used by**: `DataFetchingPanel.jsx` → `systemDataService.loadDatabaseStats()`
    - **Test Priority**: MEDIUM - Database status panel
    - **Test Scenarios**: Stats calculation accuracy

### ❌ **IMPLEMENTED BUT NOT USED** (Dead Code - Skip Testing)

#### **Legacy/Unused Commands** (10+ commands)
- `get_all_stocks()` - Not called by frontend
- `test_alpha_vantage_earnings()` - Test command only  
- `test_alpha_vantage_daily()` - Test command only
- `calculate_daily_pe_ratio()` - Test command only
- `get_available_stock_symbols()` - Legacy, not used
- `fetch_*()` commands - Legacy data fetching system
- `analyze_sp500_pe_values()` - Not exposed in UI
- `get_recommendation_stats()` - Not used separately 
- `analyze_stock_pe_history()` - Not exposed in UI

### 🚧 **MISSING IMPLEMENTATIONS** (Frontend expects but not implemented)

Currently, all frontend API calls have corresponding backend implementations. No mock APIs needed.

## Test Implementation Strategy

### **Test Structure**
```
src-tauri/tests/
├── integration_tests.rs        # Main test file
├── fixtures/
│   ├── test_database.db        # Minimal test database
│   └── sample_data.sql         # Test data setup
└── helpers/
    ├── database_setup.rs       # Test DB utilities
    ├── mock_data.rs           # Test data generation
    └── assertions.rs          # Custom test assertions
```

### **Test Database Strategy**
- **Isolated Test DB**: Create `src-tauri/tests/fixtures/test_database.db`
- **Minimal Dataset**: ~10 stocks, 100 price records, 20 financial records
- **Test Data Coverage**: 
  - Stocks with complete data (P/S, EV/S ratios)
  - Stocks with missing data (negative earnings)
  - S&P 500 vs non-S&P 500 stocks
  - Various date ranges (2023-2024)

### **Priority-Based Testing**

#### **HIGH Priority Tests** (8 commands - 60% of functionality)
1. `get_stocks_paginated` - Pagination core functionality
2. `get_stocks_with_data_status` - Stock listing with data flags
3. `get_sp500_symbols` - S&P 500 filtering system
4. `get_price_history` - Historical price charts  
5. `get_valuation_ratios` - P/S & EV/S ratio display
6. `get_ps_evs_history` - Historical ratio charts
7. `get_undervalued_stocks_by_ps` - P/S screening
8. `get_value_recommendations_with_stats` - P/E recommendations

#### **MEDIUM Priority Tests** (3 commands - 25% of functionality)  
9. `search_stocks` - Search functionality
10. `get_stock_date_range` - Date range validation
11. `get_database_stats` - Statistics display

#### **LOW Priority Tests** (2 commands - 15% of functionality)
12. `export_data` - Data export
13. `get_initialization_status` - System status

### **Test Categories**

#### **1. Data Integrity Tests**
- Verify correct data returned for each command
- Test database query accuracy
- Validate data transformations (EPS calculations, ratio computations)

#### **2. Error Handling Tests**  
- Invalid symbols, dates, parameters
- Database connection errors
- Empty result sets
- Malformed input data

#### **3. Performance Tests**
- Large dataset handling (pagination performance)
- Query optimization validation  
- Memory usage for bulk operations

#### **4. Business Logic Tests**
- P/S screening thresholds work correctly
- Recommendation scoring algorithms
- Data completeness calculations
- Date range filtering accuracy

#### **5. Integration Tests**
- Full user workflows (search → analyze → export)
- Cross-command data consistency
- Frontend service layer integration

## Test Implementation Plan

### **Phase 1: Core Infrastructure** 
- Set up test database with fixtures
- Create helper utilities for database setup
- Implement basic test framework structure

### **Phase 2: High Priority Commands**
- Test all 8 high-priority commands
- Focus on data accuracy and error handling
- Implement comprehensive scenarios for each

### **Phase 3: Medium/Low Priority Commands**
- Complete remaining 5 commands  
- Add performance and integration tests
- Validate full user workflows

### **Phase 4: Continuous Integration**
- Add tests to CI/CD pipeline
- Set up test database refresh mechanism
- Create test result reporting

## Success Criteria

### **Coverage Goals**
- ✅ **100% Frontend-Called Functions**: All 13 commands tested
- ✅ **0% Dead Code Testing**: Skip unused commands
- ✅ **90%+ Code Coverage**: On tested functions only
- ✅ **Error Scenarios**: All major error paths covered

### **Quality Gates**
- All HIGH priority tests must pass
- No test database pollution between runs
- Tests run in <30 seconds total
- Clear test failure reporting with actionable messages

### **Validation**
- Mock frontend calls succeed with test data  
- Real production data queries work with test framework
- Performance benchmarks meet targets
- Error handling provides useful feedback to frontend

---

## Test Execution Commands

```bash
# Run all backend tests
cd src-tauri && cargo test

# Run specific test categories  
cargo test integration_tests::stock_operations
cargo test integration_tests::analysis_operations
cargo test integration_tests::recommendations

# Run with test database setup
cargo test -- --test-threads=1

# Performance benchmarks
cargo test --release performance_tests
```

## Related Documentation
- **Frontend API Layer**: `src/src/services/api.js` - All frontend API calls
- **Backend Commands**: `src-tauri/src/commands/` - Tauri command implementations  
- **Database Schema**: `src-tauri/db/migrations/` - Database structure
- **Project Context**: `context/architecture.md` - Overall system design

---
*Last Updated: 2025-09-09*
*Focus: Frontend-driven testing, no dead code, production-quality test coverage*