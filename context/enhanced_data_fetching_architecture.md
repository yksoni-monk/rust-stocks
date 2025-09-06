# Enhanced Data Fetching Architecture
## Alpha Vantage Primary + Schwab Backup System

**Version**: 2.0  
**Last Updated**: 2025-09-06  
**Author**: System Architecture Team

## Overview

This document outlines the enhanced data fetching architecture that positions Alpha Vantage as the primary data source for OHLCV price data with calculated daily P/E ratios, while maintaining Schwab API as a backup system.

## System Requirements

### Primary Objectives
1. **Comprehensive Historical Data**: Fetch 20+ years of OHLCV data using Alpha Vantage TIME_SERIES_DAILY API
2. **Daily P/E Calculations**: Calculate accurate P/E ratios for every trading day using quarterly EPS data
3. **Dual Data Size Options**: Support both "compact" (100 days) and "full" (20+ years) data fetching
4. **Backup Data Source**: Maintain Schwab API integration as fallback
5. **Efficient Storage**: Optimize database schema for large-scale historical data

### User Experience Goals
- Simple UI with compact/full options (no date selection)
- Single-click data fetching for individual stocks or bulk operations
- Real-time progress tracking for large data operations
- Clear status indicators for data completeness

## Data Source Strategy

### Primary: Alpha Vantage API
**Endpoint**: `TIME_SERIES_DAILY`
- **Compact Mode**: Latest 100 trading days
- **Full Mode**: 20+ years of historical data
- **Data Quality**: Raw as-traded OHLCV data
- **Rate Limits**: 5 calls/minute (free), higher for premium

### Secondary: Schwab API (Backup)
- Maintained for fallback scenarios
- Used when Alpha Vantage is unavailable
- Provides fundamental data validation
- Supports real-time data needs

### Earnings Data: Alpha Vantage EARNINGS API
- Quarterly and annual EPS data
- Used for historical P/E ratio calculations
- Cached locally to minimize API calls

## Database Schema Enhancement

### Current Schema Analysis
The existing `daily_prices` table has comprehensive fields but needs optimization for Alpha Vantage data:

```sql
-- Current daily_prices table (needs migration)
CREATE TABLE daily_prices (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER NOT NULL,
    date DATE NOT NULL,
    open_price REAL NOT NULL,
    high_price REAL NOT NULL,
    low_price REAL NOT NULL,
    close_price REAL NOT NULL,
    volume INTEGER,
    pe_ratio REAL,                    -- Will be calculated from EPS
    market_cap REAL,                  -- Calculated from price * shares
    dividend_yield REAL,              -- From earnings data
    eps REAL,                         -- From quarterly earnings
    beta REAL,                        -- Static/calculated metric
    week_52_high REAL,               -- Rolling calculation
    week_52_low REAL,                -- Rolling calculation
    pb_ratio REAL,                   -- Calculated metric
    ps_ratio REAL,                   -- Calculated metric
    shares_outstanding REAL,         -- From company data
    profit_margin REAL,              -- From earnings data
    operating_margin REAL,           -- From earnings data
    return_on_equity REAL,           -- Calculated metric
    return_on_assets REAL,           -- Calculated metric
    debt_to_equity REAL,             -- From balance sheet
    dividend_per_share REAL,         -- From earnings data
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, date)
);
```

### New Schema Enhancement

#### 1. Add Earnings Cache Table
```sql
CREATE TABLE earnings_data (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER NOT NULL,
    fiscal_date_ending DATE NOT NULL,
    reported_date DATE,
    reported_eps REAL NOT NULL,
    estimated_eps REAL,
    surprise REAL,
    surprise_percentage REAL,
    report_time TEXT,
    earnings_type TEXT NOT NULL, -- 'quarterly' or 'annual'
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, fiscal_date_ending, earnings_type)
);
```

