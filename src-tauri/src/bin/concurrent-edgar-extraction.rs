/// Concurrent EDGAR Financial Data Extraction
/// 
/// High-performance concurrent extraction system that processes multiple EDGAR files
/// simultaneously using a work queue and thread pool architecture.

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::fs as async_fs;
use tokio::sync::{Mutex, Semaphore};
use tokio::task::JoinHandle;
use tokio::time::interval;
use tracing::{info, warn, debug, error};
use rust_stocks_tauri_lib::tools::data_freshness_checker::DataStatusReader;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    /// Number of concurrent workers
    #[arg(long, default_value = "10")]
    workers: usize,
    
    /// Batch size for database operations
    #[arg(long, default_value = "50")]
    batch_size: usize,
    
    /// Progress reporting interval in seconds
    #[arg(long, default_value = "5")]
    progress_interval: u64,
    
    /// Maximum concurrent file reads
    #[arg(long, default_value = "10")]
    max_file_readers: usize,
    
    /// Test mode - limit number of companies
    #[arg(long)]
    test_limit: Option<usize>,
    
    /// Specific symbols to process (comma-separated)
    #[arg(long)]
    symbols: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract financial data from EDGAR files
    Extract,
    /// Show current extraction status
    Status,
    /// Resume interrupted extraction
    Resume,
    /// Test with small subset
    Test,
}

// EDGAR JSON structures
#[derive(Debug, Deserialize)]
struct EdgarCompanyFacts {
    _cik: i64,
    #[serde(rename = "entityName")]
    _entity_name: String,
    facts: EdgarFacts,
}

#[derive(Debug, Deserialize)]
struct EdgarFacts {
    #[serde(rename = "us-gaap")]
    us_gaap: HashMap<String, EdgarConcept>,
}

#[derive(Debug, Deserialize)]
struct EdgarConcept {
    units: HashMap<String, Vec<EdgarFactValue>>,
}

#[derive(Debug, Deserialize)]
struct EdgarFactValue {
    end: String,  // Date in YYYY-MM-DD format
    val: f64,
    fy: Option<i32>,      // Fiscal year (can be null)
    fp: Option<String>,   // Fiscal period "Q1", "Q2", "Q3", "Q4", "FY" (can be null)
}

// Extracted financial data structures
#[derive(Debug)]
struct ExtractedFinancialData {
    income_statements: Vec<IncomeStatementData>,
    balance_sheets: Vec<BalanceSheetData>,
}

#[derive(Debug, Clone)]
struct PeriodInfo {
    year: i32,
    period: String,
    end_date: String,
}

#[derive(Debug)]
struct IncomeStatementData {
    stock_id: i64,
    period: String,  // "Q1", "Q2", "Q3", "Q4", "FY"
    year: i32,
    end_date: String,
    revenue: Option<f64>,
    net_income: Option<f64>,
    operating_income: Option<f64>,
    shares_basic: Option<f64>,
    shares_diluted: Option<f64>,
}

#[derive(Debug)]
struct BalanceSheetData {
    stock_id: i64,
    period: String,
    year: i32,
    end_date: String,
    total_assets: Option<f64>,
    total_debt: Option<f64>,
    total_equity: Option<f64>,
    cash_and_equivalents: Option<f64>,
    shares_outstanding: Option<f64>,
}

// GAAP field mapping configuration
struct GaapFieldMapping {
    income_statement_fields: HashMap<String, Vec<String>>,
    balance_sheet_fields: HashMap<String, Vec<String>>,
}

