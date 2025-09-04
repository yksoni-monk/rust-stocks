use anyhow::Result;
use chrono::{Duration, NaiveDate, Utc};
use futures::stream::{self, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{info, warn, error, debug};

use crate::api::{SchwabClient, StockDataProvider};
use crate::database_sqlx::DatabaseManagerSqlx;
use crate::models::{Config, Stock, DailyPrice, SchwabQuote};
use crate::utils::TradingWeekBatchCalculator;

/// Data collection system for fetching and storing stock data
pub struct DataCollector {
    #[allow(dead_code)]
    schwab_client: Arc<SchwabClient>,
    #[allow(dead_code)]
    database: Arc<DatabaseManagerSqlx>,
    #[allow(dead_code)]
    config: Config,
    #[allow(dead_code)]
    concurrency_semaphore: Arc<Semaphore>,
}

impl DataCollector {
    /// Create a new data collector
    #[allow(dead_code)]
    pub fn new(schwab_client: SchwabClient, database: DatabaseManagerSqlx, config: Config) -> Self {
        let max_concurrent = std::cmp::min(config.batch_size, 10); // Limit concurrent requests
        
        Self {
            schwab_client: Arc::new(schwab_client),
            database: Arc::new(database),
            config,
            concurrency_semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    /// Get all active S&P 500 stocks from database
    #[allow(dead_code)]
    pub async fn get_active_stocks(&self) -> Result<Vec<Stock>> {
        info!("📊 Getting active stocks from database...");
        let stocks = self.database.get_active_stocks().await?;
        info!("✅ Found {} active stocks in database", stocks.len());
        Ok(stocks)
    }

    /// Fetch current quotes for all active stocks
    #[allow(dead_code)]
    pub async fn fetch_current_quotes(&self) -> Result<usize> {
        info!("📊 Fetching current quotes for all active stocks...");
        
        let stocks = self.database.get_active_stocks().await?;
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
    #[allow(dead_code)]
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
                if self.database.get_price_on_date(stock_id, today).await?.is_some() {
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

                self.database.insert_daily_price(&daily_price).await?;
                updated_count += 1;
                debug!("Updated price for {}: ${:.2}", quote.symbol, daily_price.close_price);
            } else {
                warn!("Received quote for unknown stock: {}", quote.symbol);
            }
        }

        Ok(updated_count)
    }

    /// Perform historical data backfill from a start date
    #[allow(dead_code)]
    pub async fn backfill_historical_data(&self, from_date: NaiveDate, to_date: Option<NaiveDate>) -> Result<usize> {
        let end_date = to_date.unwrap_or_else(|| Utc::now().date_naive());
        
        info!("📈 Starting historical data backfill from {} to {}", from_date, end_date);
        
        let stocks = self.database.get_active_stocks().await?;
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
                    let result = Self::fetch_stock_history_with_batching_ref(&client, &database, stock, from_date, end_date, None).await;
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
        self.database.set_metadata("last_update_date", &end_date.to_string()).await?;
        
        Ok(total_records)
    }


    /// Fetch historical data for a single stock using weekly batches (takes reference)
    #[allow(dead_code)]
    pub async fn fetch_stock_history_with_batching_ref(
        client: &SchwabClient,
        database: &DatabaseManagerSqlx,
        stock: Stock,
        from_date: NaiveDate,
        to_date: NaiveDate,
        global_broadcast_sender: Option<Arc<tokio::sync::broadcast::Sender<crate::ui::state::StateUpdate>>>,
    ) -> Result<usize> {
        let stock_id = stock.id.ok_or_else(|| anyhow::anyhow!("Stock has no ID: {}", stock.symbol))?;
        
        debug!("Fetching history for {}: {} to {} using weekly batches", stock.symbol, from_date, to_date);
        
        // Calculate trading week batches
        let batches = TradingWeekBatchCalculator::calculate_batches(from_date, to_date);
        
        // Log batching plan for user visibility
        let total_days = (to_date - from_date).num_days() + 1;
        let total_weeks = batches.len();
        let plan_message = format!("📅 {} data plan: {} days ({} weeks) from {} to {}", 
                                   stock.symbol, total_days, total_weeks, from_date, to_date);
        info!("{}", plan_message);
        if let Some(sender) = &global_broadcast_sender {
            let _ = sender.send(crate::ui::state::StateUpdate::LogMessage {
                level: crate::ui::state::LogLevel::Info,
                message: plan_message,
            });
        }
        
        if !batches.is_empty() {
            let batch_message = format!("📊 {} batch plan: {} trading week batches to process", stock.symbol, batches.len());
            info!("{}", batch_message);
            if let Some(sender) = &global_broadcast_sender {
                let _ = sender.send(crate::ui::state::StateUpdate::LogMessage {
                    level: crate::ui::state::LogLevel::Info,
                    message: batch_message,
                });
            }
        }
        
        let mut total_inserted = 0;

        // Process each trading week batch
        for (i, batch) in batches.iter().enumerate() {
            let batch_start_message = format!("🔄 {} batch {}/{}: {} to {}", 
                                               stock.symbol, i + 1, batches.len(), batch.start_date, batch.end_date);
            info!("{}", batch_start_message);
            if let Some(sender) = &global_broadcast_sender {
                let _ = sender.send(crate::ui::state::StateUpdate::LogMessage {
                    level: crate::ui::state::LogLevel::Info,
                    message: batch_start_message,
                });
            }
            
            // Check existing records for this batch
            let existing_count = database.count_existing_records(stock_id, batch.start_date, batch.end_date).await?;
            
            if existing_count > 0 {
                let skip_message = format!("⏭️  {} batch {}/{}: {} existing records found, skipping", 
                                           stock.symbol, i + 1, batches.len(), existing_count);
                info!("{}", skip_message);
                if let Some(sender) = &global_broadcast_sender {
                    let _ = sender.send(crate::ui::state::StateUpdate::LogMessage {
                        level: crate::ui::state::LogLevel::Info,
                        message: skip_message,
                    });
                }
                continue;
            }

            // Fetch data for this batch using the existing function
            match client.get_price_history(&stock.symbol, batch.start_date, batch.end_date).await {
                Ok(price_bars) => {
                    let mut records_inserted = 0;
                    for bar in price_bars {
                        // Convert timestamp to date
                        let date = chrono::DateTime::from_timestamp(bar.datetime / 1000, 0)
                            .ok_or_else(|| anyhow::anyhow!("Invalid timestamp: {}", bar.datetime))?
                            .date_naive();
                        
                        // Skip if we already have data for this date
                        if database.get_price_on_date(stock_id, date).await?.is_some() {
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
                        
                        database.insert_daily_price(&daily_price).await?;
                        records_inserted += 1;
                    }
                    
                    total_inserted += records_inserted;
                    let batch_result_message = if records_inserted > 0 {
                        format!("✅ {} batch {}/{}: inserted {} records (total: {})", 
                                stock.symbol, i + 1, batches.len(), records_inserted, total_inserted)
                    } else {
                        format!("ℹ️  {} batch {}/{}: no new records needed (total: {})", 
                                stock.symbol, i + 1, batches.len(), total_inserted)
                    };
                    
                    info!("{}", batch_result_message);
                    if let Some(sender) = &global_broadcast_sender {
                        let level = if records_inserted > 0 {
                            crate::ui::state::LogLevel::Success
                        } else {
                            crate::ui::state::LogLevel::Info
                        };
                        let _ = sender.send(crate::ui::state::StateUpdate::LogMessage {
                            level,
                            message: batch_result_message,
                        });
                    }
                }
                Err(e) => {
                    let error_message = format!("❌ {} batch {}/{}: failed - {}", stock.symbol, i + 1, batches.len(), e);
                    warn!("{}", error_message);
                    if let Some(sender) = &global_broadcast_sender {
                        let _ = sender.send(crate::ui::state::StateUpdate::LogMessage {
                            level: crate::ui::state::LogLevel::Error,
                            message: error_message,
                        });
                    }
                }
            }

            // Small delay between batches to avoid rate limiting
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        // Final completion summary
        let completion_message = if total_inserted > 0 {
            format!("🏁 {} complete: {} new records added from {} batches", 
                    stock.symbol, total_inserted, batches.len())
        } else {
            format!("🏁 {} complete: all data already exists, no new records needed", stock.symbol)
        };
        
        info!("{}", completion_message);
        if let Some(sender) = &global_broadcast_sender {
            let _ = sender.send(crate::ui::state::StateUpdate::LogMessage {
                level: crate::ui::state::LogLevel::Success,
                message: completion_message,
            });
        }
        
        Ok(total_inserted)
    }


    /// Perform incremental update (fetch data since last update)
    #[allow(dead_code)]
    pub async fn incremental_update(&self) -> Result<usize> {
        info!("🔄 Starting incremental data update...");
        
        let last_update = self.database.get_metadata("last_update_date").await?;
        let today = Utc::now().date_naive();
        
        let from_date = match last_update {
            Some(date_str) => {
                let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")?;
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
    #[allow(dead_code)]
        pub async fn get_collection_stats(&self) -> Result<DataCollectionStats> {
        let stats = self.database.get_stats().await?;
        
        // Calculate date range of available data
        let (earliest_date, latest_date) = self.get_data_date_range().await?;
        
        Ok(DataCollectionStats {
            total_stocks: stats.get("total_stocks").unwrap_or(&0).clone() as usize,
            total_price_records: stats.get("total_prices").unwrap_or(&0).clone() as usize,
            last_update_date: None, // TODO: Add this to database stats
            earliest_data_date: earliest_date,
            latest_data_date: latest_date,
        })
    }

    /// Get the date range of available price data
    #[allow(dead_code)]
    async fn get_data_date_range(&self) -> Result<(Option<NaiveDate>, Option<NaiveDate>)> {
        // This would require additional database queries - simplified for now
        Ok((None, None))
    }

    /// Validate data integrity
    #[allow(dead_code)]
    pub async fn validate_data_integrity(&self) -> Result<ValidationReport> {
        info!("🔍 Validating data integrity...");
        
        let stocks = self.database.get_active_stocks().await?;
        let mut validation_report = ValidationReport::new();
        
        for stock in stocks {
            if let Some(stock_id) = stock.id {
                // Check for data gaps
                let latest_price = self.database.get_latest_price(stock_id).await?;
                
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
    #[allow(dead_code)]
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
            // Calculate batch end date
            let batch_end_date = if batch_num == total_batches {
                end_date
            } else {
                current_date + chrono::Duration::days(batch_size as i64 - 1)
            };

            if let Some(ref callback) = progress_callback {
                callback(format!("Batch {}/{}: Fetching {} to {}", 
                               batch_num, total_batches, current_date, batch_end_date));
            }

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
#[allow(dead_code)]
#[derive(Debug)]
pub struct DataCollectionStats {
    pub total_stocks: usize,
    pub total_price_records: usize,
    pub last_update_date: Option<NaiveDate>,
    pub earliest_data_date: Option<NaiveDate>,
    pub latest_data_date: Option<NaiveDate>,
}

/// Data validation report
#[derive(Debug)]
#[allow(dead_code)]
pub struct ValidationReport {
    pub stocks_with_data: usize,
    pub stocks_without_data: usize,
    pub latest_data_date: Option<NaiveDate>,
    pub issues: Vec<String>,
}

impl ValidationReport {
    #[allow(dead_code)]
    fn new() -> Self {
        Self {
            stocks_with_data: 0,
            stocks_without_data: 0,
            latest_data_date: None,
            issues: Vec::new(),
        }
    }
}

