# Rust Stocks Analysis System

A high-performance Rust-based stock analysis system that fetches, stores, and analyzes S&P 500 stock data using the Charles Schwab API.

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

### Initialize S&P 500 Data
```bash
# Update the complete S&P 500 company list (503 companies)
cargo run --bin update_sp500
```

## 📊 Data Collection

### Historical Data Collection with Detailed Logging

The main data collection tool provides professional CLI with comprehensive validation:

⚠️ **IMPORTANT:** Always use `--` to separate cargo arguments from binary arguments!

```bash
# Basic usage - collect 2023 data
cargo run --bin collect_with_detailed_logs -- --start-date 20230101 --end-date 20231231

# Short form arguments  
cargo run --bin collect_with_detailed_logs -- -s 20240101 -e 20241231

# Start date only (end date defaults to today)
cargo run --bin collect_with_detailed_logs -- --start-date 20230101

# Custom batch processing
cargo run --bin collect_with_detailed_logs -- -s 20220101 -e 20221231 --batch-size 10 --batch-delay 5

# Get help
cargo run --bin collect_with_detailed_logs -- --help
```

### CLI Arguments

| Argument | Short | Required | Default | Description |
|----------|-------|----------|---------|-------------|
| `--start-date` | `-s` | ✅ Yes | - | Start date in YYYYMMDD format |
| `--end-date` | `-e` | ❌ No | Today | End date in YYYYMMDD format |
| `--batch-size` | `-b` | ❌ No | 5 | Stocks per batch (1-50) |
| `--batch-delay` | `-d` | ❌ No | 3 | Seconds between batches (1-60) |
| `--help` | `-h` | ❌ No | - | Show help information |

### Example Usage Scenarios

```bash
# Collect recent data (fast)  
cargo run --bin collect_with_detailed_logs -- -s 20240101

# Collect specific year with fast processing
cargo run --bin collect_with_detailed_logs -- -s 20230101 -e 20231231 -b 10 -d 1

# Collect quarter data with detailed logging
cargo run --bin collect_with_detailed_logs -- -s 20240101 -e 20240331 -b 3 -d 5

# Large historical collection (2020-2024)  
cargo run --bin collect_with_detailed_logs -- -s 20200101 -e 20241231 -b 5 -d 3
```

### Smart Market Calendar Collection

For automatic weekend and holiday handling, use the smart collection tool:

```bash
# Automatically handles weekends - Saturday returns Friday's data
cargo run --bin smart_collect -- 20250810  # Saturday → Returns Friday 2025-08-08 data

# Date ranges with automatic trading day adjustment  
cargo run --bin smart_collect -- 20240101 20240131  # Adjusts to trading days only

# Single trading day (no adjustment needed)
cargo run --bin smart_collect -- 20240115  # Monday → Returns same day data
```

**Smart Calendar Features:**
- 🗓️ **Weekend Handling**: Saturday/Sunday requests automatically return Friday data
- 📅 **Holiday Detection**: Uses Schwab API to detect market holidays  
- 🔄 **Automatic Adjustment**: Shows original vs adjusted date ranges
- ✨ **Seamless Experience**: No more "no data found" errors for weekends

## 🛠️ Available Tools

### Core Binaries

- **`collect_with_detailed_logs`**: Main data collection tool with professional CLI
- **`smart_collect`**: Smart collection with automatic weekend/holiday handling
- **`update_sp500`**: Update S&P 500 company list with state tracking
- **`rust-stocks`**: Main TUI application for stock analysis
- **`test_api`**: Test Schwab API connectivity
- **`fetch_history`**: Single stock historical data fetcher

### Utility Tools

- **`collect_sample_data`**: Test data collection with small sample
- **`collect_historical_data`**: Legacy concurrent collection (deprecated)
- **`list_companies`**: List companies in database

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

## 🏗️ Architecture

### Core Components

- **Schwab API Client**: Full authentication, token management, and data retrieval
- **Market Calendar**: Smart weekend/holiday detection using Schwab market hours API
- **Database Manager**: SQLite operations with proper schema and migrations
- **Data Collector**: High-performance concurrent historical data fetching
- **Analysis Engine**: P/E calculations and stock ranking (in development)
- **Terminal UI**: Interactive stock search and analysis interface

### Data Models

- **Stock**: Company information (symbol, name, sector, market cap)
- **DailyPrice**: OHLC data with volume, P/E ratios, and market metrics
- **StockAnalysis**: P/E decline analysis and performance metrics

### Architecture Principles

- ✅ **Concurrent Processing**: Async/await with semaphore-controlled rate limiting
- ✅ **Error Isolation**: Individual stock failures don't affect batch processing  
- ✅ **Progress Tracking**: Real-time monitoring with detailed logging
- ✅ **State Management**: Database metadata for incremental updates
- ✅ **Professional CLI**: Named arguments with comprehensive validation

## 📈 Database Schema

The system uses SQLite with the following main tables:

- `stocks`: S&P 500 company information
- `daily_prices`: Historical OHLC data with financial metrics
- `metadata`: System state tracking and configuration

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