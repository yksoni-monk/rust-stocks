# Schwab API Price Data Strategy - Historical OHLCV for S&P 500

## 📊 Executive Summary

Analysis shows that the Schwab API can provide comprehensive historical OHLCV data for all S&P 500 stocks, covering the exact timeframe of our EDGAR financial data (2019-2024). The existing implementation is already capable of handling this requirement.

## 🔍 Current State Analysis

### Existing Price Data Coverage
- **Date Range**: 2019-10-11 to 2024-09-13 (4.9 years)
- **Total Records**: 6,198,657 price records
- **Stocks Covered**: 5,876 stocks with price data
- **Trading Days**: 1,244 unique trading dates

### EDGAR Financial Data Coverage
- **Date Range**: 2019-10-31 to 2024-08-31 (4.8 years)  
- **Quarters**: 70 unique quarterly reporting periods
- **Records**: 115,137 earnings records
- **Coverage Overlap**: ✅ Perfect alignment with price data timeframe

## 🏗️ Existing Schwab API Infrastructure

### Schwab Client Implementation Status
✅ **Fully Implemented** in `/src-tauri/src/api/schwab_client.rs`

#### Available Functionality
- **Authentication**: OAuth token management with refresh capability
- **Rate Limiting**: Built-in API rate limiter
- **Price History**: `get_price_history()` method for OHLCV data
- **Market Data**: Real-time quotes and fundamental data
- **Error Handling**: Comprehensive error recovery

#### Price History Method Analysis
```rust
async fn get_price_history(
    &self,
    symbol: &str,
    from_date: NaiveDate,
    to_date: NaiveDate,
) -> Result<Vec<SchwabPriceBar>>
```

**Features:**
- ✅ **Date Range Queries**: Supports arbitrary start/end dates
- ✅ **Daily Frequency**: Returns daily OHLCV bars
- ✅ **Timestamp Conversion**: Handles millisecond timestamps
- ✅ **Complete OHLCV**: Open, High, Low, Close, Volume data
- ✅ **Batch Processing**: Can handle multiple symbol requests

### Required Environment Variables
```bash
SCHWAB_API_KEY=your_api_key
SCHWAB_APP_SECRET=your_app_secret  
SCHWAB_CALLBACK_URL=https://localhost:8080
SCHWAB_TOKEN_PATH=schwab_tokens.json
```

## 📈 Price Data Strategy for S&P 500

### Objective
Download complete historical OHLCV data for all S&P 500 stocks covering the same timeframe as EDGAR financial data (2019-2024).

### Data Requirements Analysis

#### Time Range Strategy
- **Start Date**: 2019-01-01 (before earliest financial data)
- **End Date**: 2024-12-31 (current year end)
- **Coverage**: ~6 years of daily price data
- **Trading Days**: ~1,500 trading days per stock

#### Volume Estimation
- **S&P 500 Stocks**: 503 companies
- **Price Records per Stock**: ~1,500 daily bars
- **Total Records**: ~754,500 price records
- **API Calls Required**: 503 calls (1 per stock for full history)

### API Rate Limiting Strategy

#### Schwab API Limits
- **Rate Limit**: 120 requests per minute (existing rate limiter)
- **Data Retrieval Time**: ~4.2 minutes for all S&P 500 stocks
- **Batch Size**: Can request full historical range per stock in single call

#### Optimization Approach
```rust
// Existing rate limiter handles this automatically
for symbol in sp500_symbols {
    let price_data = schwab_client.get_price_history(
        &symbol,
        NaiveDate::from_ymd(2019, 1, 1),
        NaiveDate::from_ymd(2024, 12, 31)
    ).await?;
    
    // Store in database
    store_price_data(&symbol, price_data).await?;
}
```

## 🗄️ Database Integration Strategy

