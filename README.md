# Rust Stocks Analysis System

A high-performance Rust-based stock analysis system that fetches, stores, and analyzes S&P 500 stock data using the Charles Schwab API.

## 🚀 Quick Start - Just Run It!

```bash
# Clone and run the main application
git clone <repo>
cd rust-stocks
cargo run
```

**That's it!** The main interactive application will start automatically.

## 🎯 Project Overview

This system provides comprehensive stock market data collection and analysis capabilities, featuring:
- ✅ **Complete S&P 500 Coverage**: All 503 companies with real-time updates
- ✅ **High-Performance Concurrent Data Collection**: Optimized async processing
- ✅ **Professional CLI Tools**: Named arguments with comprehensive validation
- ✅ **Smart Market Calendar**: Automatic weekend/holiday handling with Schwab API integration
- ✅ **SQLite Database**: Local persistence with proper schema and indexing
- ✅ **Real-time Progress Tracking**: Detailed batch logging and error recovery
- ✅ **Charles Schwab API Integration**: Full authentication and data retrieval

## 🚀 Quick Start

### Prerequisites
- Rust (latest stable version)
- Charles Schwab API credentials
- SQLite (bundled with the application)

### Setup
1. Clone the repository
2. Copy `.env.example` to `.env` and configure your API credentials:
   ```bash
   SCHWAB_API_KEY=your_api_key
   SCHWAB_APP_SECRET=your_app_secret
   SCHWAB_CALLBACK_URL=your_callback_url
   SCHWAB_TOKEN_PATH=./schwab_tokens.json
   DATABASE_PATH=./stocks.db
   ```
3. Build the project:
   ```bash
   cargo build --release
   ```

### Run the Main Application
```bash
# Start the interactive TUI application (DEFAULT - recommended)
cargo run

# OR be explicit about the main binary
cargo run --bin rust-stocks
```

### Initialize S&P 500 Data (First Time Setup)
```bash
# Update the complete S&P 500 company list (503 companies)
cargo run --bin update_sp500
```

## 📊 Data Collection

### Concurrent Data Fetching

The system supports high-performance concurrent data fetching using multiple worker threads:

```bash
# Test concurrent fetching
cargo run --bin data_collection_test concurrent -s 20240101 --threads 5

# With custom configuration
cargo run --bin data_collection_test concurrent -s 20240101 -e 20240131 --threads 10 --retries 3
```

**Features:**
- 🚀 **Multi-threaded Processing**: Configurable number of worker threads
- 📊 **Smart Data Checking**: Automatically skips existing data
- 🔄 **Retry Logic**: Configurable retry attempts with exponential backoff
- 📈 **Real-time Progress**: Detailed logging of each thread's progress
- 🛡️ **Thread Safety**: Safe concurrent database operations
- ⚡ **Rate Limiting**: Per-thread API rate limiting to avoid violations
- 📅 **Weekly Batching**: Same trading week batching as single stock fetcher

### Data Collection Testing

The system provides comprehensive testing tools for data collection:

```bash
# Quick test with 10 stocks
cargo run --bin data_collection_test quick 20240101

# Detailed collection with full logging
cargo run --bin data_collection_test detailed -s 20240101 -e 20240131

# Concurrent collection demo
cargo run --bin data_collection_test concurrent -s 20240101 --threads 5

# Get help
cargo run --bin data_collection_test --help
```

**Features:**
- 🧪 **Quick Testing**: Test with 10 stocks for fast validation
- 📊 **Detailed Collection**: Full production-like collection with logging
- 🚀 **Concurrent Demo**: Multi-threaded collection testing
- 🗓️ **Smart Calendar**: Automatic weekend adjustment
- 📈 **Progress Tracking**: Real-time progress and error reporting

## 🛠️ Available Tools

### Main Application

- **`rust-stocks`** (DEFAULT): Interactive TUI application for stock analysis and data management
  ```bash
  cargo run  # Runs this automatically
  ```

### Utility Tools

- **`update_sp500`**: Update S&P 500 company list with state tracking
  ```bash
  cargo run --bin update_sp500
  ```