impl GaapFieldMapping {
    fn new() -> Self {
        let mut income_statement_fields = HashMap::new();
        let mut balance_sheet_fields = HashMap::new();
        
        // Income statement field mappings (priority order)
        income_statement_fields.insert("revenue".to_string(), vec![
            "RevenueFromContractWithCustomerExcludingAssessedTax".to_string(),
            "SalesRevenueNet".to_string(),
            "Revenues".to_string(),
            "RevenueFromContractWithCustomerIncludingAssessedTax".to_string(),
        ]);
        
        income_statement_fields.insert("net_income".to_string(), vec![
            "NetIncomeLoss".to_string(),
            "NetIncomeLossAvailableToCommonStockholdersBasic".to_string(),
            "ProfitLoss".to_string(),
        ]);
        
        income_statement_fields.insert("operating_income".to_string(), vec![
            "IncomeLossFromContinuingOperations".to_string(),
            "OperatingIncomeLoss".to_string(),
            "IncomeLossFromContinuingOperationsBeforeIncomeTaxesExtraordinaryItemsNoncontrollingInterest".to_string(),
        ]);
        
        income_statement_fields.insert("shares_basic".to_string(), vec![
            "WeightedAverageNumberOfSharesOutstandingBasic".to_string(),
            "CommonStockSharesOutstanding".to_string(),
        ]);
        
        income_statement_fields.insert("shares_diluted".to_string(), vec![
            "WeightedAverageNumberOfDilutedSharesOutstanding".to_string(),
            "WeightedAverageNumberOfSharesOutstandingBasic".to_string(),
        ]);
        
        // Balance sheet field mappings (priority order)
        balance_sheet_fields.insert("total_assets".to_string(), vec![
            "Assets".to_string(),
            "AssetsTotal".to_string(),
        ]);
        
        balance_sheet_fields.insert("total_debt".to_string(), vec![
            "LongTermDebt".to_string(),
            "DebtAndCapitalLeaseObligations".to_string(),
            "LongTermDebtAndCapitalLeaseObligations".to_string(),
        ]);
        
        balance_sheet_fields.insert("total_equity".to_string(), vec![
            "StockholdersEquity".to_string(),
            "ShareholdersEquity".to_string(),
            "StockholdersEquityIncludingPortionAttributableToNoncontrollingInterest".to_string(),
        ]);
        
        balance_sheet_fields.insert("cash_and_equivalents".to_string(), vec![
            "CashAndCashEquivalentsAtCarryingValue".to_string(),
            "CashCashEquivalentsAndShortTermInvestments".to_string(),
            "Cash".to_string(),
        ]);
        
        balance_sheet_fields.insert("shares_outstanding".to_string(), vec![
            "CommonStockSharesOutstanding".to_string(),
            "CommonStockSharesIssued".to_string(),
        ]);
        
        Self {
            income_statement_fields,
            balance_sheet_fields,
        }
    }
}

// Work queue task structure
#[derive(Debug, Clone)]
struct ExtractionTask {
    cik: String,
    symbol: String,
    stock_id: i64,
    _company_name: String,
    edgar_file_path: PathBuf,
    priority: u8,
}

// Work queue manager
struct WorkQueueManager {
    pending_tasks: Arc<Mutex<VecDeque<ExtractionTask>>>,
    completed_tasks: Arc<AtomicUsize>,
    failed_tasks: Arc<AtomicUsize>,
    total_tasks: AtomicUsize,
}

impl WorkQueueManager {
    async fn new() -> Self {
        Self {
            pending_tasks: Arc::new(Mutex::new(VecDeque::new())),
            completed_tasks: Arc::new(AtomicUsize::new(0)),
            failed_tasks: Arc::new(AtomicUsize::new(0)),
            total_tasks: AtomicUsize::new(0),
        }
    }
    
    async fn populate_from_database(&self, db_pool: &SqlitePool, test_limit: Option<usize>, symbols_filter: Option<&str>) -> Result<()> {
        info!("📋 Loading extraction tasks from database...");
        
        // Build query based on filters
        let mut query = "SELECT cik, symbol, stock_id, company_name, edgar_file_path FROM cik_mappings_sp500 WHERE file_exists = 1".to_string();
        let mut bindings = Vec::new();
        
        if let Some(symbols) = symbols_filter {
            let symbol_list: Vec<&str> = symbols.split(',').collect();
            let placeholders = symbol_list.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            query.push_str(&format!(" AND symbol IN ({})", placeholders));
            bindings.extend(symbol_list.iter().map(|s| s.trim()));
        }
        
        query.push_str(" ORDER BY symbol");
        
        if let Some(limit) = test_limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }
        