### Table: `daily_prices`
Current schema supports all required fields:
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
    -- Additional fields for enhanced data
    pe_ratio REAL,
    market_cap REAL,
    dividend_yield REAL,
    -- ... other fields
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, date)
);
```

### Data Mapping: Schwab API → Database
```rust
SchwabPriceBar {
    datetime: i64,     // Convert to DATE
    open: f64,         // → open_price
    high: f64,         // → high_price  
    low: f64,          // → low_price
    close: f64,        // → close_price
    volume: i64,       // → volume
}
```

## 🚀 Implementation Plan

### Phase 1: Environment Setup
1. **Configure API Keys**: Set up Schwab API credentials
2. **Token Management**: Initialize OAuth token file
3. **Database Preparation**: Ensure daily_prices table is ready

### Phase 2: Data Collection Tool
```rust
// New binary: src/bin/import-schwab-prices.rs
async fn main() -> Result<()> {
    let config = Config::from_env()?;
    let schwab_client = SchwabClient::new(&config)?;
    let db = get_database_connection().await?;
    
    let sp500_symbols = get_sp500_symbols(&db).await?;
    
    for symbol in sp500_symbols {
        println!("📈 Downloading {} price history...", symbol);
        
        let price_data = schwab_client.get_price_history(
            &symbol,
            NaiveDate::from_ymd(2019, 1, 1),
            NaiveDate::from_ymd(2024, 12, 31)
        ).await?;
        
        import_price_data(&db, &symbol, &price_data).await?;
        
        println!("✅ Imported {} bars for {}", price_data.len(), symbol);
    }
    
    Ok(())
}
```

### Phase 3: Data Validation
1. **Coverage Check**: Verify all S&P 500 stocks have price data
2. **Date Alignment**: Ensure price data aligns with earnings dates  
3. **Quality Validation**: Check for missing trading days or outliers

### Phase 4: Integration Testing
1. **Screening Validation**: Test GARP/Graham screening with new price data
2. **Performance Testing**: Ensure database performance with larger dataset
3. **Data Consistency**: Validate calculations against existing results

## 📊 Data Quality Advantages

### Schwab vs Current Data Sources
| Aspect | Schwab API | Current Data | Winner |
|---|---|---|---|
| **Accuracy** | Official broker data | Mixed sources | 🏆 Schwab |
| **Coverage** | All S&P 500 + more | 5,876 stocks | 🏆 Schwab |
| **Reliability** | Direct from broker | Third-party aggregation | 🏆 Schwab |
| **Real-time** | Live market data | Delayed/batched | 🏆 Schwab |
| **Historical Depth** | Up to 20 years | Limited lookback | 🏆 Schwab |
| **Cost** | API subscription | Various costs | Neutral |

## ⚠️ Risk Considerations

### API Limitations
- **Rate Limits**: 120 requests/minute (manageable for S&P 500)
- **Data Costs**: Potential charges for historical data volume
- **Authentication**: OAuth token expiration and refresh requirements

### Technical Risks
- **Network Reliability**: Large data downloads may timeout
- **Database Storage**: ~750K new records require storage planning
- **Processing Time**: Initial backfill may take several hours

### Mitigation Strategies
- **Incremental Loading**: Download data in date ranges
- **Resume Capability**: Track progress and resume interrupted downloads
- **Data Validation**: Verify completeness before committing to database
- **Backup Strategy**: Create database backups before bulk imports

## 🎯 Success Metrics

### Data Completeness
- ✅ 503 S&P 500 stocks with complete price history
- ✅ 2019-2024 date range coverage (1,500+ trading days)
- ✅ <1% missing data tolerance
- ✅ All OHLCV fields populated

### Performance Requirements  
- ✅ Complete S&P 500 download in <30 minutes
- ✅ Database queries remain <1 second response time
- ✅ Memory usage <2GB during bulk import
- ✅ Error rate <0.1% for API calls

## 📋 Implementation Checklist

### Prerequisites
- [ ] Schwab API credentials configured
- [ ] OAuth token file generated (using Python script)
- [ ] Database backup created
- [ ] Sufficient disk space available (~500MB for price data)

### Development Tasks
- [ ] Create `import-schwab-prices` binary
- [ ] Implement incremental download logic
- [ ] Add progress tracking and resume capability
- [ ] Create data validation functions
- [ ] Add logging and error reporting

### Testing Requirements
- [ ] Test with single stock (AAPL) first
- [ ] Validate OHLCV data accuracy against known sources
- [ ] Performance test with 10-stock subset
- [ ] Full S&P 500 test run in development environment

### Deployment Tasks
- [ ] Production environment configuration
- [ ] Monitoring and alerting setup
- [ ] Documentation for operational procedures
- [ ] Rollback plan in case of issues

## 🏁 Conclusion

**The Schwab API strategy is highly viable** for obtaining comprehensive S&P 500 price data:

✅ **Technical Feasibility**: Existing infrastructure supports full implementation
✅ **Data Quality**: Superior to current mixed-source approach  
✅ **Scalability**: Can easily extend beyond S&P 500 to broader market
✅ **Integration**: Seamless fit with existing database schema and workflows
✅ **Timeline Alignment**: Perfect overlap with EDGAR financial data coverage

**Recommendation**: Proceed with implementation using the existing Schwab client infrastructure. The investment in comprehensive price data will significantly enhance the reliability and accuracy of all screening algorithms while providing a foundation for future expansion.

**Next Steps**: 
1. Set up Schwab API credentials and test with AAPL
2. Implement bulk download tool for S&P 500
3. Execute backfill for 2019-2024 historical data
4. Validate integration with existing screening features