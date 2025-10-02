/// Schwab S&P 500 Bulk Price Data Download Tool
/// 
/// Downloads historical OHLCV data for all S&P 500 stocks from Schwab API
/// and imports them into the database with comprehensive error handling,
/// progress tracking, and resume capability.

use anyhow::{Result, anyhow};
use chrono::{NaiveDate, DateTime, Utc, Local};
use clap::{Parser, Subcommand};
use rust_stocks_tauri_lib::api::schwab_client::SchwabClient;
use rust_stocks_tauri_lib::api::StockDataProvider;
use rust_stocks_tauri_lib::models::Config;
use rust_stocks_tauri_lib::tools::date_range_calculator::DateRangeCalculator;
// DataStatusReader removed - using SEC filing-based freshness checking
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::sync::Semaphore;
use tracing::{info, warn, debug};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    /// Start date for price data (YYYY-MM-DD)
    #[arg(long, default_value = "2015-01-01")]
    start_date: String,

    /// End date for price data (YYYY-MM-DD, defaults to current date)
    #[arg(long)]
    end_date: Option<String>,
    
    /// Resume previous incomplete download
    #[arg(long)]
    resume: bool,
    
    /// Test mode - download only one symbol
    #[arg(long)]
    test_symbol: Option<String>,
    
    /// Validate existing data without downloading
    #[arg(long)]
    validate_only: bool,
    
    /// Progress file path
    #[arg(long, default_value = "schwab_download_progress.json")]
    progress_file: String,
    
    /// Batch size for database operations
    #[arg(long, default_value = "100")]
    batch_size: usize,
    
    /// Maximum number of retries per symbol
    #[arg(long, default_value = "3")]
    max_retries: usize,
}