        // Execute query with dynamic bindings
        let mut query_builder = sqlx::query_as::<_, (String, String, i64, String, String)>(&query);
        for binding in bindings {
            query_builder = query_builder.bind(binding);
        }
        
        let mappings = query_builder.fetch_all(db_pool).await?;
        
        let mut tasks = VecDeque::new();
        
        for (cik, symbol, stock_id, company_name, file_path) in mappings {
            // Assign priority based on company size/importance
            let priority = match symbol.as_str() {
                "AAPL" | "MSFT" | "GOOGL" | "AMZN" | "NVDA" => 10,
                "META" | "TSLA" | "AVGO" | "JPM" | "ORCL" => 9,
                _ => 5,
            };
            
            tasks.push_back(ExtractionTask {
                cik,
                symbol,
                stock_id,
                _company_name: company_name,
                edgar_file_path: PathBuf::from(file_path),
                priority,
            });
        }
        
        // Sort by priority (highest first)
        let mut tasks_vec: Vec<_> = tasks.into();
        tasks_vec.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        let task_count = tasks_vec.len();
        self.total_tasks.store(task_count, Ordering::Relaxed);
        
        let mut pending = self.pending_tasks.lock().await;
        *pending = tasks_vec.into();
        
        info!("✅ Loaded {} extraction tasks", task_count);
        Ok(())
    }
    
    async fn get_next_task(&self) -> Option<ExtractionTask> {
        let mut pending = self.pending_tasks.lock().await;
        pending.pop_front()
    }
    
    async fn mark_completed(&self) {
        self.completed_tasks.fetch_add(1, Ordering::Relaxed);
    }
    
    async fn mark_failed(&self) {
        self.failed_tasks.fetch_add(1, Ordering::Relaxed);
    }
    
    fn get_stats(&self) -> (usize, usize, usize, usize) {
        let total = self.total_tasks.load(Ordering::Relaxed);
        let completed = self.completed_tasks.load(Ordering::Relaxed);
        let failed = self.failed_tasks.load(Ordering::Relaxed);
        let remaining = total.saturating_sub(completed + failed);
        (total, completed, failed, remaining)
    }
}

// Worker statistics
#[derive(Debug, Clone)]
struct WorkerStats {
    _worker_id: usize,
    processed_count: usize,
    current_company: Option<String>,
    last_update: DateTime<Utc>,
    status: WorkerStatus,
}

#[derive(Debug, Clone)]
enum WorkerStatus {
    Idle,
    ReadingFile,
    ProcessingData,
    WritingDatabase,
    Error(String),
}

// Progress tracker
struct ProgressTracker {
    start_time: DateTime<Utc>,
    worker_stats: Arc<Mutex<HashMap<usize, WorkerStats>>>,
    active_workers: Arc<AtomicUsize>,
}

impl ProgressTracker {
    fn new() -> Self {
        Self {
            start_time: Utc::now(),
            worker_stats: Arc::new(Mutex::new(HashMap::new())),
            active_workers: Arc::new(AtomicUsize::new(0)),
        }
    }
    
    async fn update_worker_status(&self, worker_id: usize, status: WorkerStatus, current_company: Option<String>) {
        let mut stats = self.worker_stats.lock().await;
        let worker_stat = stats.entry(worker_id).or_insert(WorkerStats {
            _worker_id: worker_id,
            processed_count: 0,
            current_company: None,
            last_update: Utc::now(),
            status: WorkerStatus::Idle,
        });
        
        worker_stat.status = status;
        worker_stat.current_company = current_company;
        worker_stat.last_update = Utc::now();
    }
    
    async fn increment_worker_processed(&self, worker_id: usize) {
        let mut stats = self.worker_stats.lock().await;
        if let Some(worker_stat) = stats.get_mut(&worker_id) {
            worker_stat.processed_count += 1;
        }
    }
    