### Test Tools (Development/Testing)

- **`data_collection_test`**: Comprehensive data collection testing with subcommands:
  ```bash
  # Quick test with 10 stocks
  cargo run --bin data_collection_test quick 20240101
  
  # Detailed collection with full logging
  cargo run --bin data_collection_test detailed -s 20240101 -e 20240131
  
  # Concurrent collection demo
  cargo run --bin data_collection_test concurrent -s 20240101 --threads 5
  
  # Single stock testing
  cargo run --bin data_collection_test single AAPL 20240101 20240131
  ```
- **`api_connectivity_test`**: API connectivity testing with subcommands:
  ```bash
  # Test authentication
  cargo run --bin api_connectivity_test auth
  
  # Test quote fetching
  cargo run --bin api_connectivity_test quotes
  
  # Test price history
  cargo run --bin api_connectivity_test history AAPL 20240101 20240131
  
  # Run all tests
  cargo run --bin api_connectivity_test all
  ```

## 📊 Progress Tracking

The system provides detailed batch logging including:

```
📦 BATCH 1/101 - Processing 5 stocks:
   - AAPL (Apple Inc.)
   - MSFT (Microsoft Corporation)
   ...

🔄 [1/503] Starting AAPL: Apple Inc.
✅ [1/503] AAPL completed: 417 records in 2.3s

📊 BATCH 1/101 SUMMARY:
   ✅ Successful: 5/5 stocks  
   ❌ Failed: 0/5 stocks
   📈 Records added: 2,085
   ⏱️  Time taken: 12.1s
   📊 OVERALL PROGRESS: 5/503 stocks, 2,085 total records
```

## 🔧 Development

### Testing API Connectivity
```bash
cargo run --bin test_api
```

### Database Operations
```bash
# Check database stats
sqlite3 stocks.db "SELECT COUNT(*) FROM stocks;"
sqlite3 stocks.db "SELECT COUNT(*) FROM daily_prices;"

# View recent data
sqlite3 stocks.db "SELECT symbol, COUNT(*) FROM stocks s JOIN daily_prices p ON s.id = p.stock_id GROUP BY symbol LIMIT 10;"
```

### Performance Monitoring

The system includes comprehensive performance tracking:
- Individual stock processing times
- Batch completion rates and timings  
- Overall progress with record counts
- Error tracking with detailed reporting

### Testing

For comprehensive testing information, see [tests.md](tests.md).

## 📋 Current Status

### ✅ Completed Features
- **Complete S&P 500 Integration**: All 503 companies with database state tracking
- **Smart Market Calendar**: Automatic weekend/holiday handling with Schwab API integration
- **High-Performance Data Collection**: Concurrent processing with progress tracking  
- **Professional CLI**: Named arguments with comprehensive validation
- **Authentication System**: Schwab API with automatic token refresh
- **Database Architecture**: SQLite with proper schema and migrations

### 🔄 In Progress  
- **Historical Data Collection**: ~1.5M records for 503 companies (2020-2025)
- **Analysis Features**: P/E ratio decline analysis and stock ranking

### 📋 Planned Features
- **Real-time Data Updates**: Incremental daily updates
- **Advanced Analysis**: Technical indicators and trend analysis  
- **Web Interface**: REST API and web dashboard
- **Export Features**: CSV, JSON, and Excel export capabilities

## 🚨 Known Issues

1. **Token Management**: Requires periodic refresh every 30 minutes
2. **Rate Limiting**: Schwab API limits to 120 requests/minute
3. **Market Hours**: Some quotes may return $0.00 outside trading hours

## 📊 Success Metrics

- **Data Coverage**: 503/503 S&P 500 stocks ✅
- **Market Calendar**: Smart weekend/holiday handling ✅
- **Concurrent Processing**: High-performance async implementation ✅  
- **CLI Interface**: Professional named arguments with validation ✅
- **Progress Tracking**: Real-time batch monitoring ✅
- **Historical Data**: In progress (~1.5M records target)

## 📝 License

This project is for educational and personal use. Please ensure compliance with Charles Schwab API terms of service.