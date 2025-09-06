# Alpha Vantage API Architecture

## Overview
This document outlines the Alpha Vantage API integration architecture for the Rust Stocks application, including data structures, methods, and usage patterns.

## API Endpoints

### 1. Earnings Data API
**Endpoint**: `https://www.alphavantage.co/query?function=EARNINGS&symbol={SYMBOL}&apikey={API_KEY}`

**Purpose**: Fetch quarterly and annual earnings per share (EPS) data for P/E ratio calculations.

**Response Structure**:
```json
{
  "symbol": "IBM",
  "annualEarnings": [
    {
      "fiscalDateEnding": "2024-12-31",
      "reportedEPS": "10.33"
    }
  ],
  "quarterlyEarnings": [
    {
      "fiscalDateEnding": "2024-12-31",
      "reportedDate": "2025-01-29",
      "reportedEPS": "3.92",
      "estimatedEPS": "3.78",
      "surprise": "0.14",
      "surprisePercentage": "3.7037",
      "reportTime": "post-market"
    }
  ]
}
```

### 2. Daily Price Data API
**Endpoint**: `https://www.alphavantage.co/query?function=TIME_SERIES_DAILY&symbol={SYMBOL}&outputsize={SIZE}&apikey={API_KEY}`

**Purpose**: Fetch daily OHLCV price data for historical analysis.

**Response Structure**:
```json
{
  "Meta Data": {
    "1. Information": "Daily Prices (open, high, low, close) and Volumes",
    "2. Symbol": "IBM",
    "3. Last Refreshed": "2025-09-05",
    "4. Output Size": "Compact",
    "5. Time Zone": "US/Eastern"
  },
  "Time Series (Daily)": {
    "2025-09-05": {
      "1. open": "248.2300",
      "2. high": "249.0300",
      "3. low": "245.4500",
      "4. close": "248.5300",
      "5. volume": "3147478"
    }
  }
}
```

## Data Structures

### Earnings Data Structures
```rust
#[derive(Debug, Deserialize)]
pub struct AlphaVantageEarningsResponse {
    pub symbol: String,
    pub annual_earnings: Vec<AnnualEarning>,
    pub quarterly_earnings: Vec<QuarterlyEarning>,
}

#[derive(Debug, Deserialize)]
pub struct AnnualEarning {
    pub fiscal_date_ending: String,
    pub reported_eps: String,
}

#[derive(Debug, Deserialize)]
pub struct QuarterlyEarning {
    pub fiscal_date_ending: String,
    pub reported_date: String,
    pub reported_eps: String,
    pub estimated_eps: Option<String>,
    pub surprise: Option<String>,
    pub surprise_percentage: Option<String>,
    pub report_time: Option<String>,
}
```

### Daily Price Data Structures
```rust
#[derive(Debug, Deserialize)]
pub struct AlphaVantageDailyResponse {
    #[serde(rename = "Meta Data")]
    pub meta_data: DailyMetaData,
    #[serde(rename = "Time Series (Daily)")]
    pub time_series: HashMap<String, DailyPriceData>,
}

#[derive(Debug, Deserialize)]
pub struct DailyMetaData {
    #[serde(rename = "1. Information")]
    pub information: String,
    #[serde(rename = "2. Symbol")]
    pub symbol: String,
    #[serde(rename = "3. Last Refreshed")]
    pub last_refreshed: String,
    #[serde(rename = "4. Output Size")]
    pub output_size: String,
    #[serde(rename = "5. Time Zone")]
    pub time_zone: String,
}

#[derive(Debug, Deserialize)]
pub struct DailyPriceData {
    #[serde(rename = "1. open")]
    pub open: String,
    #[serde(rename = "2. high")]
    pub high: String,
    #[serde(rename = "3. low")]
    pub low: String,
    #[serde(rename = "4. close")]
    pub close: String,
    #[serde(rename = "5. volume")]
    pub volume: String,
}

#[derive(Debug, Clone)]
pub struct ConvertedDailyPrice {
    pub date: NaiveDate,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: i64,
}
```

## Core Methods

### AlphaVantageClient Methods

#### Earnings Methods
- **`get_earnings_history(symbol: &str)`**: Fetch quarterly and annual earnings data
- **`print_earnings_data()`**: Display formatted earnings data to console

#### Daily Price Methods
- **`get_daily_data(symbol: &str, output_size: Option<&str>)`**: Fetch daily OHLCV data
- **`convert_daily_data()`**: Convert string-based API response to typed internal format
- **`print_daily_data()`**: Display formatted daily data to console

#### P/E Ratio Calculation Methods (To Be Implemented)
- **`get_eps_for_date(symbol: &str, date: NaiveDate)`**: Get latest EPS for a given date
- **`calculate_daily_pe_ratio(symbol: &str, date: NaiveDate)`**: Calculate P/E ratio for a specific date

## P/E Ratio Calculation Logic

### Algorithm
1. **Fetch Earnings Data**: Get quarterly earnings history for the symbol
2. **Find Latest EPS**: For a given date, find the most recent quarterly EPS report that is ≤ the input date
3. **Fetch Daily Price**: Get the closing price for the specified date
4. **Calculate P/E**: P/E = Closing Price / EPS

