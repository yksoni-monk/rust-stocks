# Schwab Bulk Price Data Download - Implementation Architecture

## 📋 Executive Summary

Design document for implementing a robust bulk price data download system that will:
- Download historical OHLCV data for all S&P 500 stocks (2019-2024)
- Integrate seamlessly with existing database schema
- Handle errors, rate limits, and resume interrupted downloads
- Provide comprehensive progress tracking and validation

## 🎯 Objectives

### Primary Goals
1. **Data Completeness**: Download ~754K price records for 503 S&P 500 stocks
2. **Time Range**: 2019-2024 coverage to align with EDGAR financial data
3. **Reliability**: Handle network issues, API limits, and interruptions
4. **Integration**: Seamless integration with existing `daily_prices` table
5. **Performance**: Complete download in under 30 minutes with progress tracking

### Success Metrics
- ✅ 503 S&P 500 stocks with complete price history
- ✅ 2019-2024 date range coverage (1,500+ trading days per stock)
- ✅ <1% missing data tolerance
- ✅ All OHLCV fields populated correctly
- ✅ Resumable download capability
- ✅ Error rate <0.1% for API calls

## 🏗️ System Architecture

### High-Level Design
```
┌─────────────────────────────────────────────────────────────────────────┐
│                    Schwab Bulk Download System                           │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌─────────────────┐    ┌──────────────────┐    ┌──────────────────┐   │
│  │   S&P 500       │    │   Progress       │    │   Data           │   │
│  │   Symbol        │───▶│   Tracker        │───▶│   Validator      │   │
│  │   Manager       │    │                  │    │                  │   │
│  └─────────────────┘    └──────────────────┘    └──────────────────┘   │
│           │                        │                        │           │
│           ▼                        ▼                        ▼           │
│  ┌─────────────────┐    ┌──────────────────┐    ┌──────────────────┐   │
│  │   Schwab API    │    │   Rate Limiter   │    │   Database       │   │
│  │   Client        │◀──▶│   & Retry Logic  │───▶│   Integration    │   │
│  │                 │    │                  │    │                  │   │
│  └─────────────────┘    └──────────────────┘    └──────────────────┘   │
│           │                        │                        │           │
│           ▼                        ▼                        ▼           │
│  ┌─────────────────┐    ┌──────────────────┐    ┌──────────────────┐   │
│  │   Error         │    │   Resume         │    │   Reporting      │   │
│  │   Handler       │    │   Capability     │    │   & Logging      │   │
│  │                 │    │                  │    │                  │   │
│  └─────────────────┘    └──────────────────┘    └──────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

### Core Components

#### 1. S&P 500 Symbol Manager
```rust
struct SymbolManager {
    symbols: Vec<String>,
    completed: HashSet<String>,
    failed: HashMap<String, FailureReason>,
    skipped: HashSet<String>,
}
```

**Responsibilities:**
- Load S&P 500 symbols from database
- Track completion status per symbol
- Handle symbol additions/removals
- Provide progress statistics

#### 2. Progress Tracker
```rust
struct ProgressTracker {
    total_symbols: usize,
    completed_symbols: usize,
    failed_symbols: usize,
    start_time: DateTime<Utc>,
    estimated_completion: Option<DateTime<Utc>>,
    progress_file: PathBuf,
}
```

**Responsibilities:**
- Real-time progress tracking
- ETA calculations
- Persistent progress state (JSON file)
- Resume capability support
- Performance metrics

#### 3. Rate Limiter & Retry Logic
```rust
struct RateLimitedRetryClient {
    schwab_client: SchwabClient,
    rate_limiter: ApiRateLimiter,
    retry_config: RetryConfig,
    circuit_breaker: CircuitBreaker,
}
```

**Responsibilities:**
- Enforce 120 requests/minute API limit
- Exponential backoff retry strategy
- Circuit breaker for API failures
- Network timeout handling

#### 4. Data Validator
```rust
struct DataValidator {
    expected_trading_days: usize,
    tolerance_percentage: f64,
    validation_rules: Vec<ValidationRule>,
}
```

**Responsibilities:**
- Validate OHLCV data completeness
- Check price data sanity (no negative values, volume ranges)
- Verify date continuity
- Flag outliers and anomalies

#### 5. Database Integration
```rust
struct DatabaseIntegrator {
    db_pool: SqlitePool,
    batch_size: usize,
    conflict_resolution: ConflictStrategy,
}
```

**Responsibilities:**
- Batch insert price records
- Handle duplicate date conflicts
- Maintain referential integrity with stocks table
- Transaction management

## 📊 Database Schema Integration

### Existing `daily_prices` Table
```sql
CREATE TABLE daily_prices (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER NOT NULL,
    date DATE NOT NULL,
    open_price REAL NOT NULL,
    high_price REAL NOT NULL,
    low_price REAL NOT NULL,
    close_price REAL NOT NULL,
    volume INTEGER,
    -- Additional fields...
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, date)
);
```

### Data Mapping Strategy
```rust
// Schwab API Response → Database Record
SchwabPriceBar {
    datetime: i64,     // Convert to DATE
    open: f64,         // → open_price
    high: f64,         // → high_price
    low: f64,          // → low_price
    close: f64,        // → close_price
    volume: i64,       // → volume
} → DailyPriceRecord
```

### Conflict Resolution
- **UPSERT Strategy**: Update existing records if date already exists
- **Data Priority**: Schwab API data takes precedence over existing data
- **Audit Trail**: Log all overwrites for review

## 🔄 Error Handling & Recovery

### Error Categories

#### 1. Network Errors
- **Timeout**: Retry with exponential backoff
- **Connection Issues**: Circuit breaker activation
- **DNS Resolution**: Immediate retry with different endpoint

#### 2. API Errors
- **Rate Limit (429)**: Wait and retry with rate limiter
- **Authentication (401)**: Token refresh attempt
- **Invalid Symbol (404)**: Skip symbol and continue
- **Server Error (5xx)**: Retry with exponential backoff

#### 3. Data Errors
- **Malformed Response**: Log error, skip record
- **Missing Fields**: Use default values where possible
- **Invalid Dates**: Skip record, log issue

#### 4. Database Errors
- **Connection Issues**: Retry with connection pool
- **Constraint Violations**: Handle duplicate key gracefully
- **Transaction Failures**: Rollback and retry batch

### Recovery Strategies

#### Progress Persistence
```json
{
  "session_id": "uuid",
  "start_time": "2024-09-18T04:16:13Z",
  "total_symbols": 503,
  "completed_symbols": ["AAPL", "MSFT", ...],
  "failed_symbols": {
    "INVALID": "Symbol not found",
    "NETZ": "API timeout"
  },
  "current_symbol": "GOOGL",
  "settings": {
    "start_date": "2019-01-01",
    "end_date": "2024-12-31",
    "batch_size": 100
  }
}
```

#### Resume Capability
- Automatic detection of incomplete downloads
- Skip already completed symbols
- Retry failed symbols with fresh API calls
- Validate existing data before resume

## ⚡ Performance Optimization

### Concurrent Processing
```rust
// Process multiple symbols concurrently while respecting rate limits
let semaphore = Arc::new(Semaphore::new(3)); // Max 3 concurrent requests
let rate_limiter = Arc::new(ApiRateLimiter::new(120)); // 120/minute

