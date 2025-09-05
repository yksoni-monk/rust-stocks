//! Concurrent stock data fetching module
//! 
//! This module provides functionality to fetch stock data from multiple stocks
//! concurrently using a configurable number of worker threads.

use anyhow::Result;
use chrono::NaiveDate;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tracing::{info, warn, error};

use crate::{
    api::schwab_client::SchwabClient,
    database_sqlx::DatabaseManagerSqlx,
    models::{Stock, Config},
    data_collector::DataCollector,
};

/// Unified configuration for stock fetching (single stock or concurrent)
#[derive(Debug, Clone)]
pub struct UnifiedFetchConfig {
    pub stocks: Vec<Stock>,           // Single=[selected], All=get_active_stocks()
    pub date_range: DateRange,
    pub num_threads: usize,           // Single=1, Concurrent=5-10
    pub retry_attempts: u32,
    pub rate_limit_ms: u64,
    pub max_stocks: Option<usize>,    // For testing limits
}

/// Legacy configuration for backward compatibility
#[derive(Debug, Clone)]
pub struct ConcurrentFetchConfig {
    pub date_range: DateRange,
    pub num_threads: usize,
    pub retry_attempts: u32,
    pub max_stocks: Option<usize>, // Optional limit for testing
}

/// Date range for fetching data
#[derive(Debug, Clone)]
pub struct DateRange {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

/// Progress update from worker threads
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct FetchProgress {
    pub thread_id: usize,
    pub stock_symbol: String,
    pub status: FetchStatus,
    pub message: String,
}

/// Status of a fetch operation
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum FetchStatus {
    Started,
    Skipped, // Data already exists
    Completed,
    Failed(String),
}

/// Result of concurrent fetching operation
#[derive(Debug)]
pub struct FetchResult {
    pub total_stocks: usize,
    pub processed_stocks: usize,
    pub skipped_stocks: usize,
    pub failed_stocks: usize,
    pub total_records_fetched: usize,
}

/// Unified function to fetch stock data (single stock or concurrent)
pub async fn fetch_stocks_unified_with_logging(
    database: Arc<DatabaseManagerSqlx>,
    config: UnifiedFetchConfig,
    global_broadcast_sender: Option<Arc<broadcast::Sender<crate::ui::state::StateUpdate>>>,
) -> Result<FetchResult> {
    info!("🚀 Starting unified fetch with {} threads", config.num_threads);
    info!("📅 Date range: {} to {}", config.date_range.start_date, config.date_range.end_date);

    // Use provided stocks instead of loading from database
    let stocks = config.stocks.clone();
    let total_stocks = stocks.len();
    info!("📊 Processing {} stocks", total_stocks);

    // Send log to TUI if available
    if let Some(sender) = &global_broadcast_sender {
        let _ = sender.send(crate::ui::state::StateUpdate::LogMessage {
            level: crate::ui::state::LogLevel::Info,
            message: format!("📊 Processing {} stocks", total_stocks),
        });
    }

    // Apply stock limit if specified (for testing)
    let stocks = if let Some(max_stocks) = config.max_stocks {
        let limited_stocks = stocks.into_iter().take(max_stocks).collect::<Vec<_>>();
        info!("🔢 Limiting to {} stocks for testing", limited_stocks.len());
        limited_stocks
    } else {
        stocks
    };
    let actual_total = stocks.len();

    // Create thread-safe stock queue
    let stock_queue = Arc::new(Mutex::new(stocks));
    
    // Create progress tracking channel
    let (progress_sender, _progress_receiver) = broadcast::channel(100);
    let progress_sender = Arc::new(progress_sender);

    // Create shared counters for result tracking
    let counters = Arc::new(Mutex::new(FetchCounters::new()));
    
    // Set the total number of stocks
    {
        let mut counters = counters.lock().unwrap();
        counters.total_stocks = actual_total;
    }

    // Spawn worker threads
    let mut handles = Vec::new();
    
    for thread_id in 0..config.num_threads {
        let stock_queue = Arc::clone(&stock_queue);
        let database = Arc::clone(&database);
        let progress_sender = Arc::clone(&progress_sender);
        let counters = Arc::clone(&counters);
        let config = config.clone();
        let global_broadcast_sender = global_broadcast_sender.clone();

        let handle = tokio::spawn(async move {
            worker_thread_with_logging(
                thread_id,
                stock_queue,
                database,
                progress_sender,
                counters,
                config,
                global_broadcast_sender,
            ).await
        });
        
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.await??;
    }

    // Get final results
    let final_counters = counters.lock().unwrap();
    let result = FetchResult {
        total_stocks: final_counters.total_stocks,
        processed_stocks: final_counters.processed_stocks,
        skipped_stocks: final_counters.skipped_stocks,
        failed_stocks: final_counters.failed_stocks,
        total_records_fetched: final_counters.total_records_fetched,
    };

    info!("✅ Unified fetch completed");
    info!("📊 Results: {} processed, {} skipped, {} failed, {} records fetched", 
          result.processed_stocks, result.skipped_stocks, result.failed_stocks, result.total_records_fetched);

    Ok(result)
}

/// Main function to fetch stock data concurrently
pub async fn fetch_stocks_concurrently(
    database: Arc<DatabaseManagerSqlx>,
    config: ConcurrentFetchConfig,
) -> Result<FetchResult> {
    // Convert legacy config to unified config
    let stocks = database.get_active_stocks().await?;
    let unified_config = UnifiedFetchConfig {
        stocks,
        date_range: config.date_range,
        num_threads: config.num_threads,
        retry_attempts: config.retry_attempts,
        rate_limit_ms: 500, // Default rate limit
        max_stocks: config.max_stocks,
    };
    fetch_stocks_unified_with_logging(database, unified_config, None).await
}

/// Legacy function to fetch stock data concurrently with TUI logging
/// Delegates to the unified function for consistency
pub async fn fetch_stocks_concurrently_with_logging(
    database: Arc<DatabaseManagerSqlx>,
    config: ConcurrentFetchConfig,
    global_broadcast_sender: Option<Arc<broadcast::Sender<crate::ui::state::StateUpdate>>>,
) -> Result<FetchResult> {
    // Convert legacy config to unified config
    let stocks = database.get_active_stocks().await?;
    let unified_config = UnifiedFetchConfig {
        stocks,
        date_range: config.date_range,
        num_threads: config.num_threads,
        retry_attempts: config.retry_attempts,
        rate_limit_ms: 500, // Default rate limit
        max_stocks: config.max_stocks,
    };
    fetch_stocks_unified_with_logging(database, unified_config, global_broadcast_sender).await
}

/// Worker thread function with TUI logging
async fn worker_thread_with_logging(
    thread_id: usize,
    stock_queue: Arc<Mutex<Vec<Stock>>>,
    database: Arc<DatabaseManagerSqlx>,
    progress_sender: Arc<broadcast::Sender<FetchProgress>>,
    counters: Arc<Mutex<FetchCounters>>,
    config: UnifiedFetchConfig,
    global_broadcast_sender: Option<Arc<broadcast::Sender<crate::ui::state::StateUpdate>>>,
) -> Result<()> {
    // Create API client for this thread
    let api_config = Config::from_env()?;
    let api_client = SchwabClient::new(&api_config)?;
    
    loop {
        // Get next stock from queue
        let stock = {
            let mut queue = stock_queue.lock().unwrap();
            if queue.is_empty() {
                break; // No more stocks to process
            }
            queue.remove(0)
        };

        let stock_symbol = stock.symbol.clone();
        let _stock_id = stock.id.unwrap();

        // Send progress update to TUI
        if let Some(sender) = &global_broadcast_sender {
            let _ = sender.send(crate::ui::state::StateUpdate::LogMessage {
                level: crate::ui::state::LogLevel::Info,
                message: format!("🔄 Thread {}: Starting {}", thread_id, stock_symbol),
            });
        }

        // Send progress update to internal channel
        let _ = progress_sender.send(FetchProgress {
            thread_id,
            stock_symbol: stock_symbol.clone(),
            status: FetchStatus::Started,
            message: format!("Thread {}: Starting {}", thread_id, stock_symbol),
        });

        // Fetch data for this stock (let the data collector handle existing records)
        match fetch_stock_data(&api_client, &database, &stock, &config, global_broadcast_sender.clone()).await {
            Ok(records_fetched) => {
                let success_message = format!("✅ Thread {}: Completed {} ({} records fetched)", 
                                           thread_id, stock_symbol, records_fetched);
                
                if let Some(sender) = &global_broadcast_sender {
                    let _ = sender.send(crate::ui::state::StateUpdate::LogMessage {
                        level: crate::ui::state::LogLevel::Success,
                        message: success_message.clone(),
                    });
                }

                let _ = progress_sender.send(FetchProgress {
                    thread_id,
                    stock_symbol: stock_symbol.clone(),
                    status: FetchStatus::Completed,
                    message: success_message,
                });

                let mut counters = counters.lock().unwrap();
                counters.processed_stocks += 1;
                counters.total_records_fetched += records_fetched;
            }
            Err(e) => {
                let error_msg = format!("❌ Thread {}: Failed {} - {}", thread_id, stock_symbol, e);
                error!("{}", error_msg);
                
                if let Some(sender) = &global_broadcast_sender {
                    let _ = sender.send(crate::ui::state::StateUpdate::LogMessage {
                        level: crate::ui::state::LogLevel::Error,
                        message: error_msg.clone(),
                    });
                }

                let _ = progress_sender.send(FetchProgress {
                    thread_id,
                    stock_symbol: stock_symbol.clone(),
                    status: FetchStatus::Failed(e.to_string()),
                    message: error_msg,
                });

                let mut counters = counters.lock().unwrap();
                counters.failed_stocks += 1;
            }
        }
    }

    Ok(())
}
#[allow(dead_code)]
/// Fetch data for a single stock with retry logic using existing batching function
async fn fetch_stock_data(
    api_client: &SchwabClient,
    database: &DatabaseManagerSqlx,
    stock: &Stock,
    config: &UnifiedFetchConfig,
    global_broadcast_sender: Option<Arc<broadcast::Sender<crate::ui::state::StateUpdate>>>,
) -> Result<usize> {
    let mut attempts = 0;
    let mut last_error = None;

    while attempts < config.retry_attempts {
        match DataCollector::fetch_stock_history_with_batching_ref(
            api_client,
            database,
            stock.clone(),
            config.date_range.start_date,
            config.date_range.end_date,
            global_broadcast_sender.clone(),
        ).await {
            Ok(records_inserted) => {
                return Ok(records_inserted);
            }
            Err(e) => {
                attempts += 1;
                let error_msg = e.to_string();
                last_error = Some(e);
                
                if attempts < config.retry_attempts {
                    warn!("Attempt {} failed for {}: {}. Retrying...", 
                          attempts, stock.symbol, error_msg);
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
            }
        }
    }

    // All attempts failed
    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Unknown error")))
}

/// Internal counters for tracking progress
#[allow(dead_code)]
#[derive(Debug)]
struct FetchCounters {
    total_stocks: usize,
    processed_stocks: usize,
    skipped_stocks: usize,
    failed_stocks: usize,
    total_records_fetched: usize,
}

impl FetchCounters {
    fn new() -> Self {
        Self {
            total_stocks: 0,
            processed_stocks: 0,
            skipped_stocks: 0,
            failed_stocks: 0,
            total_records_fetched: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[tokio::test]
    async fn test_concurrent_fetch_config() {
        let config = ConcurrentFetchConfig {
            date_range: DateRange {
                start_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
                end_date: NaiveDate::from_ymd_opt(2025, 8, 31).unwrap(),
            },
            num_threads: 10,
            retry_attempts: 3,
            max_stocks: Some(5),
        };

        assert_eq!(config.num_threads, 10);
        assert_eq!(config.retry_attempts, 3);
        assert_eq!(config.max_stocks, Some(5));
        assert_eq!(config.date_range.start_date, NaiveDate::from_ymd_opt(2020, 1, 1).unwrap());
        assert_eq!(config.date_range.end_date, NaiveDate::from_ymd_opt(2025, 8, 31).unwrap());
    }

    #[test]
    fn test_fetch_progress() {
        let progress = FetchProgress {
            thread_id: 1,
            stock_symbol: "AAPL".to_string(),
            status: FetchStatus::Started,
            message: "Thread 1: Starting AAPL".to_string(),
        };

        assert_eq!(progress.thread_id, 1);
        assert_eq!(progress.stock_symbol, "AAPL");
        assert!(matches!(progress.status, FetchStatus::Started));
    }
}