### Implementation Strategy
```rust
pub async fn calculate_daily_pe_ratio(&self, symbol: &str, date: NaiveDate) -> Result<f64, String> {
    // 1. Get earnings data
    let earnings_data = self.get_earnings_history(symbol).await?;
    
    // 2. Find latest EPS for the date
    let eps = self.get_eps_for_date(&earnings_data, date)?;
    
    // 3. Get daily price data
    let daily_data = self.get_daily_data(symbol, Some("compact")).await?;
    
    // 4. Find closing price for the date
    let closing_price = self.get_closing_price_for_date(&daily_data, date)?;
    
    // 5. Calculate P/E ratio
    Ok(closing_price / eps)
}
```

## Tauri Commands

### Available Commands
- **`test_alpha_vantage_earnings(symbol: String)`**: Test earnings data fetching
- **`test_alpha_vantage_daily(symbol: String, output_size: Option<String>)`**: Test daily data fetching
- **`calculate_daily_pe_ratio(symbol: String, date: String)`**: Calculate P/E ratio for a specific date (To Be Implemented)

## Configuration

### Environment Variables
```bash
ALPHA_VANTAGE_API_KEY=demo  # Replace with actual API key for production
```

### API Key Management
- **Demo Key**: Limited functionality, returns demo data with usage message
- **Free Key**: 5 API calls per minute, 500 calls per day
- **Premium Key**: Higher rate limits for production use

## Error Handling

### Common Error Scenarios
1. **API Rate Limiting**: Handle 429 status codes with exponential backoff
2. **Invalid Symbol**: Handle 404 or empty response for non-existent symbols
3. **Date Parsing**: Handle invalid date formats gracefully
4. **Missing Data**: Handle cases where EPS or price data is unavailable for specific dates

### Error Response Structure
```rust
pub enum AlphaVantageError {
    ApiError(String),
    ParseError(String),
    DataNotFound(String),
    RateLimitExceeded,
    InvalidSymbol(String),
}
```

## Usage Examples

### Fetch Earnings Data
```rust
let client = AlphaVantageClient::new(api_key);
let earnings = client.get_earnings_history("AAPL").await?;
client.print_earnings_data(&earnings);
```

### Fetch Daily Data
```rust
let client = AlphaVantageClient::new(api_key);
let daily_data = client.get_daily_data("AAPL", Some("compact")).await?;
let converted_data = client.convert_daily_data(&daily_data)?;
client.print_daily_data(&daily_data, &converted_data);
```

### Calculate P/E Ratio
```rust
let client = AlphaVantageClient::new(api_key);
let date = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
let pe_ratio = client.calculate_daily_pe_ratio("AAPL", date).await?;
println!("P/E Ratio for AAPL on 2024-12-31: {:.2}", pe_ratio);
```

## Future Enhancements

### Planned Features
1. **Caching**: Implement local caching for earnings data to reduce API calls
2. **Batch Processing**: Support for calculating P/E ratios for multiple dates/symbols
3. **Historical P/E**: Calculate P/E ratios for entire date ranges
4. **Database Integration**: Store Alpha Vantage data in local database
5. **Real-time Updates**: Implement webhook-based real-time data updates

### Performance Optimizations
1. **Connection Pooling**: Reuse HTTP connections for multiple API calls
2. **Parallel Requests**: Fetch earnings and daily data concurrently
3. **Data Compression**: Compress large responses for storage
4. **Smart Caching**: Cache frequently accessed data with TTL

## Dependencies

### Required Crates
```toml
[dependencies]
reqwest = { version = "0.11", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1.0"
```

### Optional Dependencies
```toml
[dependencies]
tokio = { version = "1.0", features = ["full"] }  # For async runtime
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite"] }  # For database integration
```

## Testing Strategy

### Unit Tests
- Test data parsing for various API response formats
- Test date calculations and EPS lookup logic
- Test P/E ratio calculation accuracy

### Integration Tests
- Test API connectivity with demo key
- Test error handling for various failure scenarios
- Test performance with large datasets

### Mock Testing
- Mock API responses for consistent testing
- Test error scenarios without making actual API calls
- Validate data transformation logic

## Security Considerations

### API Key Security
- Store API keys in environment variables
- Never commit API keys to version control
- Rotate API keys regularly
- Use different keys for development and production

### Rate Limiting
- Implement exponential backoff for rate limit errors
- Monitor API usage to avoid exceeding limits
- Cache responses to minimize API calls

### Data Validation
- Validate all input parameters before API calls
- Sanitize symbol inputs to prevent injection attacks
- Validate date ranges to prevent excessive data requests

## Monitoring and Logging

### Logging Strategy
- Log all API requests and responses (with sensitive data redacted)
- Log performance metrics (response times, data sizes)
- Log error conditions with sufficient context for debugging

### Metrics to Track
- API call frequency and success rates
- Data processing times
- Cache hit/miss ratios
- Error rates by error type

## Documentation Updates

This document should be updated whenever:
- New API endpoints are added
- Data structures are modified
- New methods are implemented
- Configuration options change
- Error handling is enhanced

---

*Last Updated: 2025-09-06*
*Version: 1.0*
