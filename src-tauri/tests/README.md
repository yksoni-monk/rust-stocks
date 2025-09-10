# Backend Testing System

## 🛡️ **Safety-First Testing Strategy**

This testing system is designed to **NEVER touch your production database** by default. It uses a **copy-based approach** for maximum safety.

## 🧪 **How It Works**

### **Default Safe Mode** (Recommended)
```bash
# Safe testing - copies db/stocks.db to db/test.db
cargo test --test safe_backend_tests
```

- ✅ **Copies** `db/stocks.db` → `db/test.db` 
- ✅ **Tests run against the copy** (never production)
- ✅ **Real data testing** with production safety
- ✅ **No risk** to your valuable data

### **Production Database Testing** (Dangerous)
```bash
# DANGEROUS - tests against real production database
USE_PRODUCTION_DB=true cargo test --test safe_backend_tests
```

- ⚠️ **Direct access** to `db/stocks.db`
- ⚠️ **High risk** - could corrupt data
- ⚠️ **Only use** when absolutely necessary

### **Custom Test Database Path**
```bash
# Use custom test database location
TEST_DB_PATH=db/my_test.db cargo test --test safe_backend_tests
```

## 📁 **File Structure**

```
tests/
├── README.md                     ← This file
├── basic_integration_tests.rs    ← Basic compilation/structure tests
├── safe_backend_tests.rs         ← Safe tests with copied database
├── helpers/
│   ├── mod.rs
│   ├── test_config.rs            ← Environment configuration
│   └── database_setup.rs         ← Safe database copying logic
└── fixtures/
    └── sample_data.sql            ← Fallback sample data (10 stocks)
```

## 🎯 **Test Categories**

### **1. Basic Tests** (`basic_integration_tests.rs`)
- ✅ Compilation and import verification
- ✅ Basic error handling
- ✅ Module structure validation
- ✅ No database required

### **2. Safe Backend Tests** (`safe_backend_tests.rs`) 
- ✅ Full functionality testing with **copied production data**
- ✅ All 13 frontend-called functions tested
- ✅ Performance benchmarking
- ✅ Real data integration testing

## 🔧 **Environment Variables**

| Variable | Default | Description |
|----------|---------|-------------|
| `USE_PRODUCTION_DB` | `false` | Use production database directly ⚠️ |
| `TEST_DB_PATH` | `db/test.db` | Path for test database copy |

## 🚀 **Running Tests**

### **Quick Start** (Safe)
```bash
cd src-tauri
cargo test --test safe_backend_tests
```

### **All Tests**
```bash
cargo test
```

### **Specific Test**
```bash
cargo test test_get_stocks_paginated_with_real_data
```

### **With Production Database** (⚠️ Dangerous)
```bash
USE_PRODUCTION_DB=true cargo test --test safe_backend_tests
```

## 📊 **Test Output Examples**

### **Safe Mode** (Default)
```
🧪 Setting up test database: Test Database (db/test.db)
📋 Copying production database (2547.2 MB) to test database...
   Source: db/stocks.db  
   Target: db/test.db
✅ Successfully copied production database to test database
✅ Connected to test database: sqlite:db/test.db
✅ Database verified: 5892 stocks (production data)
```

### **Production Mode** (⚠️ Dangerous)
```
🧪 Setting up test database: Production Database (db/stocks.db)
⚠️  Using production database directly - BE CAREFUL!
✅ Connected to test database: sqlite:db/stocks.db
🚨 WARNING: Tests will run against PRODUCTION database
```

## 🛡️ **Safety Features**

### **Multiple Safety Layers**
1. **Default Copy Mode** - Never touches production by default
2. **Environment Variable Required** - Must explicitly set `USE_PRODUCTION_DB=true`
3. **Clear Warnings** - Obvious warnings when using production database
4. **Path Validation** - Prevents accidental production database access
5. **Size Verification** - Ensures database copy completed successfully

### **Database Copy Process**
1. **Check Production DB** exists (`db/stocks.db`)
2. **Report Size** (e.g., "2547.2 MB") for progress tracking
3. **Copy File** using secure file system operations
4. **Verify Copy** by comparing file sizes
5. **Connect Safely** to the test database copy

### **Fallback Strategy**
If production database doesn't exist:
- **Creates fresh test database** with sample data
- **10 test stocks** with complete financial data
- **Allows testing** even without production database

## 📈 **Test Coverage**

### **Frontend-Called Functions** (13 total)
✅ **Stock Operations** (4):
- `get_stocks_paginated` - Pagination testing
- `get_stocks_with_data_status` - Data availability flags
- `search_stocks` - Search functionality  
- `get_sp500_symbols` - S&P 500 filtering

✅ **Analysis Operations** (5):
- `get_stock_date_range` - Date range validation
- `get_price_history` - Historical price data
- `get_valuation_ratios` - P/S & EV/S ratios
- `get_ps_evs_history` - Historical ratio data
- `export_data` - Data export functionality

✅ **Recommendations** (2):
- `get_undervalued_stocks_by_ps` - P/S screening
- `get_value_recommendations_with_stats` - P/E recommendations

✅ **System Operations** (2):
- `get_initialization_status` - System status
- `get_database_stats` - Database statistics

## 🔍 **Performance Benchmarking**

Tests include performance measurements:
- **Pagination**: < 500ms for 50 stocks
- **Search**: < 200ms for query results
- **Price History**: < 1s for 1-year data
- **Database Stats**: < 300ms for calculations

## ⚠️ **Important Warnings**

### **Production Database Testing**
- **Only use** `USE_PRODUCTION_DB=true` when absolutely necessary
- **Always backup** your production database first
- **Review test code** before running against production
- **Monitor test output** for any unexpected behavior

### **Test Database Management**
- Test database (`db/test.db`) can be **safely deleted** anytime
- Will be **automatically recreated** on next test run
- **2.5GB size** - ensure adequate disk space
- **Temporary file** - safe to exclude from version control

---

## 🎯 **Best Practices**

1. **Always use safe mode** for regular testing
2. **Backup production database** before any risky operations
3. **Review test output** to ensure expected behavior
4. **Clean up test databases** when disk space is low
5. **Use production mode sparingly** and with extreme caution

**Your production data safety is the top priority.** 🛡️