#### 2. Add Data Source Tracking
```sql
ALTER TABLE daily_prices ADD COLUMN data_source TEXT DEFAULT 'alpha_vantage';
ALTER TABLE daily_prices ADD COLUMN last_updated DATETIME DEFAULT CURRENT_TIMESTAMP;
```

#### 3. Add Processing Metadata
```sql
CREATE TABLE processing_status (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER NOT NULL,
    data_type TEXT NOT NULL, -- 'prices', 'earnings', 'fundamentals'
    status TEXT NOT NULL, -- 'pending', 'processing', 'completed', 'failed'
    fetch_mode TEXT, -- 'compact', 'full'
    records_processed INTEGER DEFAULT 0,
    total_records INTEGER DEFAULT 0,
    error_message TEXT,
    started_at DATETIME,
    completed_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, data_type)
);
```

## API Integration Architecture

### Alpha Vantage Client Enhancement

#### Core Methods
```rust
impl AlphaVantageClient {
    // Enhanced daily data fetching
    pub async fn fetch_comprehensive_daily_data(&self, symbol: &str, output_size: DataFetchMode) -> Result<ComprehensiveStockData, String>;
    
    // Batch processing for multiple symbols
    pub async fn fetch_bulk_data(&self, symbols: Vec<String>, output_size: DataFetchMode) -> Result<BulkFetchResult, String>;
    
    // P/E ratio calculation for date ranges
    pub async fn calculate_pe_ratios_for_range(&self, symbol: &str, start_date: NaiveDate, end_date: NaiveDate) -> Result<Vec<DailyPERatio>, String>;
    
    // Data validation and quality checks
    pub fn validate_data_quality(&self, data: &ConvertedDailyPrice) -> DataQualityReport;
}
```

#### Data Structures
```rust
#[derive(Debug, Clone)]
pub enum DataFetchMode {
    Compact,  // 100 days
    Full,     // 20+ years
}

#[derive(Debug)]
pub struct ComprehensiveStockData {
    pub symbol: String,
    pub daily_prices: Vec<ConvertedDailyPrice>,
    pub earnings_data: AlphaVantageEarningsResponse,
    pub calculated_pe_ratios: Vec<DailyPERatio>,
    pub data_quality: DataQualityReport,
    pub fetch_metadata: FetchMetadata,
}

#[derive(Debug)]
pub struct DailyPERatio {
    pub date: NaiveDate,
    pub pe_ratio: Option<f64>,
    pub eps_used: Option<f64>,
    pub closing_price: f64,
    pub calculation_method: PECalculationMethod,
}

#[derive(Debug)]
pub enum PECalculationMethod {
    QuarterlyEPS(NaiveDate), // Date of the quarterly earnings used
    DefaultValue(f64),       // Default value when no EPS available
    Interpolated,            // Interpolated between quarters
}

#[derive(Debug)]
pub struct DataQualityReport {
    pub total_records: usize,
    pub missing_data_points: Vec<NaiveDate>,
    pub data_anomalies: Vec<DataAnomaly>,
    pub pe_calculation_coverage: f64, // Percentage of dates with calculated P/E
}
```

## Enhanced Fetching Logic

### Single Stock Fetch Process
1. **Initialize Processing Status**
   ```rust
   // Set status to 'processing'
   update_processing_status(stock_id, "prices", "processing", fetch_mode);
   ```

2. **Fetch Daily Price Data**
   ```rust
   let daily_data = alpha_client.get_daily_data(symbol, fetch_mode).await?;
   let converted_prices = alpha_client.convert_daily_data(&daily_data)?;
   ```

3. **Store Price Data in Database**
   ```rust
   for price_data in converted_prices {
       insert_daily_price(stock_id, &price_data, "alpha_vantage").await?;
   }
   ```

4. **Fetch and Cache Earnings Data**
   ```rust
   let earnings = alpha_client.get_earnings(symbol).await?;
   cache_earnings_data(stock_id, &earnings).await?;
   ```

