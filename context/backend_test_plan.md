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

---

## Future Work & Enhancements

### **Performance Benchmarks**

#### **Response Time Targets**
```markdown
### **Performance Benchmarks**
- **Stock Pagination**: <100ms for 50 stocks (current: ~200ms)
- **Stock Search**: <200ms for query results (current: ~300ms)  
- **S&P 500 Filter**: <150ms for symbol loading (current: ~500ms)
- **Price History**: <500ms for 1-year data (current: ~800ms)
- **Valuation Ratios**: <300ms for P/S & EV/S calculation (current: ~400ms)
- **Recommendations**: <1s for 20 recommendations with stats (current: ~1.5s)
- **Database Stats**: <200ms for statistics calculation (current: ~300ms)
```

#### **Performance Test Implementation**
```rust
// Add to tests/performance_tests.rs
#[tokio::test]
async fn test_pagination_performance() {
    let start = Instant::now();
    let result = get_stocks_paginated(50, 0).await;
    let duration = start.elapsed();
    
    assert!(duration < Duration::from_millis(100));
    assert_eq!(result.stocks.len(), 50);
}

#[tokio::test]
async fn test_search_performance() {
    let start = Instant::now();
    let result = search_stocks("AAPL").await;
    let duration = start.elapsed();
    
    assert!(duration < Duration::from_millis(200));
    assert!(!result.is_empty());
}

#[tokio::test]
async fn test_analysis_performance() {
    let start = Instant::now();
    let result = get_price_history("AAPL", "2023-01-01", "2023-12-31").await;
    let duration = start.elapsed();
    
    assert!(duration < Duration::from_millis(500));
    assert!(result.len() > 200); // ~250 trading days
}
```

### **Integration Test Workflows**