    async fn display_progress(&self, work_queue: &WorkQueueManager) {
        let (total, completed, failed, remaining) = work_queue.get_stats();
        let elapsed = Utc::now().signed_duration_since(self.start_time);
        let active = self.active_workers.load(Ordering::Relaxed);
        
        let rate = if elapsed.num_seconds() > 0 {
            completed as f64 / elapsed.num_seconds() as f64 * 60.0
        } else {
            0.0
        };
        
        let eta_minutes = if rate > 0.0 {
            remaining as f64 / rate
        } else {
            0.0
        };
        
        println!("\n🏗️ Concurrent EDGAR Extraction Progress");
        println!("=======================================");
        println!("Total: {} | Completed: {} | Failed: {} | Remaining: {}", 
                 total, completed, failed, remaining);
        println!("Active Workers: {} | Processing Rate: {:.1} companies/minute", active, rate);
        println!("Elapsed: {}m {}s | ETA: {:.1}m", 
                 elapsed.num_minutes(), elapsed.num_seconds() % 60, eta_minutes);
        
        if completed > 0 {
            let success_rate = (completed as f64 / (completed + failed) as f64) * 100.0;
            println!("Success Rate: {:.1}%", success_rate);
        }
        
        // Show worker details
        let worker_stats = self.worker_stats.lock().await;
        if !worker_stats.is_empty() {
            println!("\n👷 Worker Status:");
            for (_worker_id, stats) in worker_stats.iter() {
                let status_str = match &stats.status {
                    WorkerStatus::Idle => "💤 Idle",
                    WorkerStatus::ReadingFile => "📖 Reading",
                    WorkerStatus::ProcessingData => "⚙️ Processing", 
                    WorkerStatus::WritingDatabase => "💾 Writing",
                    WorkerStatus::Error(e) => &format!("❌ Error: {}", e),
                };
                
                let company_str = stats.current_company
                    .as_deref()
                    .unwrap_or("none");
                
                println!("   Worker {}: {} processed | {} | Current: {}", 
                         stats._worker_id, stats.processed_count, status_str, company_str);
            }
        }
        println!();
    }
}

// Main concurrent extractor
struct ConcurrentEdgarExtractor {
    db_pool: Arc<SqlitePool>,
    work_queue: Arc<WorkQueueManager>,
    progress_tracker: Arc<ProgressTracker>,
    worker_handles: Vec<JoinHandle<Result<()>>>,
    file_semaphore: Arc<Semaphore>,
    config: ExtractionConfig,
}

#[derive(Debug, Clone)]
struct ExtractionConfig {
    max_workers: usize,
    batch_size: usize,
    max_file_readers: usize,
    progress_interval: Duration,
}

impl ConcurrentEdgarExtractor {
    async fn new(config: ExtractionConfig) -> Result<Self> {
        // Connect to database
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite:./db/stocks.db".to_string());
        let db_pool = Arc::new(SqlitePool::connect(&database_url).await?);
        
        // Initialize components
        let work_queue = Arc::new(WorkQueueManager::new().await);
        let progress_tracker = Arc::new(ProgressTracker::new());
        let file_semaphore = Arc::new(Semaphore::new(config.max_file_readers));
        
        Ok(Self {
            db_pool,
            work_queue,
            progress_tracker,
            worker_handles: Vec::new(),
            file_semaphore,
            config,
        })
    }
    
    async fn populate_work_queue(&self, test_limit: Option<usize>, symbols_filter: Option<&str>) -> Result<()> {
        self.work_queue.populate_from_database(&self.db_pool, test_limit, symbols_filter).await
    }
    
    async fn spawn_workers(&mut self) -> Result<()> {
        info!("🚀 Spawning {} worker threads...", self.config.max_workers);
        
        for worker_id in 0..self.config.max_workers {
            let db_pool = Arc::clone(&self.db_pool);
            let work_queue = Arc::clone(&self.work_queue);
            let progress_tracker = Arc::clone(&self.progress_tracker);
            let file_semaphore = Arc::clone(&self.file_semaphore);
            
            let handle = tokio::spawn(async move {
                worker_thread(worker_id, db_pool, work_queue, progress_tracker, file_semaphore).await
            });
            
            self.worker_handles.push(handle);
        }
        
        Ok(())
    }
    