for symbol in symbols {
    let permit = semaphore.acquire().await;
    let rate_limiter = rate_limiter.clone();
    
    tokio::spawn(async move {
        rate_limiter.wait().await;
        let result = fetch_price_data(symbol).await;
        drop(permit);
        result
    });
}
```

### Batch Database Operations
- **Batch Size**: 100 records per transaction
- **Connection Pooling**: 5 concurrent database connections
- **Prepared Statements**: Reuse for performance
- **Bulk Inserts**: Use SQLite's bulk insert optimization

### Memory Management
- **Streaming**: Process one symbol at a time
- **Memory Limits**: Monitor memory usage, trigger GC if needed
- **Data Buffering**: Limited buffer size to prevent OOM

## 📈 Progress Monitoring

### Real-Time Display
```
🚀 Schwab S&P 500 Price Data Download
=====================================
Progress: [████████████████████████████████████████] 342/503 (68.0%)
Current: Downloading NVDA (2019-2024)
Completed: 342 stocks | Failed: 3 stocks | Remaining: 158 stocks
Speed: 2.3 stocks/minute | ETA: 68 minutes
Data: 485,627 price records imported

Recent Activity:
✅ AAPL: 251 bars (2019-2024) - 0.8s
✅ MSFT: 249 bars (2019-2024) - 1.2s  
❌ INVALID: Symbol not found
✅ GOOGL: 252 bars (2019-2024) - 0.9s
🔄 NVDA: In progress...