#### **Complete User Journey Tests**
```markdown
### **End-to-End User Workflows**

#### **1. Stock Analysis Workflow**
```rust
#[tokio::test]
async fn test_complete_analysis_workflow() {
    // Step 1: Load initial stock list
    let stocks = get_stocks_paginated(50, 0).await;
    assert!(!stocks.stocks.is_empty());
    
    // Step 2: Search for specific stock
    let search_results = search_stocks("Apple").await;
    assert!(!search_results.is_empty());
    
    // Step 3: Get stock date range
    let date_range = get_stock_date_range("AAPL").await;
    assert!(date_range.start_date.is_some());
    
    // Step 4: Load price history
    let price_history = get_price_history(
        "AAPL", 
        date_range.start_date.unwrap(), 
        date_range.end_date.unwrap()
    ).await;
    assert!(!price_history.is_empty());
    
    // Step 5: Get valuation ratios
    let ratios = get_valuation_ratios("AAPL").await;
    assert!(ratios.ps_ratio.is_some());
    
    // Step 6: Export data
    let export_result = export_data("AAPL", "csv").await;
    assert!(export_result.success);
}
```

#### **2. S&P 500 Filter Workflow**
```rust
#[tokio::test]
async fn test_sp500_filter_workflow() {
    // Step 1: Load S&P 500 symbols
    let sp500_symbols = get_sp500_symbols().await;
    assert!(!sp500_symbols.is_empty());
    assert!(sp500_symbols.len() > 400); // Should have ~500 symbols
    
    // Step 2: Load all stocks
    let all_stocks = get_stocks_with_data_status().await;
    assert!(!all_stocks.is_empty());
    
    // Step 3: Filter to S&P 500 only
    let sp500_stocks: Vec<_> = all_stocks.iter()
        .filter(|stock| sp500_symbols.contains(&stock.symbol))
        .collect();
    
    // Step 4: Test pagination with filtered results
    let paginated_sp500 = get_stocks_paginated(20, 0).await;
    // Note: This would need backend support for filtered pagination
    
    // Step 5: Verify S&P 500 stocks have complete data
    for stock in &sp500_stocks[..5] { // Test first 5
        let ratios = get_valuation_ratios(&stock.symbol).await;
        assert!(ratios.ps_ratio.is_some() || ratios.evs_ratio.is_some());
    }
}
```

#### **3. Recommendations Workflow**
```rust
#[tokio::test]
async fn test_recommendations_workflow() {
    // Step 1: Load P/E based recommendations
    let pe_recommendations = get_value_recommendations_with_stats(10).await;
    assert!(!pe_recommendations.recommendations.is_empty());
    assert!(pe_recommendations.stats.total_analyzed > 0);
    
    // Step 2: Load P/S based recommendations
    let ps_recommendations = get_undervalued_stocks_by_ps(2.0, 10).await;
    assert!(!ps_recommendations.is_empty());
    
    // Step 3: Cross-validate recommendations
    let pe_symbols: HashSet<_> = pe_recommendations.recommendations
        .iter()
        .map(|r| &r.symbol)
        .collect();
    
    let ps_symbols: HashSet<_> = ps_recommendations
        .iter()
        .map(|r| &r.symbol)
        .collect();
    
    // Should have some overlap between P/E and P/S recommendations
    let overlap = pe_symbols.intersection(&ps_symbols).count();
    assert!(overlap > 0, "P/E and P/S recommendations should have some overlap");
    
    // Step 4: Analyze specific recommendation
    if let Some(recommendation) = pe_recommendations.recommendations.first() {
        let ratios = get_valuation_ratios(&recommendation.symbol).await;
        assert!(ratios.pe_ratio.is_some());
        
        let price_history = get_price_history(
            &recommendation.symbol,
            "2023-01-01",
            "2023-12-31"
        ).await;
        assert!(!price_history.is_empty());
    }
}
```

#### **4. Error Recovery Workflow**
```rust
#[tokio::test]
async fn test_error_recovery_workflow() {
    // Step 1: Test invalid symbol handling
    let invalid_result = get_price_history("INVALID_SYMBOL", "2023-01-01", "2023-12-31").await;
    assert!(invalid_result.is_empty());
    
    // Step 2: Test invalid date range
    let invalid_dates = get_price_history("AAPL", "2023-12-31", "2023-01-01").await;
    assert!(invalid_dates.is_empty());
    
    // Step 3: Test empty search results
    let empty_search = search_stocks("NONEXISTENT_COMPANY_XYZ").await;
    assert!(empty_search.is_empty());
    
    // Step 4: Test pagination beyond available data
    let beyond_data = get_stocks_paginated(50, 10000).await;
    assert!(beyond_data.stocks.is_empty());
    assert_eq!(beyond_data.offset, 10000);
    
    // Step 5: Test S&P 500 timeout fallback
    // This would require mocking the GitHub API timeout
    let sp500_fallback = get_sp500_symbols().await;
    assert!(!sp500_fallback.is_empty()); // Should fallback to DB
}
```

### **Enhanced Test Data Scenarios**

#### **Edge Case Test Data**
```markdown
### **Comprehensive Test Data Coverage**

#### **1. Financial Edge Cases**
- **Zero Revenue Stock**: Company with $0 revenue (division by zero in P/S)
- **Negative P/E Stock**: Unprofitable company with negative earnings
- **Missing Financial Data**: Stock with price data but no financials
- **Extreme Ratios**: P/S > 100 or P/E > 500 (growth stocks)
- **Penny Stock**: Stock with price < $1 (different calculation needs)

#### **2. Date Edge Cases**
- **Weekend Dates**: Request data for Saturday/Sunday
- **Holiday Dates**: Market closed dates (Christmas, New Year)
- **Future Dates**: Request data beyond today
- **Very Old Dates**: Pre-2019 data (before current schema)
- **Leap Year**: February 29th handling
- **Timezone Edge Cases**: Different timezone handling