    async fn start_progress_monitor(&self) {
        let work_queue = Arc::clone(&self.work_queue);
        let progress_tracker = Arc::clone(&self.progress_tracker);
        let interval_duration = self.config.progress_interval;
        
        tokio::spawn(async move {
            let mut ticker = interval(interval_duration);
            loop {
                ticker.tick().await;
                progress_tracker.display_progress(&work_queue).await;
                
                // Check if work is complete
                let (total, _completed, _failed, remaining) = work_queue.get_stats();
                if remaining == 0 && total > 0 {
                    break;
                }
            }
        });
    }
    
    async fn wait_for_completion(&mut self) -> Result<()> {
        info!("⏳ Waiting for all workers to complete...");
        
        // Wait for all workers to finish
        while let Some(handle) = self.worker_handles.pop() {
            if let Err(e) = handle.await? {
                error!("Worker thread failed: {}", e);
            }
        }
        
        // Final progress display
        self.progress_tracker.display_progress(&self.work_queue).await;
        
        let (total, completed, failed, _) = self.work_queue.get_stats();
        
        info!("🎉 Extraction completed!");
        info!("   Total companies: {}", total);
        info!("   Successfully processed: {}", completed);
        info!("   Failed: {}", failed);
        info!("   Success rate: {:.1}%", (completed as f64 / total as f64) * 100.0);

        // Update tracking status for financial statements if we processed any companies
        if completed > 0 {
            info!("📊 Updating financial data tracking status...");

            // Get the latest period date from imported data
            let latest_date = sqlx::query_scalar::<_, Option<String>>(
                "SELECT MAX(report_date) FROM income_statements WHERE data_source = 'edgar'"
            ).fetch_one(&*self.db_pool).await.unwrap_or(None);

            // Count total records imported from EDGAR
            let total_records = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM (
                    SELECT 1 FROM income_statements WHERE data_source = 'edgar'
                    UNION ALL
                    SELECT 1 FROM balance_sheets WHERE data_source = 'edgar'
                )"
            ).fetch_one(&*self.db_pool).await.unwrap_or(0);

            if let Err(e) = DataStatusReader::update_import_status(
                &self.db_pool,
                "financial_statements",
                total_records,
                latest_date.as_deref()
            ).await {
                warn!("⚠️ Failed to update tracking status: {}", e);
            } else {
                info!("✅ Financial data tracking status updated ({} records)", total_records);
            }
        }

        Ok(())
    }
}

// Worker thread implementation
async fn worker_thread(
    worker_id: usize,
    db_pool: Arc<SqlitePool>,
    work_queue: Arc<WorkQueueManager>,
    progress_tracker: Arc<ProgressTracker>,
    file_semaphore: Arc<Semaphore>,
) -> Result<()> {
    debug!("Worker {} started", worker_id);
    progress_tracker.active_workers.fetch_add(1, Ordering::Relaxed);
    
    loop {
        // Update status to idle
        progress_tracker.update_worker_status(worker_id, WorkerStatus::Idle, None).await;
        
        // Get next task
        let task = match work_queue.get_next_task().await {
            Some(task) => task,
            None => {
                debug!("Worker {} finished - no more tasks", worker_id);
                break;
            }
        };
        
        info!("Worker {} processing {}", worker_id, task.symbol);
        
        // Update status
        progress_tracker.update_worker_status(
            worker_id, 
            WorkerStatus::ReadingFile, 
            Some(task.symbol.clone())
        ).await;
        
        // Process the task
        match process_extraction_task(&task, &db_pool, &file_semaphore, &progress_tracker, worker_id).await {
            Ok(_) => {
                work_queue.mark_completed().await;
                progress_tracker.increment_worker_processed(worker_id).await;
                debug!("Worker {} completed {}", worker_id, task.symbol);
            }
            Err(e) => {
                work_queue.mark_failed().await;
                progress_tracker.update_worker_status(
                    worker_id,
                    WorkerStatus::Error(e.to_string()),
                    Some(task.symbol.clone())
                ).await;
                warn!("Worker {} failed on {}: {}", worker_id, task.symbol, e);
            }
        }
    }
    
    progress_tracker.active_workers.fetch_sub(1, Ordering::Relaxed);
    debug!("Worker {} shutdown", worker_id);
    Ok(())
}