5. **Calculate Daily P/E Ratios**
   ```rust
   for price_data in price_range {
       let pe_ratio = calculate_pe_for_date(symbol, price_data.date, &earnings)?;
       update_daily_price_pe_ratio(stock_id, price_data.date, pe_ratio).await?;
   }
   ```

6. **Update Processing Status**
   ```rust
   update_processing_status(stock_id, "prices", "completed", fetch_mode);
   ```

### Bulk Fetch Process
```rust
pub async fn fetch_all_stocks_data(fetch_mode: DataFetchMode) -> Result<BulkProcessingResult, String> {
    let stocks = get_available_stock_symbols().await?;
    let mut results = Vec::new();
    
    // Process in batches to respect API limits
    for batch in stocks.chunks(5) { // 5 stocks per minute due to rate limits
        for stock in batch {
            let result = fetch_single_comprehensive_data(stock, fetch_mode.clone()).await;
            results.push(result);
        }
        
        // Rate limiting: wait 1 minute between batches
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
    
    Ok(BulkProcessingResult { 
        results,
        total_processed: stocks.len(),
        success_count: results.iter().filter(|r| r.is_ok()).count(),
        error_count: results.iter().filter(|r| r.is_err()).count(),
    })
}
```

## P/E Ratio Calculation Logic

### Daily P/E Calculation Algorithm
```rust
pub fn calculate_pe_for_date(
    earnings_data: &[QuarterlyEarning], 
    target_date: NaiveDate, 
    closing_price: f64
) -> Option<f64> {
    // 1. Find the most recent quarterly EPS <= target_date
    let latest_eps = earnings_data
        .iter()
        .filter(|e| parse_date(&e.fiscal_date_ending) <= target_date)
        .max_by_key(|e| parse_date(&e.fiscal_date_ending))?;
    
    // 2. Parse EPS value
    let eps = latest_eps.reported_eps.parse::<f64>().ok()?;
    
    // 3. Calculate P/E ratio
    if eps > 0.0 {
        Some(closing_price / eps)
    } else {
        None // Don't calculate P/E for negative earnings
    }
}

// Default P/E handling
pub fn get_pe_ratio_or_default(calculated_pe: Option<f64>) -> f64 {
    calculated_pe.unwrap_or(0.0) // Use 0.0 as default when no EPS available
}
```

### P/E Data Coverage Strategy
- **Full Coverage**: Calculate P/E for all dates with price data
- **Missing EPS Handling**: Use 0.0 default for dates without earnings data
- **Negative EPS**: Set P/E to 0.0 for negative earnings
- **Data Quality Tracking**: Track percentage of successful P/E calculations

## Tauri Commands Architecture

### Enhanced Commands
```rust
// Primary fetch commands
#[tauri::command]
pub async fn fetch_stock_data_comprehensive(
    symbol: String, 
    fetch_mode: String // "compact" or "full"
) -> Result<String, String>;

#[tauri::command]
pub async fn fetch_all_stocks_comprehensive(
    fetch_mode: String
) -> Result<String, String>;

// Progress tracking commands
#[tauri::command]
pub async fn get_fetch_progress(stock_id: Option<i64>) -> Result<ProcessingStatus, String>;

#[tauri::command]
pub async fn get_bulk_fetch_progress() -> Result<BulkProcessingStatus, String>;

// Data validation commands
#[tauri::command]
pub async fn validate_stock_data(symbol: String) -> Result<DataQualityReport, String>;

// Backup/fallback commands (existing Schwab API)
#[tauri::command]
pub async fn fetch_stock_data_schwab_fallback(
    symbol: String,
    start_date: String,
    end_date: String
) -> Result<String, String>;
```

## Frontend UI Enhancement

