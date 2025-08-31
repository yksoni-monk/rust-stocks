use anyhow::Result;
use chrono::{Duration, NaiveDate, Utc};
use futures::stream::{self, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{info, warn, error, debug};

use crate::api::{SchwabClient, StockDataProvider};
use crate::database::DatabaseManager;
use crate::models::{Config, Stock, DailyPrice, SchwabQuote};

/// Data collection system for fetching and storing stock data
pub struct DataCollector {
    schwab_client: Arc<SchwabClient>,
    database: Arc<DatabaseManager>,
    config: Config,
    concurrency_semaphore: Arc<Semaphore>,
}

impl DataCollector {
    /// Create a new data collector
    pub fn new(schwab_client: SchwabClient, database: DatabaseManager, config: Config) -> Self {
        let max_concurrent = std::cmp::min(config.batch_size, 10); // Limit concurrent requests
        
        Self {
            schwab_client: Arc::new(schwab_client),
            database: Arc::new(database),
            config,
            concurrency_semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    /// Get all active S&P 500 stocks from database
    pub fn get_active_stocks(&self) -> Result<Vec<Stock>> {
        info!("📊 Getting active stocks from database...");
        let stocks = self.database.get_active_stocks()?;
        info!("✅ Found {} active stocks in database", stocks.len());
        Ok(stocks)
    }

    /// Fetch current quotes for all active stocks
    pub async fn fetch_current_quotes(&self) -> Result<usize> {
        info!("📊 Fetching current quotes for all active stocks...");
        
        let stocks = self.database.get_active_stocks()?;
        let symbols: Vec<String> = stocks.iter().map(|s| s.symbol.clone()).collect();
        
        let mut updated_count = 0;
        
        // Process in batches to respect rate limits
        for chunk in symbols.chunks(self.config.batch_size) {
            let quotes = self.schwab_client.get_quotes(chunk).await?;
            updated_count += self.process_quotes(quotes, &stocks).await?;
            
            // Brief pause between batches
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        info!("✅ Updated quotes for {} stocks", updated_count);
        Ok(updated_count)
    }

    /// Process quotes and update database
    async fn process_quotes(&self, quotes: Vec<SchwabQuote>, stocks: &[Stock]) -> Result<usize> {
        let mut updated_count = 0;
        let today = Utc::now().date_naive();

        // Create lookup map for stock IDs
        let stock_lookup: HashMap<String, i64> = stocks
            .iter()
            .filter_map(|s| s.id.map(|id| (s.symbol.clone(), id)))
            .collect();

        for quote in quotes {
            if let Some(&stock_id) = stock_lookup.get(&quote.symbol) {
                // Check if we already have data for today
                if self.database.get_price_on_date(stock_id, today)?.is_some() {
                    debug!("Skipping {}: already have data for {}", quote.symbol, today);
                    continue;
                }

                let daily_price = DailyPrice {
                    id: None,
                    stock_id,
                    date: today,
                    open_price: quote.open_price.unwrap_or(quote.last_price),
                    high_price: quote.high_price.unwrap_or(quote.last_price),
                    low_price: quote.low_price.unwrap_or(quote.last_price),
                    close_price: quote.close_price.unwrap_or(quote.last_price),
                    volume: quote.volume,
                    pe_ratio: quote.pe_ratio,
                    market_cap: quote.market_cap,
                    dividend_yield: quote.dividend_yield,
                };

                self.database.insert_daily_price(&daily_price)?;
                updated_count += 1;
                debug!("Updated price for {}: ${:.2}", quote.symbol, daily_price.close_price);
            } else {
                warn!("Received quote for unknown stock: {}", quote.symbol);
            }
        }

        Ok(updated_count)
    }

    /// Perform historical data backfill from a start date
    pub async fn backfill_historical_data(&self, from_date: NaiveDate, to_date: Option<NaiveDate>) -> Result<usize> {
        let end_date = to_date.unwrap_or_else(|| Utc::now().date_naive());
        
        info!("📈 Starting historical data backfill from {} to {}", from_date, end_date);
        
        let stocks = self.database.get_active_stocks()?;
        let total_stocks = stocks.len();
        
        info!("Processing {} stocks concurrently with max 10 parallel requests...", total_stocks);
        
        // Track progress
        let mut total_records = 0;
        let mut success_count = 0;
        let mut error_count = 0;
        let mut processed = 0;
        
        // Create concurrent stream that yields results as they complete
        let mut results = stream::iter(stocks)
            .enumerate()
            .map(|(index, stock)| {
                let client = self.schwab_client.clone();
                let database = self.database.clone();
                let semaphore = self.concurrency_semaphore.clone();
                let symbol = stock.symbol.clone();
                
                async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    let result = Self::fetch_stock_history(client, database, stock, from_date, end_date).await;
                    (index, symbol, result)
                }
            })
            .buffer_unordered(10); // Process up to 10 stocks concurrently
        
        // Process results as they come in
        while let Some((_index, symbol, result)) = results.next().await {
            processed += 1;
            
            match result {
                Ok(records) => {
                    total_records += records;
                    success_count += 1;
                    if records > 0 {
                        info!("✅ {}/{}: {} - {} records added", processed, total_stocks, symbol, records);
                    } else {
                        debug!("⚪ {}/{}: {} - no new records", processed, total_stocks, symbol);
                    }
                }
                Err(e) => {
                    error_count += 1;
                    error!("❌ {}/{}: {} failed - {}", processed, total_stocks, symbol, e);
                }
            }
            
            // Progress update every 25 stocks
            if processed % 25 == 0 {
                info!("📊 Progress: {}/{} stocks processed, {} records collected, {} errors", 
                      processed, total_stocks, total_records, error_count);
            }
        }

        info!("✅ Historical backfill completed: {} records from {} stocks ({} errors)", 
              total_records, success_count, error_count);
        
        // Update last sync date
        self.database.set_last_update_date(end_date)?;
        
        Ok(total_records)
    }

    /// Fetch historical data for a single stock
    pub async fn fetch_stock_history(
        client: Arc<SchwabClient>,
        database: Arc<DatabaseManager>,
        stock: Stock,
        from_date: NaiveDate,
        to_date: NaiveDate,
    ) -> Result<usize> {
        let stock_id = stock.id.ok_or_else(|| anyhow::anyhow!("Stock has no ID: {}", stock.symbol))?;
        
        debug!("Fetching history for {}: {} to {}", stock.symbol, from_date, to_date);
        
        // Get price history from API
        let price_bars = client.get_price_history(&stock.symbol, from_date, to_date).await?;
        
        let mut inserted_count = 0;
        
        for bar in price_bars {
            // Convert timestamp to date
            let date = chrono::DateTime::from_timestamp(bar.datetime / 1000, 0)
                .ok_or_else(|| anyhow::anyhow!("Invalid timestamp: {}", bar.datetime))?
                .date_naive();
            
            // Skip if we already have data for this date
            if database.get_price_on_date(stock_id, date)?.is_some() {
                continue;
            }
            
            let daily_price = DailyPrice {
                id: None,
                stock_id,
                date,
                open_price: bar.open,
                high_price: bar.high,
                low_price: bar.low,
                close_price: bar.close,
                volume: Some(bar.volume),
                pe_ratio: None, // Historical P/E not available in price history
                market_cap: None,
                dividend_yield: None,
            };

            database.insert_daily_price(&daily_price)?;
            inserted_count += 1;
        }

        if inserted_count > 0 {
            info!("✅ {}: Added {} historical records", stock.symbol, inserted_count);
        }
        
        Ok(inserted_count)
    }

    /// Perform incremental update (fetch data since last update)
    pub async fn incremental_update(&self) -> Result<usize> {
        info!("🔄 Starting incremental data update...");
        
        let last_update = self.database.get_last_update_date()?;
        let today = Utc::now().date_naive();
        
        let from_date = match last_update {
            Some(date) => {
                if date >= today {
                    info!("✅ Data is already up to date (last update: {})", date);
                    return Ok(0);
                }
                date + Duration::days(1) // Start from day after last update
            }
            None => {
                info!("No previous updates found, fetching current quotes only");
                return self.fetch_current_quotes().await;
            }
        };

        info!("Updating data from {} to {}", from_date, today);
        
        // For incremental updates, we focus on recent data
        let records = self.backfill_historical_data(from_date, Some(today)).await?;
        
        info!("✅ Incremental update completed: {} new records", records);
        Ok(records)
    }

    /// Get collection statistics
    pub async fn get_collection_stats(&self) -> Result<CollectionStats> {
        let (stock_count, price_count, last_update) = self.database.get_stats()?;
        
        // Calculate date range of available data
        let (earliest_date, latest_date) = self.get_data_date_range().await?;
        
        Ok(CollectionStats {
            total_stocks: stock_count,
            total_price_records: price_count,
            last_update_date: last_update,
            earliest_data_date: earliest_date,
            latest_data_date: latest_date,
        })
    }

    /// Get the date range of available price data
    async fn get_data_date_range(&self) -> Result<(Option<NaiveDate>, Option<NaiveDate>)> {
        // This would require additional database queries - simplified for now
        Ok((None, None))
    }

    /// Validate data integrity
    pub async fn validate_data_integrity(&self) -> Result<ValidationReport> {
        info!("🔍 Validating data integrity...");
        
        let stocks = self.database.get_active_stocks()?;
        let mut validation_report = ValidationReport::new();
        
        for stock in stocks {
            if let Some(stock_id) = stock.id {
                // Check for data gaps
                let latest_price = self.database.get_latest_price(stock_id)?;
                
                if let Some(price) = latest_price {
                    validation_report.stocks_with_data += 1;
                    validation_report.latest_data_date = 
                        validation_report.latest_data_date.max(Some(price.date));
                } else {
                    validation_report.stocks_without_data += 1;
                    validation_report.issues.push(format!("No price data for {}", stock.symbol));
                }
            }
        }
        
        info!("✅ Validation complete: {} stocks with data, {} without data", 
              validation_report.stocks_with_data, validation_report.stocks_without_data);
        
        Ok(validation_report)
    }

    /// Fetch data for a single stock in batches with progress updates
    pub async fn fetch_single_stock_batched(
        &self,
        symbol: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
        batch_size: usize,
        progress_callback: Option<Box<dyn Fn(String) + Send>>,
    ) -> Result<usize> {
        let total_days = (end_date - start_date).num_days() as usize;
        let total_batches = (total_days + batch_size - 1) / batch_size;
        let mut total_bars = 0;
        let mut current_date = start_date;

        if let Some(ref callback) = progress_callback {
            callback(format!("Starting batched collection for {} ({} days, {} batches)", 
                           symbol, total_days, total_batches));
        }

        for batch_num in 1..=total_batches {
            if let Some(ref callback) = progress_callback {
                callback(format!("Batch {}/{}: Fetching {} to {}", 
                               batch_num, total_batches, current_date, end_date));
            }

            // Calculate batch end date
            let batch_end_date = if batch_num == total_batches {
                end_date
            } else {
                current_date + chrono::Duration::days(batch_size as i64 - 1)
            };

            // Fetch data for this batch
            match self.schwab_client.get_price_history(symbol, current_date, batch_end_date).await {
                Ok(bars) => {
                    total_bars += bars.len();
                    if let Some(ref callback) = progress_callback {
                        callback(format!("✅ Batch {}: Fetched {} bars (Total: {})", 
                                       batch_num, bars.len(), total_bars));
                    }
                }
                Err(e) => {
                    if let Some(ref callback) = progress_callback {
                        callback(format!("❌ Batch {}: Error - {}", batch_num, e));
                    }
                    // Continue with next batch instead of failing completely
                }
            }

            // Move to next batch start date
            current_date = batch_end_date + chrono::Duration::days(1);
            
            // Add a small delay between batches to avoid rate limiting
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        if let Some(ref callback) = progress_callback {
            callback(format!("🎉 Collection complete! Total bars: {}", total_bars));
        }

        Ok(total_bars)
    }
}

/// Statistics about data collection
#[derive(Debug)]
pub struct CollectionStats {
    pub total_stocks: usize,
    pub total_price_records: usize,
    pub last_update_date: Option<NaiveDate>,
    pub earliest_data_date: Option<NaiveDate>,
    pub latest_data_date: Option<NaiveDate>,
}

/// Data validation report
#[derive(Debug)]
pub struct ValidationReport {
    pub stocks_with_data: usize,
    pub stocks_without_data: usize,
    pub latest_data_date: Option<NaiveDate>,
    pub issues: Vec<String>,
}

impl ValidationReport {
    fn new() -> Self {
        Self {
            stocks_with_data: 0,
            stocks_without_data: 0,
            latest_data_date: None,
            issues: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_collection_stats() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let database = DatabaseManager::new(db_path.to_str().unwrap()).unwrap();
        
        let config = Config {
            schwab_api_key: "test".to_string(),
            schwab_app_secret: "test".to_string(),
            schwab_callback_url: "test".to_string(),
            schwab_token_path: "test".to_string(),
            database_path: "test".to_string(),
            rate_limit_per_minute: 120,
            batch_size: 50,
        };
        
        let client = SchwabClient::new(&config).unwrap();
        let collector = DataCollector::new(client, database, config);
        
        let stats = collector.get_collection_stats().await.unwrap();
        assert_eq!(stats.total_stocks, 0);
        assert_eq!(stats.total_price_records, 0);
    }
}