Statistics:
- API Calls: 342/503 (68.0%)
- Success Rate: 99.1% (339/342)
- Average Speed: 1.2 seconds per stock
- Data Quality: 99.3% coverage
```

### Logging Strategy
```rust
// Structured logging with different levels
info!("Starting bulk download for {} symbols", symbol_count);
debug!("Fetching price data for {} from {} to {}", symbol, start_date, end_date);
warn!("Retrying {} after API error: {}", symbol, error);
error!("Failed to process {} after {} retries: {}", symbol, max_retries, error);
```

## 🎯 Implementation Phases

### Phase 1: Core Infrastructure (Day 1)
1. **Project Structure**: Create `import-schwab-prices` binary
2. **Configuration**: Environment variables and CLI arguments
3. **Symbol Loading**: S&P 500 symbol list from database
4. **Progress Tracking**: Basic progress display and persistence
5. **Rate Limiting**: Implement API rate limiting

**Deliverable**: Basic tool that can download single stock data with progress tracking

### Phase 2: Robust Error Handling (Day 1-2)
1. **Retry Logic**: Exponential backoff for API failures
2. **Circuit Breaker**: API failure protection
3. **Data Validation**: OHLCV data sanity checks
4. **Resume Capability**: Skip completed symbols on restart
5. **Comprehensive Logging**: Structured logging system

**Deliverable**: Production-ready download tool with error recovery

### Phase 3: Database Integration (Day 2)
1. **Batch Insertion**: Efficient database writes
2. **Conflict Resolution**: Handle duplicate dates
3. **Transaction Management**: Ensure data consistency
4. **Data Validation**: Verify inserted records
5. **Performance Optimization**: Connection pooling, prepared statements

**Deliverable**: Complete integration with database schema

### Phase 4: Testing & Validation (Day 2-3)
1. **Unit Tests**: Core component testing
2. **Integration Tests**: End-to-end workflow testing
3. **Performance Tests**: Load testing with subset of symbols
4. **Data Quality Tests**: Validate downloaded data accuracy
5. **Recovery Tests**: Test resume and retry capabilities

**Deliverable**: Thoroughly tested, production-ready system

### Phase 5: Production Deployment (Day 3)
1. **Configuration**: Production environment setup
2. **Monitoring**: Real-time progress monitoring
3. **Backup Strategy**: Database backup before bulk import
4. **Execution**: Full S&P 500 download
5. **Validation**: Post-download data quality assessment

**Deliverable**: Complete historical price data for all S&P 500 stocks

## 🛡️ Risk Mitigation

### Technical Risks

#### API Rate Limiting
- **Risk**: Exceeding 120 requests/minute limit
- **Mitigation**: Built-in rate limiter with 10% safety margin
- **Monitoring**: Track API call frequency in real-time

#### Token Expiration
- **Risk**: OAuth tokens expire during long download
- **Mitigation**: Automatic token refresh with 5-minute buffer
- **Fallback**: Manual token refresh instructions

#### Network Instability
- **Risk**: Network issues causing failed downloads
- **Mitigation**: Exponential backoff retry with circuit breaker
- **Recovery**: Resume capability for interrupted downloads

#### Database Corruption
- **Risk**: Database corruption during bulk insert
- **Mitigation**: Transaction-based inserts with rollback capability
- **Backup**: Automatic database backup before bulk operations

### Operational Risks

#### Incomplete Data
- **Risk**: Missing trading days or invalid price data
- **Mitigation**: Data validation with 99% coverage requirement
- **Quality Gates**: Reject stocks with <95% data coverage

#### Memory Usage
- **Risk**: Out-of-memory errors during bulk processing
- **Mitigation**: Streaming processing, memory monitoring
- **Limits**: Process one symbol at a time, limited buffer size

#### Disk Space
- **Risk**: Insufficient disk space for 754K price records
- **Mitigation**: Pre-flight disk space check (require 1GB free)
- **Monitoring**: Monitor disk usage during download

## 📊 Quality Assurance

### Data Quality Metrics
1. **Completeness**: >99% of expected trading days
2. **Accuracy**: Price values within reasonable ranges
3. **Consistency**: No gaps in date sequences
4. **Integrity**: All OHLCV fields populated

### Validation Rules
```rust
fn validate_price_bar(bar: &SchwabPriceBar, symbol: &str) -> ValidationResult {
    // Price sanity checks
    if bar.open <= 0.0 || bar.high <= 0.0 || bar.low <= 0.0 || bar.close <= 0.0 {
        return ValidationResult::Error("Invalid price: negative or zero".to_string());
    }
    
    // OHLC relationship validation
    if bar.high < bar.low || bar.high < bar.open || bar.high < bar.close {
        return ValidationResult::Warning("Invalid OHLC relationship".to_string());
    }
    
    // Volume sanity check
    if bar.volume < 0 {
        return ValidationResult::Error("Invalid volume: negative".to_string());
    }
    
    ValidationResult::Valid
}
```

### Success Criteria
- ✅ **Data Coverage**: 503 S&P 500 stocks with >99% trading day coverage
- ✅ **Data Quality**: <1% validation errors or warnings
- ✅ **Performance**: Complete download in <30 minutes
- ✅ **Reliability**: <0.1% unrecoverable failures
- ✅ **Integration**: Seamless database integration without conflicts

## 🔧 Configuration & Deployment

### Environment Variables
```bash
# Required
SCHWAB_API_KEY=your_api_key
SCHWAB_APP_SECRET=your_app_secret
SCHWAB_TOKEN_PATH=schwab_tokens.json

