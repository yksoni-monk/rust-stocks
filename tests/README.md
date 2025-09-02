# Testing Architecture for Rust Stocks TUI

## Overview
This document outlines the testing strategy for the Rust Stocks TUI application, categorizing actions into programmatically testable and manually testable components.

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

## Test Structure

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
└── integration/             # Integration tests
    ├── mod.rs
    ├── database_integration.rs
    └── api_integration.rs
```

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

### Phase 1: Infrastructure Setup
1. Create test directory structure
2. Set up common test utilities
3. Create database test helpers
4. Set up API mocking framework

### Phase 2: Unit Tests
1. Database operation tests
2. Business logic tests (trading week batches)
3. Data processing tests
4. Configuration tests

### Phase 3: Integration Tests
1. Database integration tests
2. API integration tests (with mocks)
3. Component interaction tests

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
