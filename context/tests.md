# Tests Documentation

## 🏗️ Test Architecture

### Test Database Isolation

The test suite uses a sophisticated database isolation strategy to prevent test interference:

- **`init_fresh_test_database()`**: Creates a unique database file for each test using timestamps
- **`init_test_database()`**: Shared database for tests that need sample data
- **`tests/tmp/`**: Temporary directory for all test database files
- **Automatic cleanup**: Test database files are cleaned up after test completion

### Test Organization

```
tests/
├── main.rs                          # Main test entry point and infrastructure tests
├── common/                          # Shared test utilities
│   ├── mod.rs                       # Common test modules and utilities
│   ├── database.rs                  # Database test utilities and isolation
│   ├── test_data.rs                 # Test data generation utilities
│   └── logging.rs                   # Test logging utilities
├── unit/                            # Unit tests
│   ├── database/                    # Database operation unit tests
│   │   ├── mod.rs                   # Database test module
│   │   └── operations.rs            # CRUD operations and statistics tests
│   └── business_logic/              # Business logic unit tests
│       └── trading_week_batches.rs  # Trading week batch calculation tests
├── integration/                     # Integration tests
│   ├── mod.rs                       # Integration test module
│   ├── database_integration.rs      # Database integration workflow tests
│   └── concurrent_fetcher_integration.rs  # Concurrent data fetching tests
└── bin/                             # Test binaries (moved from src/bin/)
    ├── api_connectivity_test.rs     # API connectivity test binary
    └── data_collection_test.rs      # Data collection test binary
```

### Test Categories

1. **Unit Tests**: Test individual functions and components in isolation
2. **Integration Tests**: Test complete workflows and component interactions
3. **Test Binaries**: Standalone test executables for specific scenarios

## 📋 Test Descriptions

### Main Test Infrastructure (`tests/main.rs`)

| Test | Description |
|------|-------------|
| `test_test_infrastructure` | Verifies basic test infrastructure is working |
| `test_common_utilities` | Tests common test utilities (test data, logging) |
| `test_module_imports` | Ensures all test modules can be imported without errors |

### Database Unit Tests (`tests/unit/database/operations.rs`)

| Test | Description |
|------|-------------|
| `test_stock_crud_operations` | Tests stock creation, reading, updating, and deletion operations |
| `test_daily_price_operations` | Tests daily price insertion, retrieval, and duplicate handling |
| `test_database_statistics` | Tests database statistics calculation with empty database |
| `test_existing_records_counting` | Tests counting existing records for date ranges |
| `test_metadata_operations` | Tests metadata setting, getting, and updating operations |
| `test_error_handling` | Tests error handling for non-existent stocks and prices |
| `test_database_cleanup` | Tests database cleanup operations and data removal |
| `test_pe_ratio_and_market_cap_operations` | Tests P/E ratio and market cap retrieval operations |

### Business Logic Unit Tests (`tests/unit/business_logic/trading_week_batches.rs`)

| Test | Description |
|------|-------------|
| `test_trading_week_batch_calculation_basic` | Tests basic trading week batch calculation |
| `test_trading_week_batch_calculation_single_day` | Tests batch calculation for single day ranges |
| `test_trading_week_batch_calculation_single_week` | Tests batch calculation for single week ranges |
| `test_trading_week_batch_calculation_multiple_months` | Tests batch calculation for multi-month ranges |
| `test_trading_week_batch_calculation_edge_cases` | Tests edge cases in batch calculation |
| `test_trading_week_batch_calculation_weekend_start` | Tests batch calculation starting on weekends |
| `test_trading_week_batch_calculation_weekend_end` | Tests batch calculation ending on weekends |
| `test_batch_ordering` | Tests that batches are properly ordered chronologically |
| `test_batch_descriptions` | Tests batch description generation |
| `test_week_start_calculation` | Tests trading week start date calculation |
| `test_week_end_calculation` | Tests trading week end date calculation |

### Database Integration Tests (`tests/integration/database_integration.rs`)

| Test | Description |
|------|-------------|
| `test_full_data_collection_workflow` | Tests complete data collection workflow with multiple stocks |
| `test_batch_processing_simulation` | Tests batch processing simulation with trading week logic |
| `test_error_recovery_scenarios` | Tests error recovery scenarios and data integrity |

### Concurrent Fetcher Integration Tests (`tests/integration/concurrent_fetcher_integration.rs`)

| Test | Description |
|------|-------------|
| `test_concurrent_fetch_config_validation` | Tests concurrent fetch configuration validation |
| `test_date_range_validation` | Tests date range validation for concurrent fetching |
| `test_concurrent_fetch_integration` | Tests concurrent fetch integration with real API calls |
| `test_concurrent_fetch_with_small_date_range` | Tests concurrent fetching with small date ranges |
| `test_concurrent_fetch_error_handling` | Tests error handling during concurrent fetching |

### Common Database Tests (`tests/common/database.rs`)

| Test | Description |
|------|-------------|
| `test_database_creation` | Tests database creation and sample data insertion |
| `test_concurrent_access` | Tests concurrent database access and thread safety |

### Test Binaries (`tests/bin/`)

| Binary | Description |
|--------|-------------|
| `api_connectivity_test` | Standalone binary for testing API connectivity |
| `data_collection_test` | Standalone binary for testing data collection workflows |

## 🚀 Running Tests

### Run All Tests
```bash
cargo test
```

### Run Specific Test Categories
```bash
# Run only unit tests
cargo test --test main -- unit

# Run only integration tests
cargo test --test main -- integration

# Run only database tests
cargo test --test main -- unit::database

# Run only business logic tests
cargo test --test main -- unit::business_logic
```

### Run Individual Tests
```bash
# Run a specific test
cargo test test_database_statistics

# Run tests with output
cargo test -- --nocapture

# Run tests matching a pattern
cargo test database
```

### Test Database Management
```bash
# Clean up test databases
rm -rf tests/tmp/*.db

# View test database contents
sqlite3 tests/tmp/test.db ".tables"
```

## 📊 Test Coverage

### Current Status
- **32 tests total** ✅
- **All tests passing** ✅
- **Complete database isolation** ✅
- **Comprehensive error handling** ✅
- **Concurrent access testing** ✅

### Test Categories Breakdown
- **Unit Tests**: 18 tests (database operations, business logic)
- **Integration Tests**: 8 tests (workflows, concurrent processing)
- **Infrastructure Tests**: 3 tests (test setup, utilities)
- **Test Binaries**: 2 binaries (API connectivity, data collection)

### Key Testing Principles
- **Isolation**: Each test uses its own database when needed
- **Completeness**: Tests cover all major functionality
- **Error Handling**: Tests verify proper error handling
- **Performance**: Tests include concurrent access scenarios
- **Real-world**: Integration tests use real API calls

## 🔧 Test Utilities

### Database Utilities
- `init_fresh_test_database()`: Creates unique database for each test
- `init_test_database()`: Creates shared database with sample data
- `insert_sample_stocks()`: Inserts sample stock data for testing
- `cleanup_test_database()`: Cleans up test database files

### Test Data Utilities
- `create_test_stock()`: Creates test stock objects
- `create_test_daily_price()`: Creates test daily price objects
- `create_test_date_range()`: Creates test date ranges

### Logging Utilities
- `init_test_logging()`: Initializes test logging
- `log_test_step()`: Logs test execution steps
- `log_test_data()`: Logs test data for debugging