#### **3. Data Completeness Scenarios**
- **Complete Data**: Stock with all ratios, price history, financials
- **Partial Data**: Stock with price but missing P/E (negative earnings)
- **Minimal Data**: Stock with only 1 day of price data
- **Inconsistent Data**: Stock with gaps in price history
- **Corrupted Data**: Malformed database entries
```

#### **Test Data Factory Implementation**
```rust
// Add to tests/helpers/test_data_factory.rs
pub struct TestDataFactory;

impl TestDataFactory {
    pub async fn create_complete_stock(symbol: &str) -> Stock {
        // Create stock with all data (price, ratios, financials)
    }
    
    pub async fn create_unprofitable_stock(symbol: &str) -> Stock {
        // Create stock with negative earnings
    }
    
    pub async fn create_zero_revenue_stock(symbol: &str) -> Stock {
        // Create stock with $0 revenue
    }
    
    pub async fn create_penny_stock(symbol: &str) -> Stock {
        // Create stock with price < $1
    }
    
    pub async fn create_minimal_data_stock(symbol: &str) -> Stock {
        // Create stock with only 1 day of price data
    }
}
```

### **Advanced Testing Features**

#### **Concurrent Access Testing**
```rust
#[tokio::test]
async fn test_concurrent_database_access() {
    let handles: Vec<_> = (0..10)
        .map(|i| {
            tokio::spawn(async move {
                get_stocks_paginated(10, i * 10).await
            })
        })
        .collect();
    
    let results = futures::future::join_all(handles).await;
    
    for result in results {
        assert!(result.is_ok());
        let stocks = result.unwrap();
        assert_eq!(stocks.stocks.len(), 10);
    }
}
```

#### **Memory Usage Testing**
```rust
#[tokio::test]
async fn test_memory_usage_large_dataset() {
    let start_memory = get_memory_usage();
    
    // Load large dataset
    let large_result = get_stocks_paginated(1000, 0).await;
    assert_eq!(large_result.stocks.len(), 1000);
    
    let end_memory = get_memory_usage();
    let memory_increase = end_memory - start_memory;
    
    // Should not use more than 50MB for 1000 stocks
    assert!(memory_increase < 50 * 1024 * 1024);
}
```

#### **Database Corruption Testing**
```rust
#[tokio::test]
async fn test_database_corruption_recovery() {
    // Simulate database corruption
    corrupt_test_database().await;
    
    // Test that commands handle corruption gracefully
    let result = get_stocks_paginated(10, 0).await;
    
    // Should either return empty results or handle error gracefully
    match result {
        Ok(stocks) => {
            // If it succeeds, data should be valid
            for stock in stocks.stocks {
                assert!(!stock.symbol.is_empty());
            }
        }
        Err(e) => {
            // Error should be informative
            assert!(e.to_string().contains("database"));
        }
    }
}
```

### **Continuous Integration Enhancements**

#### **Test Reporting**
```markdown
### **CI/CD Integration**

#### **1. Test Result Reporting**
- **Coverage Reports**: Generate HTML coverage reports
- **Performance Reports**: Track performance regression over time
- **Failure Analysis**: Categorize test failures (data, performance, integration)
- **Trend Analysis**: Track test execution time trends

#### **2. Automated Test Data Refresh**
- **Weekly Refresh**: Update test database with latest production data sample
- **Data Validation**: Ensure test data remains representative
- **Schema Validation**: Verify test data matches current schema
```

#### **Test Execution Optimization**
```rust
// Add to tests/helpers/test_runner.rs
pub struct TestRunner {
    pub parallel_tests: bool,
    pub test_timeout: Duration,
    pub memory_limit: usize,
}

impl TestRunner {
    pub async fn run_performance_tests(&self) -> TestResults {
        // Run performance tests with timing
    }
    
    pub async fn run_integration_tests(&self) -> TestResults {
        // Run integration tests with workflow validation
    }
    
    pub async fn run_stress_tests(&self) -> TestResults {
        // Run stress tests with high load
    }
}
```