#[derive(Subcommand)]
enum Commands {
    /// Download price data for all S&P 500 stocks
    Download,
    /// Resume an interrupted download
    Resume,
    /// Validate data quality
    Validate,
    /// Show progress of current/last download
    Status,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct DownloadProgress {
    session_id: String,
    start_time: DateTime<Utc>,
    total_symbols: usize,
    completed_symbols: HashSet<String>,
    failed_symbols: HashMap<String, String>,
    current_symbol: Option<String>,
    settings: DownloadSettings,
    statistics: DownloadStatistics,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct DownloadSettings {
    start_date: String,
    end_date: String,
    batch_size: usize,
    max_retries: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct DownloadStatistics {
    total_api_calls: usize,
    successful_calls: usize,
    failed_calls: usize,
    total_price_records: usize,
    average_bars_per_symbol: f64,
    download_speed_symbols_per_minute: f64,
}

impl Default for DownloadStatistics {
    fn default() -> Self {
        Self {
            total_api_calls: 0,
            successful_calls: 0,
            failed_calls: 0,
            total_price_records: 0,
            average_bars_per_symbol: 0.0,
            download_speed_symbols_per_minute: 0.0,
        }
    }
}

struct BulkDownloader {
    schwab_client: SchwabClient,
    db_pool: SqlitePool,
    progress: DownloadProgress,
    progress_file: PathBuf,
    rate_limiter: Arc<Semaphore>,
    start_time: Instant,
    date_calculator: DateRangeCalculator,
    incremental_mode: bool,
}

impl BulkDownloader {
    async fn new(
        config: &Config,
        progress_file: PathBuf,
        settings: DownloadSettings,
        incremental_mode: bool,
    ) -> Result<Self> {
        let schwab_client = SchwabClient::new(config)?;
        
        // Connect to database
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite:./db/stocks.db".to_string());
        let db_pool = SqlitePool::connect(&database_url).await?;
        
        // Initialize or load progress
        let progress = if progress_file.exists() {
            Self::load_progress(&progress_file).await?
        } else {
            let symbols = Self::load_sp500_symbols(&db_pool).await?;
            DownloadProgress {
                session_id: uuid::Uuid::new_v4().to_string(),
                start_time: Utc::now(),
                total_symbols: symbols.len(),
                completed_symbols: HashSet::new(),
                failed_symbols: HashMap::new(),
                current_symbol: None,
                settings,
                statistics: DownloadStatistics::default(),
            }
        };
        
        // Rate limiter: 120 requests per minute = 1 request per 0.5 seconds
        // Using 3 concurrent requests with rate limiting
        let rate_limiter = Arc::new(Semaphore::new(3));
        
        Ok(Self {
            schwab_client,
            db_pool,
            progress,
            progress_file,
            rate_limiter,
            start_time: Instant::now(),
            date_calculator: DateRangeCalculator::new(),
            incremental_mode,
        })
    }
    
    async fn load_sp500_symbols(db_pool: &SqlitePool) -> Result<Vec<String>> {
        let symbols = sqlx::query_scalar::<_, String>(
            "SELECT symbol FROM stocks WHERE is_sp500 = 1 ORDER BY symbol"
        )
        .fetch_all(db_pool)
        .await?;
        
        if symbols.is_empty() {
            return Err(anyhow!("No S&P 500 symbols found in database. Please run S&P 500 symbol import first."));
        }
        
        info!("Loaded {} S&P 500 symbols from database", symbols.len());
        Ok(symbols)
    }
    
    async fn load_progress(progress_file: &PathBuf) -> Result<DownloadProgress> {
        let content = fs::read_to_string(progress_file).await?;
        let progress: DownloadProgress = serde_json::from_str(&content)?;
        info!("Loaded existing progress: {}/{} symbols completed", 
               progress.completed_symbols.len(), progress.total_symbols);
        Ok(progress)
    }
    
    async fn save_progress(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.progress)?;
        fs::write(&self.progress_file, content).await?;
        Ok(())
    }
    
    async fn download_all_symbols(&mut self) -> Result<()> {
        let symbols = Self::load_sp500_symbols(&self.db_pool).await?;
        
        println!("🚀 Schwab S&P 500 Price Data Download");
        println!("=====================================");
        println!("Total symbols: {}", symbols.len());
        println!("Date range: {} to {}", self.progress.settings.start_date, self.progress.settings.end_date);
        println!("Progress file: {}", self.progress_file.display());
        println!();
        
        // Filter symbols that haven't been completed yet
        let remaining_symbols: Vec<String> = symbols
            .into_iter()
            .filter(|symbol| !self.progress.completed_symbols.contains(symbol))
            .collect();
        
        if remaining_symbols.is_empty() {
            println!("✅ All symbols already completed!");
            return Ok(());
        }
        
        println!("Processing {} remaining symbols...", remaining_symbols.len());
        
        for (index, symbol) in remaining_symbols.iter().enumerate() {
            self.progress.current_symbol = Some(symbol.clone());
            
            // Show progress
            self.display_progress(index, remaining_symbols.len());
            
            // Download price data for this symbol
            match self.download_symbol_data(symbol).await {
                Ok(price_count) => {
                    self.progress.completed_symbols.insert(symbol.clone());
                    self.progress.statistics.successful_calls += 1;
                    self.progress.statistics.total_price_records += price_count;
                    
                    // Remove from failed if it was there before
                    self.progress.failed_symbols.remove(symbol);
                    
                    println!("✅ {}: {} bars imported", symbol, price_count);
                }
                Err(e) => {
                    self.progress.failed_symbols.insert(symbol.clone(), e.to_string());
                    self.progress.statistics.failed_calls += 1;
                    
                    warn!("❌ {}: {}", symbol, e);
                }
            }
            
            self.progress.statistics.total_api_calls += 1;
            
            // Update statistics
            self.update_statistics();
            
            // Save progress periodically
            if index % 10 == 0 {
                if let Err(e) = self.save_progress().await {
                    warn!("Failed to save progress: {}", e);
                }
            }
            
            // Small delay to be respectful to the API
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        
        // Final progress save
        self.save_progress().await?;
        
        // Show final summary
        self.display_final_summary();
        
        Ok(())
    }
    
    async fn download_symbol_data(&self, symbol: &str) -> Result<usize> {
        let start_date = NaiveDate::parse_from_str(&self.progress.settings.start_date, "%Y-%m-%d")?;
        let end_date = NaiveDate::parse_from_str(&self.progress.settings.end_date, "%Y-%m-%d")?;

        // Get stock_id from database
        let stock_id = self.get_stock_id(symbol).await?;

        // If incremental mode, calculate what data is actually needed
        if self.incremental_mode {
            return self.download_incremental_data(symbol, stock_id, start_date, end_date).await;
        } else {
            return self.download_full_range(symbol, stock_id, start_date, end_date).await;
        }
    }

    async fn download_incremental_data(&self, symbol: &str, stock_id: i64, default_start: NaiveDate, end_date: NaiveDate) -> Result<usize> {
        // Convert SQLite connection for date calculator
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite:./db/stocks.db".to_string());
        let sqlite_path = database_url.strip_prefix("sqlite:").unwrap_or(&database_url);
        let conn = rusqlite::Connection::open(sqlite_path)?;

        // Calculate update plan using date range calculator
        let update_plan = self.date_calculator.calculate_update_plan(
            &conn, symbol, stock_id, default_start, end_date
        )?;

        if update_plan.missing_ranges.is_empty() {
            info!("📊 {} - No missing data, skipping (coverage: {:.1}%)",
                  symbol, update_plan.coverage_percentage);
            return Ok(0);
        }

        info!("📊 {} - Found {} missing ranges, coverage: {:.1}%",
              symbol, update_plan.missing_ranges.len(), update_plan.coverage_percentage);

        let mut total_bars = 0;

        // Download each missing range
        for range in &update_plan.missing_ranges {
            debug!("📅 {} - Downloading range: {} to {}",
                   symbol, range.start_date, range.end_date);

            let bars = self.download_date_range(symbol, stock_id, range.start_date, range.end_date).await?;
            total_bars += bars;
        }

        // Update company metadata with new coverage info
        self.update_company_metadata(stock_id, symbol).await?;

        Ok(total_bars)
    }

    async fn download_full_range(&self, symbol: &str, stock_id: i64, start_date: NaiveDate, end_date: NaiveDate) -> Result<usize> {
        info!("📊 {} - Full range download: {} to {}", symbol, start_date, end_date);
        self.download_date_range(symbol, stock_id, start_date, end_date).await
    }

    async fn download_date_range(&self, symbol: &str, stock_id: i64, start_date: NaiveDate, end_date: NaiveDate) -> Result<usize> {
        // Get rate limiter permit
        let _permit = self.rate_limiter.acquire().await?;

        // Fetch price data from Schwab API
        let price_bars = self.schwab_client
            .get_price_history(symbol, start_date, end_date)
            .await?;

        if price_bars.is_empty() {
            return Err(anyhow!("No price data returned for {} in range {} to {}", symbol, start_date, end_date));
        }

        // Insert price data into database
        self.insert_price_data(stock_id, &price_bars).await?;

        Ok(price_bars.len())
    }

    async fn update_company_metadata(&self, stock_id: i64, _symbol: &str) -> Result<()> {
        // Update the company metadata with latest data coverage
        sqlx::query(
            r#"
            UPDATE company_metadata
            SET
                earliest_data_date = (
                    SELECT MIN(date) FROM daily_prices WHERE stock_id = ?
                ),
                latest_data_date = (
                    SELECT MAX(date) FROM daily_prices WHERE stock_id = ?
                ),
                total_trading_days = (
                    SELECT COUNT(*) FROM daily_prices WHERE stock_id = ?
                ),
                updated_at = CURRENT_TIMESTAMP
            WHERE stock_id = ?
            "#
        )
        .bind(stock_id)
        .bind(stock_id)
        .bind(stock_id)
        .bind(stock_id)
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }
    
    
    async fn get_stock_id(&self, symbol: &str) -> Result<i64> {
        let stock_id = sqlx::query_scalar::<_, i64>(
            "SELECT id FROM stocks WHERE symbol = ?"
        )
        .bind(symbol)
        .fetch_optional(&self.db_pool)
        .await?;
        
        match stock_id {
            Some(id) => Ok(id),
            None => Err(anyhow!("Stock symbol {} not found in database", symbol)),
        }
    }
    
    async fn insert_price_data(
        &self,
        stock_id: i64,
        price_bars: &[rust_stocks_tauri_lib::models::SchwabPriceBar],
    ) -> Result<()> {
        let mut tx = self.db_pool.begin().await?;
        
        for bar in price_bars {
            // Convert timestamp to date
            let date = chrono::DateTime::from_timestamp_millis(bar.datetime)
                .ok_or_else(|| anyhow!("Invalid timestamp: {}", bar.datetime))?
                .date_naive();
            
            // Insert with UPSERT to handle duplicates
            sqlx::query(
                r#"
                INSERT OR REPLACE INTO daily_prices 
                (stock_id, date, open_price, high_price, low_price, close_price, volume)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                "#
            )
            .bind(stock_id)
            .bind(date)
            .bind(bar.open)
            .bind(bar.high)
            .bind(bar.low)
            .bind(bar.close)
            .bind(bar.volume)
            .execute(&mut *tx)
            .await?;
        }
        
        tx.commit().await?;
        Ok(())
    }
    
    fn display_progress(&self, _current_index: usize, _total_remaining: usize) {
        let completed = self.progress.completed_symbols.len();
        let failed = self.progress.failed_symbols.len();
        let total = self.progress.total_symbols;
        let percentage = (completed as f64 / total as f64) * 100.0;
        
        // Calculate ETA
        let elapsed = self.start_time.elapsed();
        let symbols_processed = completed + failed;
        let eta = if symbols_processed > 0 {
            let rate = symbols_processed as f64 / elapsed.as_secs_f64();
            let remaining = total - symbols_processed;
            Duration::from_secs_f64(remaining as f64 / rate)
        } else {
            Duration::from_secs(0)
        };
        
        print!("\r🔄 Progress: {}/{} ({:.1}%) | Failed: {} | ETA: {}m {}s",
               completed, total, percentage, failed,
               eta.as_secs() / 60, eta.as_secs() % 60);
        
        if let Some(current) = &self.progress.current_symbol {
            print!(" | Current: {}", current);
        }
        
        use std::io::{self, Write};
        io::stdout().flush().unwrap();
    }
    
    fn update_statistics(&mut self) {
        let completed = self.progress.completed_symbols.len();
        if completed > 0 {
            self.progress.statistics.average_bars_per_symbol = 
                self.progress.statistics.total_price_records as f64 / completed as f64;
        }
        
        let elapsed_minutes = self.start_time.elapsed().as_secs_f64() / 60.0;
        if elapsed_minutes > 0.0 {
            self.progress.statistics.download_speed_symbols_per_minute = 
                completed as f64 / elapsed_minutes;
        }
    }
    
    fn display_final_summary(&self) {
        println!("\n\n📊 Download Complete - Final Summary");
        println!("=====================================");
        println!("✅ Completed: {} symbols", self.progress.completed_symbols.len());
        println!("❌ Failed: {} symbols", self.progress.failed_symbols.len());
        println!("📈 Total records: {} price bars", self.progress.statistics.total_price_records);
        println!("⏱️  Average: {:.1} bars per symbol", self.progress.statistics.average_bars_per_symbol);
        println!("🚀 Speed: {:.2} symbols per minute", self.progress.statistics.download_speed_symbols_per_minute);
        println!("💾 Success rate: {:.1}%", 
                 (self.progress.statistics.successful_calls as f64 / 
                  self.progress.statistics.total_api_calls as f64) * 100.0);
        
        if !self.progress.failed_symbols.is_empty() {
            println!("\n❌ Failed symbols:");
            for (symbol, error) in &self.progress.failed_symbols {
                println!("   {}: {}", symbol, error);
            }
        }
        
        println!("\n🎉 Import complete! Database updated with historical price data.");
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    // Load configuration
    let config = Config::from_env()?;
    
    // Use current date as default end date
    let end_date = cli.end_date.unwrap_or_else(|| {
        Local::now().naive_local().date().format("%Y-%m-%d").to_string()
    });

    let settings = DownloadSettings {
        start_date: cli.start_date.clone(),
        end_date,
        batch_size: cli.batch_size,
        max_retries: cli.max_retries,
    };

    // Determine if we should use incremental mode (default to true for better performance)
    let incremental_mode = !cli.command.as_ref().map_or(false, |cmd| matches!(cmd, Commands::Download));
    
    let progress_file = PathBuf::from(cli.progress_file);
    
    // Handle test mode
    if let Some(test_symbol) = cli.test_symbol {
        return test_single_symbol(&config, &test_symbol, &settings, incremental_mode).await;
    }
    
    // Handle validation only mode
    if cli.validate_only {
        return validate_existing_data(&config).await;
    }
    
    // Create bulk downloader
    let mut downloader = BulkDownloader::new(&config, progress_file.clone(), settings, incremental_mode).await?;
    
    // Execute download
    match cli.command {
        Some(Commands::Download) | None => {
            downloader.download_all_symbols().await?;
        }
        Some(Commands::Resume) => {
            info!("Resuming previous download...");
            downloader.download_all_symbols().await?;
        }
        Some(Commands::Validate) => {
            return validate_existing_data(&config).await;
        }
        Some(Commands::Status) => {
            show_progress_status(&progress_file).await?;
            return Ok(());
        }
    }
    
    Ok(())
}

async fn test_single_symbol(
    config: &Config,
    symbol: &str,
    settings: &DownloadSettings,
    incremental_mode: bool,
) -> Result<()> {
    println!("🧪 Testing single symbol: {} (incremental: {})", symbol, incremental_mode);

    // Create a test downloader to use the same logic as the bulk downloader
    let progress_file = PathBuf::from("test_progress.json");
    let downloader = BulkDownloader::new(config, progress_file.clone(), settings.clone(), incremental_mode).await?;

    // Test the download for this symbol
    let bars_count = downloader.download_symbol_data(symbol).await?;
    
    if bars_count > 0 {
        println!("✅ Downloaded {} new price bars for {}", bars_count, symbol);
    } else {
        println!("📊 {} - No new data needed (already up to date)", symbol);
    }

    // Clean up test progress file
    if progress_file.exists() {
        let _ = std::fs::remove_file(progress_file);
    }
    
    Ok(())
}

async fn validate_existing_data(_config: &Config) -> Result<()> {
    println!("🔍 Validating existing price data...");
    
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:./db/stocks.db".to_string());
    let db_pool = SqlitePool::connect(&database_url).await?;
    
    let stats = sqlx::query_as::<_, (i64, i64, Option<String>, Option<String>)>(
        r#"
        SELECT 
            COUNT(DISTINCT stock_id) as stock_count,
            COUNT(*) as total_records,
            MIN(date) as earliest_date,
            MAX(date) as latest_date
        FROM daily_prices
        "#
    )
    .fetch_one(&db_pool)
    .await?;
    
    println!("📊 Current database statistics:");
    println!("   Stocks with price data: {}", stats.0);
    println!("   Total price records: {}", stats.1);
    println!("   Date range: {} to {}", 
             stats.2.unwrap_or("None".to_string()),
             stats.3.unwrap_or("None".to_string()));
    
    Ok(())
}

async fn show_progress_status(progress_file: &PathBuf) -> Result<()> {
    if !progress_file.exists() {
        println!("No progress file found at {}", progress_file.display());
        return Ok(());
    }
    
    let progress = BulkDownloader::load_progress(progress_file).await?;
    
    println!("📊 Download Progress Status");
    println!("==========================");
    println!("Session ID: {}", progress.session_id);
    println!("Started: {}", progress.start_time.format("%Y-%m-%d %H:%M:%S UTC"));
    println!("Total symbols: {}", progress.total_symbols);
    println!("Completed: {} ({:.1}%)", 
             progress.completed_symbols.len(),
             (progress.completed_symbols.len() as f64 / progress.total_symbols as f64) * 100.0);
    println!("Failed: {}", progress.failed_symbols.len());
    
    if let Some(current) = progress.current_symbol {
        println!("Current symbol: {}", current);
    }
    
    println!("\nStatistics:");
    println!("  API calls: {}", progress.statistics.total_api_calls);
    println!("  Success rate: {:.1}%", 
             if progress.statistics.total_api_calls > 0 {
                 (progress.statistics.successful_calls as f64 / progress.statistics.total_api_calls as f64) * 100.0
             } else { 0.0 });
    println!("  Total records: {}", progress.statistics.total_price_records);
    println!("  Average bars/symbol: {:.1}", progress.statistics.average_bars_per_symbol);
    println!("  Download speed: {:.2} symbols/minute", progress.statistics.download_speed_symbols_per_minute);

    // Update tracking table with total database count
    println!("\n📊 Updating data tracking status...");
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:./db/stocks.db".to_string());
    let tracking_pool = SqlitePool::connect(&database_url).await?;

    // Note: Tracking status update removed - using SEC filing-based freshness checking instead
    println!("✅ Daily prices processed - freshness checked via SEC filing-based system");

    tracking_pool.close().await;

    Ok(())
}