### Data Fetch Panel Design
```jsx
// Enhanced DataFetchingPanel.jsx
function DataFetchingPanel({ stock }) {
  const [fetchMode, setFetchMode] = useState('compact');
  const [loading, setLoading] = useState(false);
  const [progress, setProgress] = useState(null);

  const handleFetchData = async () => {
    try {
      setLoading(true);
      const result = await invoke('fetch_stock_data_comprehensive', {
        symbol: stock.symbol,
        fetchMode: fetchMode
      });
      
      // Poll for progress updates
      const progressInterval = setInterval(async () => {
        const progress = await invoke('get_fetch_progress', { 
          stockId: stock.id 
        });
        setProgress(progress);
        
        if (progress.status === 'completed' || progress.status === 'failed') {
          clearInterval(progressInterval);
          setLoading(false);
        }
      }, 1000);
      
    } catch (error) {
      setLoading(false);
      console.error('Fetch failed:', error);
    }
  };

  return (
    <div className="enhanced-fetch-panel">
      <h3>Comprehensive Data Fetching</h3>
      
      {/* Fetch Mode Selection */}
      <div className="fetch-mode-selector">
        <label>
          <input 
            type="radio" 
            value="compact" 
            checked={fetchMode === 'compact'}
            onChange={(e) => setFetchMode(e.target.value)}
          />
          Compact (100 days)
        </label>
        <label>
          <input 
            type="radio" 
            value="full" 
            checked={fetchMode === 'full'}
            onChange={(e) => setFetchMode(e.target.value)}
          />
          Full (20+ years)
        </label>
      </div>

      {/* Fetch Button */}
      <button onClick={handleFetchData} disabled={loading}>
        {loading ? 'Fetching...' : `Fetch ${fetchMode} Data`}
      </button>

      {/* Progress Display */}
      {progress && (
        <div className="progress-display">
          <div>Status: {progress.status}</div>
          <div>Records: {progress.records_processed} / {progress.total_records}</div>
          {progress.error_message && (
            <div className="error">Error: {progress.error_message}</div>
          )}
        </div>
      )}

      {/* Data Info */}
      <div className="data-info">
        <h4>What will be fetched:</h4>
        <ul>
          <li>Daily OHLCV price data ({fetchMode === 'full' ? '20+ years' : '100 days'})</li>
          <li>Quarterly earnings data (for P/E calculations)</li>
          <li>Calculated daily P/E ratios</li>
          <li>Data quality metrics</li>
        </ul>
      </div>
    </div>
  );
}
```

### Bulk Operations Panel
```jsx
function BulkFetchPanel({ stocks }) {
  return (
    <div className="bulk-fetch-panel">
      <h3>Bulk Data Fetching</h3>
      
      <div className="bulk-options">
        <select value={fetchMode} onChange={(e) => setFetchMode(e.target.value)}>
          <option value="compact">Compact (100 days) - All {stocks.length} stocks</option>
          <option value="full">Full (20+ years) - All {stocks.length} stocks</option>
        </select>
        
        <button onClick={handleBulkFetch} disabled={loading}>
          {loading ? 'Processing...' : 'Fetch All Stocks'}
        </button>
      </div>

      {/* Progress tracking for bulk operations */}
      {bulkProgress && <BulkProgressDisplay progress={bulkProgress} />}
      
      {/* Rate limit warning */}
      <div className="rate-limit-warning">
        ⚠️ Bulk fetching respects API rate limits (5 calls/minute).
        Full mode for all stocks will take approximately {Math.ceil(stocks.length / 5)} minutes.
      </div>
    </div>
  );
}
```

## Database Migration Strategy

### Migration Plan
1. **Backup Current Data**
   ```sql
   -- Export current data
   .backup stocks_backup_$(date +%Y%m%d).db
   ```

2. **Clean Existing Data**
   ```sql
   DELETE FROM daily_prices; -- Remove existing price data
   DELETE FROM metadata WHERE key LIKE 'last_fetch_%';
   ```