// Process individual extraction task with real EDGAR data
async fn process_extraction_task(
    task: &ExtractionTask,
    db_pool: &Arc<SqlitePool>,
    file_semaphore: &Arc<Semaphore>,
    progress_tracker: &Arc<ProgressTracker>,
    worker_id: usize,
) -> Result<()> {
    // Acquire file reading semaphore
    let _permit = file_semaphore.acquire().await?;
    
    // Update status
    progress_tracker.update_worker_status(worker_id, WorkerStatus::ReadingFile, Some(task.symbol.clone())).await;
    
    // Read and parse EDGAR JSON file
    let content = async_fs::read_to_string(&task.edgar_file_path).await
        .map_err(|e| anyhow!("Failed to read EDGAR file for {}: {}", task.symbol, e))?;
    
    let edgar_data: EdgarCompanyFacts = serde_json::from_str(&content)
        .map_err(|e| anyhow!("Failed to parse EDGAR JSON for {}: {}", task.symbol, e))?;
    
    // Update status
    progress_tracker.update_worker_status(worker_id, WorkerStatus::ProcessingData, Some(task.symbol.clone())).await;
    
    // Extract financial data using GAAP field mapping
    let gaap_mapping = GaapFieldMapping::new();
    let extracted_data = extract_financial_statements(&edgar_data, task.stock_id, &gaap_mapping)?;
    
    debug!("Extracted {} income statements and {} balance sheets for {}", 
           extracted_data.income_statements.len(), 
           extracted_data.balance_sheets.len(), 
           task.symbol);
    
    // Update status
    progress_tracker.update_worker_status(worker_id, WorkerStatus::WritingDatabase, Some(task.symbol.clone())).await;
    
    // Insert data into database
    insert_financial_data_to_db(db_pool, &extracted_data).await
        .map_err(|e| anyhow!("Failed to insert data for {}: {}", task.symbol, e))?;
    
    debug!("Successfully processed {} (CIK: {})", task.symbol, task.cik);
    Ok(())
}

// Extract financial statements from EDGAR data
fn extract_financial_statements(
    edgar_data: &EdgarCompanyFacts,
    stock_id: i64,
    gaap_mapping: &GaapFieldMapping,
) -> Result<ExtractedFinancialData> {
    let mut income_statements = Vec::new();
    let mut balance_sheets = Vec::new();
    
    // Extract periods from GAAP facts
    let periods = extract_available_periods(&edgar_data.facts.us_gaap)?;
    
    for period_info in periods {
        // Extract income statement data for this period
        if let Ok(income_stmt) = extract_income_statement_for_period(
            &edgar_data.facts.us_gaap, 
            stock_id, 
            &period_info,
            gaap_mapping,
        ) {
            income_statements.push(income_stmt);
        }
        
        // Extract balance sheet data for this period
        if let Ok(balance_sheet) = extract_balance_sheet_for_period(
            &edgar_data.facts.us_gaap, 
            stock_id, 
            &period_info,
            gaap_mapping,
        ) {
            balance_sheets.push(balance_sheet);
        }
    }
    
    Ok(ExtractedFinancialData {
        income_statements,
        balance_sheets,
    })
}

fn extract_available_periods(gaap_facts: &HashMap<String, EdgarConcept>) -> Result<Vec<PeriodInfo>> {
    let mut periods = Vec::new();
    let mut seen_periods = HashSet::new();
    
    // Look through all fields to find available periods
    for (_field_name, concept) in gaap_facts {
        if let Some(usd_values) = concept.units.get("USD") {
            for fact_value in usd_values {
                if let (Some(fy), Some(fp)) = (fact_value.fy, fact_value.fp.as_ref()) {
                    let period_key = format!("{}-{}", fy, fp);
                    if !seen_periods.contains(&period_key) {
                        seen_periods.insert(period_key);
                        periods.push(PeriodInfo {
                            year: fy,
                            period: fp.clone(),
                            end_date: fact_value.end.clone(),
                        });
                    }
                }
            }
        }
    }
    
    // Sort periods by year and quarter
    periods.sort_by(|a, b| {
        a.year.cmp(&b.year).then_with(|| {
            let order_a = match a.period.as_str() {
                "Q1" => 1, "Q2" => 2, "Q3" => 3, "Q4" => 4, "FY" => 5,
                _ => 99,
            };
            let order_b = match b.period.as_str() {
                "Q1" => 1, "Q2" => 2, "Q3" => 3, "Q4" => 4, "FY" => 5,
                _ => 99,
            };
            order_a.cmp(&order_b)
        })
    });
    
    Ok(periods)
}