# Optional
DATABASE_URL=sqlite:./db/stocks.db
RATE_LIMIT_PER_MINUTE=120
BATCH_SIZE=100
MAX_RETRIES=3
PROGRESS_FILE=./progress.json
LOG_LEVEL=info
```

### CLI Interface
```bash
# Basic usage
cargo run --bin import-schwab-prices

# Custom date range
cargo run --bin import-schwab-prices --start-date 2020-01-01 --end-date 2023-12-31

# Resume interrupted download
cargo run --bin import-schwab-prices --resume

# Test mode (single symbol)
cargo run --bin import-schwab-prices --test-symbol AAPL

# Validation only (no download)
cargo run --bin import-schwab-prices --validate-only
```

### Monitoring Dashboard
```
🚀 Schwab Bulk Download Monitor
================================
Status: RUNNING | Session: a1b2c3d4
Started: 2024-09-18 04:16:13 | Runtime: 42m 15s

Progress Overview:
├─ Total Symbols: 503
├─ Completed: 342 (68.0%) ████████████████████████████
├─ Failed: 3 (0.6%) █
├─ Remaining: 158 (31.4%) ████████████

Performance Metrics:
├─ Download Speed: 2.3 stocks/minute
├─ API Success Rate: 99.1%
├─ Data Quality Score: 99.3%
├─ ETA: 68 minutes

Resource Usage:
├─ Memory: 245MB / 2GB (12.3%)
├─ CPU: 15.2%
├─ Disk: 892MB used
├─ Network: 1.2MB/s

Recent Activity:
├─ ✅ NVDA: 252 bars | 1.1s | Quality: 100%
├─ ✅ TSLA: 251 bars | 0.9s | Quality: 99.6%
├─ ✅ META: 248 bars | 1.3s | Quality: 98.4%
└─ 🔄 AMZN: Downloading...
```

## 📋 Implementation Checklist

### Prerequisites
- [ ] Environment variables configured
- [ ] OAuth tokens valid (>1 hour remaining)
- [ ] Database backup created
- [ ] Sufficient disk space (>1GB free)
- [ ] S&P 500 symbols available in database

### Core Implementation
- [ ] Symbol manager with progress tracking
- [ ] Rate-limited Schwab API client
- [ ] Retry logic with exponential backoff
- [ ] Progress persistence and resume capability
- [ ] Data validation and quality checks
- [ ] Batch database insertion
- [ ] Comprehensive error handling
- [ ] Real-time progress monitoring
- [ ] Structured logging system

### Testing & Validation
- [ ] Unit tests for core components
- [ ] Integration tests for API and database
- [ ] Test with single symbol (AAPL)
- [ ] Test with small subset (10 symbols)
- [ ] Test resume capability
- [ ] Test error recovery scenarios
- [ ] Performance testing

### Production Deployment
- [ ] Production configuration validation
- [ ] Database backup verification
- [ ] Monitoring system ready
- [ ] Execute full S&P 500 download
- [ ] Post-download data validation
- [ ] Performance metrics collection

## 🏁 Expected Outcomes

### Data Delivery
- **754,500+ price records** for S&P 500 stocks
- **2019-2024 coverage** aligning with EDGAR financial data  
- **99%+ data completeness** with comprehensive validation
- **Production-ready database** with historical price foundation

### System Capabilities
- **Resumable downloads** for operational flexibility
- **Error recovery** for unattended operations
- **Quality assurance** for reliable data
- **Performance optimization** for efficient operations

### Strategic Impact
- **Enhanced screening accuracy** with comprehensive price data
- **Foundation for advanced analytics** (technical indicators, volatility analysis)
- **Reduced data dependencies** on external APIs
- **Improved user experience** with faster query responses

This architecture provides a robust, scalable foundation for downloading and managing large-scale historical price data while maintaining the highest standards of reliability and data quality.