3. **Create New Tables**
   ```sql
   CREATE TABLE earnings_data (...);
   CREATE TABLE processing_status (...);
   ALTER TABLE daily_prices ADD COLUMN data_source TEXT DEFAULT 'alpha_vantage';
   ALTER TABLE daily_prices ADD COLUMN last_updated DATETIME DEFAULT CURRENT_TIMESTAMP;
   ```

4. **Create Indexes for Performance**
   ```sql
   CREATE INDEX idx_earnings_stock_date ON earnings_data(stock_id, fiscal_date_ending);
   CREATE INDEX idx_processing_status ON processing_status(stock_id, data_type, status);
   CREATE INDEX idx_daily_prices_source ON daily_prices(data_source);
   ```

## Error Handling Strategy

### API Error Handling
```rust
#[derive(Debug)]
pub enum DataFetchError {
    AlphaVantageRateLimit,
    AlphaVantageInvalidSymbol(String),
    SchwabAPIError(String),
    DatabaseError(String),
    DataParsingError(String),
    NetworkError(String),
}

impl DataFetchError {
    pub fn should_retry(&self) -> bool {
        matches!(self, 
            DataFetchError::AlphaVantageRateLimit |
            DataFetchError::NetworkError(_)
        )
    }
    
    pub fn fallback_to_schwab(&self) -> bool {
        matches!(self, 
            DataFetchError::AlphaVantageRateLimit |
            DataFetchError::AlphaVantageInvalidSymbol(_)
        )
    }
}
```

### Retry Logic with Exponential Backoff
```rust
pub async fn fetch_with_retry<T, F, Fut>(
    operation: F,
    max_retries: u32,
) -> Result<T, DataFetchError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, DataFetchError>>,
{
    let mut delay = Duration::from_secs(1);
    
    for attempt in 0..=max_retries {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(error) => {
                if attempt == max_retries || !error.should_retry() {
                    return Err(error);
                }
                
                tokio::time::sleep(delay).await;
                delay *= 2; // Exponential backoff
            }
        }
    }
    
    unreachable!()
}
```

## Performance Optimizations

### Database Optimizations
1. **Batch Inserts**: Use transaction batching for large datasets
2. **Prepared Statements**: Pre-compile frequently used queries
3. **Index Strategy**: Optimize indexes for common query patterns
4. **Connection Pooling**: Reuse database connections

### API Optimizations
1. **Rate Limit Management**: Implement smart rate limiting with queuing
2. **Parallel Processing**: Process non-dependent operations concurrently
3. **Data Caching**: Cache earnings data locally to reduce API calls
4. **Incremental Updates**: Only fetch new data, not full historical refetch

### Memory Optimizations
1. **Streaming Processing**: Process large datasets in chunks
2. **Data Compression**: Compress stored price data when possible
3. **Memory Pool Management**: Reuse allocated memory for bulk operations

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pe_calculation_with_valid_eps() {
        let earnings_data = vec![/* test data */];
        let target_date = NaiveDate::from_ymd(2024, 6, 15);
        let closing_price = 150.0;
        
        let pe_ratio = calculate_pe_for_date(&earnings_data, target_date, closing_price);
        assert!(pe_ratio.is_some());
        assert_eq!(pe_ratio.unwrap(), 15.0); // Assuming EPS = 10.0
    }

    #[tokio::test]
    async fn test_data_quality_validation() {
        let mock_data = create_mock_daily_prices();
        let quality_report = validate_data_quality(&mock_data);
        
        assert!(quality_report.pe_calculation_coverage > 0.8);
        assert!(quality_report.missing_data_points.is_empty());
    }
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_comprehensive_fetch_integration() {
    let client = AlphaVantageClient::new("demo".to_string());
    let result = client.fetch_comprehensive_daily_data("AAPL", DataFetchMode::Compact).await;
    
    assert!(result.is_ok());
    let data = result.unwrap();
    assert!(!data.daily_prices.is_empty());
    assert!(!data.earnings_data.quarterly_earnings.is_empty());
}
```

## Monitoring and Logging

### Logging Strategy
```rust
use tracing::{info, warn, error, debug};