fn extract_income_statement_for_period(
    gaap_facts: &HashMap<String, EdgarConcept>,
    stock_id: i64,
    period_info: &PeriodInfo,
    gaap_mapping: &GaapFieldMapping,
) -> Result<IncomeStatementData> {
    let mut income_stmt = IncomeStatementData {
        stock_id,
        period: period_info.period.clone(),
        year: period_info.year,
        end_date: period_info.end_date.clone(),
        revenue: None,
        net_income: None,
        operating_income: None,
        shares_basic: None,
        shares_diluted: None,
    };
    
    // Extract revenue using priority field mapping
    income_stmt.revenue = extract_field_value_for_period(
        gaap_facts,
        &gaap_mapping.income_statement_fields["revenue"],
        period_info,
    );
    
    // Extract net income
    income_stmt.net_income = extract_field_value_for_period(
        gaap_facts,
        &gaap_mapping.income_statement_fields["net_income"],
        period_info,
    );
    
    // Extract operating income
    income_stmt.operating_income = extract_field_value_for_period(
        gaap_facts,
        &gaap_mapping.income_statement_fields["operating_income"],
        period_info,
    );
    
    // Extract basic shares
    income_stmt.shares_basic = extract_field_value_for_period(
        gaap_facts,
        &gaap_mapping.income_statement_fields["shares_basic"],
        period_info,
    );
    
    // Extract diluted shares
    income_stmt.shares_diluted = extract_field_value_for_period(
        gaap_facts,
        &gaap_mapping.income_statement_fields["shares_diluted"],
        period_info,
    );
    
    Ok(income_stmt)
}

fn extract_balance_sheet_for_period(
    gaap_facts: &HashMap<String, EdgarConcept>,
    stock_id: i64,
    period_info: &PeriodInfo,
    gaap_mapping: &GaapFieldMapping,
) -> Result<BalanceSheetData> {
    let mut balance_sheet = BalanceSheetData {
        stock_id,
        period: period_info.period.clone(),
        year: period_info.year,
        end_date: period_info.end_date.clone(),
        total_assets: None,
        total_debt: None,
        total_equity: None,
        cash_and_equivalents: None,
        shares_outstanding: None,
    };
    
    // Extract total assets
    balance_sheet.total_assets = extract_field_value_for_period(
        gaap_facts,
        &gaap_mapping.balance_sheet_fields["total_assets"],
        period_info,
    );
    
    // Extract total debt
    balance_sheet.total_debt = extract_field_value_for_period(
        gaap_facts,
        &gaap_mapping.balance_sheet_fields["total_debt"],
        period_info,
    );
    
    // Extract total equity
    balance_sheet.total_equity = extract_field_value_for_period(
        gaap_facts,
        &gaap_mapping.balance_sheet_fields["total_equity"],
        period_info,
    );
    
    // Extract cash and equivalents
    balance_sheet.cash_and_equivalents = extract_field_value_for_period(
        gaap_facts,
        &gaap_mapping.balance_sheet_fields["cash_and_equivalents"],
        period_info,
    );
    
    // Extract shares outstanding
    balance_sheet.shares_outstanding = extract_field_value_for_period(
        gaap_facts,
        &gaap_mapping.balance_sheet_fields["shares_outstanding"],
        period_info,
    );
    
    Ok(balance_sheet)
}

fn extract_field_value_for_period(
    gaap_facts: &HashMap<String, EdgarConcept>,
    field_priorities: &[String],
    period_info: &PeriodInfo,
) -> Option<f64> {
    // Try each field in priority order
    for field_name in field_priorities {
        if let Some(concept) = gaap_facts.get(field_name) {
            if let Some(usd_values) = concept.units.get("USD") {
                // Find the value for this specific period
                for fact_value in usd_values {
                    if fact_value.fy == Some(period_info.year) && 
                       fact_value.fp.as_ref() == Some(&period_info.period) &&
                       fact_value.end == period_info.end_date {
                        return Some(fact_value.val);
                    }
                }
            }
        }
    }
    None
}

