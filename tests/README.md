# Testing Architecture for Rust Stocks TUI

## Overview
This document outlines the testing strategy for the Rust Stocks TUI application, categorizing tests into unit tests, integration tests, and manual testing.

## Current Test Structure Analysis

### ❌ **Current Issues**
1. **API Testing Duplication**: 4 different binaries all testing API connectivity
2. **Batch Definition Inconsistency**: `test_batched_stock.rs` uses 20-day batches vs trading week batches elsewhere
3. **Missing Unit Tests**: No proper unit tests for business logic
4. **All Manual Testing**: All current tests are manual integration tests

### ✅ **Proposed Structure**

## Action Categorization

### ✅ Programmatically Testable Actions (Unit Tests)

#### **Database Operations** (All testable)
- Insert/Update/Get stock operations
- Daily price operations
- Data statistics and coverage
- Metadata operations
- Error handling scenarios

#### **Business Logic** (All testable)
- Trading week batch calculations
- Date range processing
- Stock data validation
- Configuration loading
- Error handling logic

#### **API Client Operations** (Mockable)
- API request/response handling
- Token validation
- Rate limiting
- Error handling

#### **Data Processing** (All testable)
- Log message processing
- Data transformation
- Validation logic
- State management

#### **Configuration & Setup** (All testable)
- Environment variable loading
- Database initialization
- API client setup
- Error handling

### ❌ Manually Testable Actions (User Testing)

#### **UI Interactions**
- Keyboard navigation
- Real-time UI updates
- Visual rendering
- User experience flow

#### **Real API Integration**
- Live Schwab API calls
- Real-time data fetching
- Network connectivity
- Rate limiting behavior

#### **TUI Framework Integration**
- Ratatui rendering
- Crossterm event handling
- Terminal state management

## Testing Methodology

### 1. Unit Testing Strategy
- **Isolation**: Each component tested in isolation
- **Mocking**: External dependencies (API, database) mocked
- **Comprehensive Coverage**: All business logic paths tested
- **Error Scenarios**: Edge cases and error conditions tested

### 2. Integration Testing Strategy
- **Database Integration**: Real SQLite database with test data
- **API Mocking**: Mock HTTP responses for API testing
- **Component Integration**: Test component interactions

### 3. Test Data Management
- **Test Database**: Separate test database file
- **Fixture Data**: Predefined test data sets
- **Cleanup**: Automatic cleanup after tests

### 4. Logging & Debugging
- **Structured Logging**: All tests log their actions
- **Log Parsing**: Parse logs to verify behavior
- **Error Context**: Detailed error information in logs

## Consolidated Test Structure

```
tests/
├── README.md                 # This document
├── common/                   # Shared test utilities
│   ├── mod.rs
│   ├── database.rs           # Database test helpers
│   ├── api_mock.rs          # API mocking utilities
│   └── fixtures.rs          # Test data fixtures
├── unit/                     # Unit tests
│   ├── mod.rs
│   ├── database/            # Database operation tests
│   ├── business_logic/      # Business logic tests
│   ├── api_client/          # API client tests
│   └── data_processing/     # Data processing tests
├── integration/             # Integration tests
│   ├── mod.rs
│   ├── database_integration.rs
│   └── api_integration.rs
└── bin/                     # Manual testing binaries (consolidated)
    ├── data_collection_test.rs    # Comprehensive data collection testing
    ├── api_connectivity_test.rs   # API connectivity testing
    └── update_sp500.rs            # Database maintenance
```

## Consolidated Test Binaries

### **Manual Testing Tools** (Keep only these)

#### **`data_collection_test.rs`** - Comprehensive Data Collection Testing
- **`quick <date>`** - Quick test with 10 stocks
- **`detailed -s <start> -e <end>`** - Full production-like collection
- **`concurrent -s <start> --threads <n>`** - Multi-threaded collection demo
- **`single <symbol> <start> <end>`** - Single stock testing

#### **`api_connectivity_test.rs`** - API Connectivity Testing
- Test API authentication
- Test quote fetching
- Test price history fetching
- Test error handling

#### **`update_sp500.rs`** - Database Maintenance
- Update S&P 500 list
- Database statistics
- Data validation

### **Removed Binaries** (Duplicated functionality)
- ❌ `simple_test.rs` - Duplicated by `data_collection_test quick`
- ❌ `test_api.rs` - Duplicated by `api_connectivity_test`
- ❌ `test_single_stock.rs` - Duplicated by `data_collection_test single`
- ❌ `test_batched_stock.rs` - Wrong batch definition, duplicated functionality

## Testing Tools & Crates

### Core Testing
- `tokio-test`: Async testing utilities
- `mockall`: Mocking framework
- `tempfile`: Temporary file management
- `test-log`: Logging in tests

### Database Testing
- `rusqlite`: SQLite testing
- `tempfile`: Temporary database files

### API Testing
- `wiremock`: HTTP API mocking
- `reqwest`: HTTP client testing

### Assertion & Validation
- `assert_matches`: Pattern matching assertions
- `pretty_assertions`: Better assertion output

## Implementation Plan

### Phase 1: Test Consolidation (Current)
1. ✅ Consolidate duplicate binary tests
2. ✅ Remove `test_batched_stock.rs` (wrong batch definition)
3. ✅ Update test plan documentation
4. ✅ Fix batch definition consistency

### Phase 2: Unit Test Implementation
1. Create proper unit tests for business logic
2. Implement API client mocking
3. Add database operation unit tests
4. Add configuration unit tests

### Phase 3: Integration Test Enhancement
1. Enhance existing integration tests
2. Add component interaction tests
3. Add error scenario tests
4. Add performance tests

### Phase 4: Test Coverage & Quality
1. Add comprehensive error scenario tests
2. Performance testing for large datasets
3. Memory leak testing
4. Test documentation

## Success Criteria

### Unit Test Coverage
- **Database Operations**: 100% coverage
- **Business Logic**: 100% coverage
- **Data Processing**: 100% coverage
- **Error Handling**: 100% coverage

### Integration Test Coverage
- **Database Integration**: All CRUD operations
- **API Integration**: All API endpoints (mocked)
- **Component Integration**: All component interactions

### Quality Metrics
- **Test Execution Time**: < 30 seconds for full suite
- **Memory Usage**: No memory leaks detected
- **Error Detection**: All error scenarios covered
- **Log Quality**: Structured, parseable logs

## Manual Testing Checklist

### UI Testing (Manual)
- [ ] Tab navigation works correctly
- [ ] Keyboard shortcuts function properly
- [ ] Real-time log updates display correctly
- [ ] Date input validation works
- [ ] Stock selection and search works
- [ ] Error messages display properly
- [ ] UI responsiveness during operations

### API Integration Testing (Manual)
- [ ] Live API calls work correctly
- [ ] Rate limiting behavior is appropriate
- [ ] Error handling for network issues
- [ ] Token refresh works correctly
- [ ] Real data fetching and storage

### End-to-End Testing (Manual)
- [ ] Complete data collection workflow
- [ ] Data analysis workflow
- [ ] Error recovery scenarios
- [ ] Performance with large datasets
- [ ] User experience flow