pub async fn fetch_stock_data_with_logging(symbol: &str) -> Result<(), DataFetchError> {
    info!("Starting comprehensive data fetch for {}", symbol);
    
    let start_time = Instant::now();
    
    match fetch_comprehensive_data(symbol).await {
        Ok(data) => {
            info!(
                "Successfully fetched {} price records for {} in {:?}",
                data.daily_prices.len(),
                symbol,
                start_time.elapsed()
            );
        }
        Err(error) => {
            error!("Failed to fetch data for {}: {:?}", symbol, error);
            return Err(error);
        }
    }
    
    Ok(())
}
```

### Metrics Collection
```rust
pub struct FetchMetrics {
    pub total_api_calls: u64,
    pub successful_fetches: u64,
    pub failed_fetches: u64,
    pub average_response_time: Duration,
    pub data_points_processed: u64,
    pub pe_calculations_completed: u64,
}
```

## Security Considerations

### API Key Management
```rust
// Environment-based configuration
pub struct Config {
    pub alpha_vantage_api_key: String,
    pub schwab_client_id: String,
    pub schwab_client_secret: String,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Config {
            alpha_vantage_api_key: env::var("ALPHA_VANTAGE_API_KEY")
                .map_err(|_| ConfigError::MissingApiKey("ALPHA_VANTAGE_API_KEY"))?,
            schwab_client_id: env::var("SCHWAB_CLIENT_ID")
                .map_err(|_| ConfigError::MissingApiKey("SCHWAB_CLIENT_ID"))?,
            schwab_client_secret: env::var("SCHWAB_CLIENT_SECRET")
                .map_err(|_| ConfigError::MissingApiKey("SCHWAB_CLIENT_SECRET"))?,
        })
    }
}
```

### Input Validation
```rust
pub fn validate_symbol(symbol: &str) -> Result<String, ValidationError> {
    let cleaned = symbol.trim().to_uppercase();
    
    if cleaned.is_empty() || cleaned.len() > 10 {
        return Err(ValidationError::InvalidSymbol("Invalid symbol length"));
    }
    
    if !cleaned.chars().all(|c| c.is_alphabetic()) {
        return Err(ValidationError::InvalidSymbol("Symbol must contain only letters"));
    }
    
    Ok(cleaned)
}
```

## Deployment Strategy

### Environment Configuration
```bash
# .env file
ALPHA_VANTAGE_API_KEY=your_alpha_vantage_key_here
SCHWAB_CLIENT_ID=your_schwab_client_id
SCHWAB_CLIENT_SECRET=your_schwab_client_secret

# Database configuration
DATABASE_URL=sqlite:./stocks.db
DATABASE_POOL_SIZE=10

# Logging configuration
RUST_LOG=info
LOG_FILE=./logs/stock_fetcher.log
```

### Production Deployment Checklist
- [ ] API keys configured securely
- [ ] Database migrations completed
- [ ] Rate limiting configured appropriately
- [ ] Logging and monitoring setup
- [ ] Error alerting configured
- [ ] Backup strategy implemented
- [ ] Performance benchmarks established

## Future Enhancements

### Phase 1 (Current Implementation)
- [ ] Alpha Vantage integration
- [ ] Daily P/E calculations
- [ ] Compact/Full fetch modes
- [ ] Database schema migration

### Phase 2 (Next Quarter)
- [ ] Real-time data updates
- [ ] Advanced technical indicators
- [ ] Data export functionality
- [ ] Enhanced error recovery

### Phase 3 (Future)
- [ ] Machine learning predictions
- [ ] Alternative data sources
- [ ] Advanced analytics dashboard
- [ ] Mobile application support

---

**Status**: Ready for Implementation  
**Priority**: High  
**Estimated Timeline**: 2-3 weeks  
**Dependencies**: Alpha Vantage API access, Database migration completion