async fn insert_financial_data_to_db(db_pool: &SqlitePool, data: &ExtractedFinancialData) -> Result<()> {
    let mut tx = db_pool.begin().await?;
    
    // Insert income statements - adapt to existing schema
    for income_stmt in &data.income_statements {
        let period_type = match income_stmt.period.as_str() {
            "FY" => "Annual",
            "Q1" | "Q2" | "Q3" | "Q4" => "Quarterly",
            _ => "Quarterly", // Default to quarterly
        };
        
        let fiscal_period = if income_stmt.period == "FY" {
            None
        } else {
            Some(income_stmt.period.clone())
        };
        
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO income_statements 
            (stock_id, period_type, report_date, fiscal_year, fiscal_period, revenue, net_income, operating_income, shares_basic, shares_diluted, data_source)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'edgar')
            "#
        )
        .bind(income_stmt.stock_id)
        .bind(period_type)
        .bind(&income_stmt.end_date)
        .bind(income_stmt.year)
        .bind(fiscal_period)
        .bind(income_stmt.revenue)
        .bind(income_stmt.net_income)
        .bind(income_stmt.operating_income)
        .bind(income_stmt.shares_basic)
        .bind(income_stmt.shares_diluted)
        .execute(&mut *tx)
        .await?;
    }
    
    // Insert balance sheets - adapt to existing schema
    for balance_sheet in &data.balance_sheets {
        let period_type = match balance_sheet.period.as_str() {
            "FY" => "Annual",
            "Q1" | "Q2" | "Q3" | "Q4" => "Quarterly",
            _ => "Quarterly", // Default to quarterly
        };
        
        let fiscal_period = if balance_sheet.period == "FY" {
            None
        } else {
            Some(balance_sheet.period.clone())
        };
        
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO balance_sheets 
            (stock_id, period_type, report_date, fiscal_year, fiscal_period, total_assets, total_debt, total_equity, cash_and_equivalents, shares_outstanding, data_source)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'edgar')
            "#
        )
        .bind(balance_sheet.stock_id)
        .bind(period_type)
        .bind(&balance_sheet.end_date)
        .bind(balance_sheet.year)
        .bind(fiscal_period)
        .bind(balance_sheet.total_assets)
        .bind(balance_sheet.total_debt)
        .bind(balance_sheet.total_equity)
        .bind(balance_sheet.cash_and_equivalents)
        .bind(balance_sheet.shares_outstanding)
        .execute(&mut *tx)
        .await?;
    }
    
    tx.commit().await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    info!("🚀 Concurrent EDGAR Data Extraction");
    info!("===================================");
    
    let config = ExtractionConfig {
        max_workers: cli.workers,
        batch_size: cli.batch_size,
        max_file_readers: cli.max_file_readers,
        progress_interval: Duration::from_secs(cli.progress_interval),
    };
    
    info!("Configuration:");
    info!("   Workers: {}", config.max_workers);
    info!("   Batch size: {}", config.batch_size);
    info!("   Max file readers: {}", config.max_file_readers);
    info!("   Progress interval: {}s", cli.progress_interval);
    
    let mut extractor = ConcurrentEdgarExtractor::new(config).await?;
    
    // Populate work queue based on command/filters
    match cli.command {
        Some(Commands::Test) | None if cli.test_limit.is_some() => {
            extractor.populate_work_queue(cli.test_limit, cli.symbols.as_deref()).await?;
        }
        _ => {
            extractor.populate_work_queue(cli.test_limit, cli.symbols.as_deref()).await?;
        }
    }
    
    // Start workers and monitoring
    extractor.spawn_workers().await?;
    extractor.start_progress_monitor().await;
    
    // Wait for completion
    extractor.wait_for_completion().await?;
    
    info!("✅ Concurrent EDGAR extraction completed successfully!");
    
    Ok(())
}