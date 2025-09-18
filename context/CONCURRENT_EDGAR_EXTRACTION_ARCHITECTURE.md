# Concurrent EDGAR Data Extraction - Architecture & Implementation Plan

## 🎯 Objectives

### Performance Goals
- **Concurrency**: Process 10+ EDGAR files simultaneously 
- **Database Efficiency**: Utilize existing 10-connection pool
- **Throughput**: Target 100+ companies/minute processing speed
- **Resource Management**: Optimize memory and I/O usage

### Data Management Goals
- **Single Source of Truth**: Store all CIK mappings in database only
- **S&P 500 Focus**: Extract data only for S&P 500 companies (503 stocks)
- **Data Integrity**: Ensure concurrent writes don't corrupt data
- **Progress Tracking**: Real-time progress monitoring across threads

## 🏗️ Concurrent Architecture Design

### High-Level Architecture
```
┌─────────────────────────────────────────────────────────────────────────┐
│                 Concurrent EDGAR Extraction System                      │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌─────────────────┐    ┌──────────────────┐    ┌──────────────────┐   │
│  │   Database      │    │   Work Queue     │    │   Thread Pool    │   │
│  │   CIK Mappings  │───▶│   Manager        │───▶│   (10 workers)   │   │
│  │                 │    │                  │    │                  │   │
│  └─────────────────┘    └──────────────────┘    └──────────────────┘   │
│           │                        │                        │           │
│           ▼                        ▼                        ▼           │
│  ┌─────────────────┐    ┌──────────────────┐    ┌──────────────────┐   │
│  │   Connection    │    │   Task           │    │   File Reader    │   │
│  │   Pool (10)     │◀──▶│   Distributor    │◀──▶│   Pool           │   │
│  │                 │    │                  │    │                  │   │
│  └─────────────────┘    └──────────────────┘    └──────────────────┘   │
│           │                        │                        │           │
│           ▼                        ▼                        ▼           │
│  ┌─────────────────┐    ┌──────────────────┐    ┌──────────────────┐   │
│  │   Batch Writer  │    │   Progress       │    │   Error          │   │
│  │   Coordinator   │    │   Aggregator     │    │   Handler        │   │
│  │                 │    │                  │    │                  │   │
│  └─────────────────┘    └──────────────────┘    └──────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

### Core Components

#### 1. Database CIK Mapping Strategy
```sql
-- Single source of truth for CIK mappings
CREATE TABLE cik_mappings_sp500 (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    cik TEXT NOT NULL UNIQUE,
    stock_id INTEGER NOT NULL,
    symbol TEXT NOT NULL,
    company_name TEXT NOT NULL,
    edgar_file_path TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    INDEX idx_cik (cik),
    INDEX idx_symbol (symbol)
);
```

#### 2. Work Queue Manager
```rust
struct WorkQueueManager {
    pending_tasks: Arc<Mutex<VecDeque<ExtractionTask>>>,
    completed_tasks: Arc<AtomicUsize>,
    failed_tasks: Arc<AtomicUsize>,
    total_tasks: usize,
}

#[derive(Debug, Clone)]
struct ExtractionTask {
    cik: String,
    symbol: String,
    stock_id: i64,
    edgar_file_path: PathBuf,
    priority: u8, // Higher for major companies
}
```

#### 3. Concurrent Worker Pool
```rust
struct ConcurrentEdgarExtractor {
    db_pool: Arc<SqlitePool>,
    work_queue: Arc<WorkQueueManager>,
    worker_handles: Vec<JoinHandle<Result<()>>>,
    progress_tracker: Arc<Mutex<ExtractionProgress>>,
    semaphore: Arc<Semaphore>, // Limit concurrent file I/O
}

impl ConcurrentEdgarExtractor {
    async fn new(concurrency_level: usize) -> Result<Self> {
        // Initialize with configurable worker count
    }
    
    async fn spawn_workers(&mut self) -> Result<()> {
        for worker_id in 0..self.concurrency_level {
            let handle = self.spawn_worker(worker_id).await?;
            self.worker_handles.push(handle);
        }
        Ok(())
    }
}
```

#### 4. Worker Thread Implementation
```rust
async fn worker_thread(
    worker_id: usize,
    db_pool: Arc<SqlitePool>,
    work_queue: Arc<WorkQueueManager>,
    progress_tracker: Arc<Mutex<ExtractionProgress>>,
    semaphore: Arc<Semaphore>,
) -> Result<()> {
    loop {
        // Get next task from queue
        let task = match work_queue.get_next_task().await {
            Some(task) => task,
            None => break, // No more work
        };
        
        // Acquire semaphore for file I/O
        let _permit = semaphore.acquire().await?;
        
        // Process EDGAR file
        match process_edgar_file(&task, &db_pool).await {
            Ok(extracted_data) => {
                // Insert to database
                insert_financial_data(&db_pool, extracted_data).await?;
                work_queue.mark_completed(&task).await;
            }
            Err(e) => {
                work_queue.mark_failed(&task, e).await;
            }
        }
        
        // Update progress
        update_progress(&progress_tracker, worker_id).await;
    }
    
    Ok(())
}
```

## 📊 Database Strategy Redesign

### 1. Populate CIK Mappings Table
```rust
async fn populate_cik_mappings_from_sec_file(db_pool: &SqlitePool) -> Result<()> {
    info!("Populating CIK mappings table from SEC company tickers...");
    
    // Load S&P 500 stocks
    let sp500_stocks = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, symbol, company_name FROM stocks WHERE is_sp500 = 1"
    )
    .fetch_all(db_pool)
    .await?;
    
    // Create lookup map
    let mut symbol_to_stock: HashMap<String, (i64, String)> = HashMap::new();
    for (id, symbol, name) in sp500_stocks {
        symbol_to_stock.insert(symbol.clone(), (id, name));
    }
    
    // Load SEC company tickers
    let sec_tickers = load_sec_company_tickers().await?;
    
    // Clear existing mappings
    sqlx::query("DELETE FROM cik_mappings_sp500").execute(db_pool).await?;
    
    let mut batch_inserts = Vec::new();
    
    for entry in sec_tickers.data {
        if let (Some(cik), Some(ticker)) = (entry[0].as_i64(), entry[2].as_str()) {
            if let Some((stock_id, company_name)) = symbol_to_stock.get(ticker) {
                let edgar_file_path = format!("/Users/yksoni/code/misc/rust-stocks/edgar_data/companyfacts/CIK{:010}.json", cik);
                
                // Verify file exists
                if std::path::Path::new(&edgar_file_path).exists() {
                    batch_inserts.push((
                        cik.to_string(),
                        *stock_id,
                        ticker.to_string(),
                        company_name.clone(),
                        edgar_file_path,
                    ));
                }
            }
        }
    }
    
    // Batch insert all mappings
    let mut tx = db_pool.begin().await?;
    
    for (cik, stock_id, symbol, company_name, file_path) in batch_inserts {
        sqlx::query(
            "INSERT INTO cik_mappings_sp500 (cik, stock_id, symbol, company_name, edgar_file_path) 
             VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&cik)
        .bind(stock_id)
        .bind(&symbol)
        .bind(&company_name)
        .bind(&file_path)
        .execute(&mut *tx)
        .await?;
    }
    
    tx.commit().await?;
    
    let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM cik_mappings_sp500")
        .fetch_one(db_pool)
        .await?;
    
    info!("Populated {} CIK mappings for S&P 500 companies", count);
    Ok(())
}
```

### 2. Work Queue Population
```rust
async fn populate_work_queue(db_pool: &SqlitePool) -> Result<WorkQueueManager> {
    let mappings = sqlx::query_as::<_, (String, String, i64, String)>(
        "SELECT cik, symbol, stock_id, edgar_file_path FROM cik_mappings_sp500 ORDER BY symbol"
    )
    .fetch_all(db_pool)
    .await?;
    
    let mut tasks = VecDeque::new();
    
    for (cik, symbol, stock_id, file_path) in mappings {
        // Assign priority (higher for major companies)
        let priority = match symbol.as_str() {
            "AAPL" | "MSFT" | "GOOGL" | "AMZN" | "NVDA" => 10,
            _ => 5,
        };
        
        tasks.push_back(ExtractionTask {
            cik,
            symbol,
            stock_id,
            edgar_file_path: PathBuf::from(file_path),
            priority,
        });
    }
    
    // Sort by priority (high to low)
    let mut tasks_vec: Vec<_> = tasks.into();
    tasks_vec.sort_by(|a, b| b.priority.cmp(&a.priority));
    
    Ok(WorkQueueManager {
        pending_tasks: Arc::new(Mutex::new(tasks_vec.into())),
        completed_tasks: Arc::new(AtomicUsize::new(0)),
        failed_tasks: Arc::new(AtomicUsize::new(0)),
        total_tasks: tasks_vec.len(),
    })
}
```

## 🔄 Concurrency Implementation

### 1. File I/O Optimization
```rust
// Limit concurrent file reads to prevent I/O saturation
const MAX_CONCURRENT_FILE_READS: usize = 10;
const MAX_CONCURRENT_DB_WRITES: usize = 10;

struct ResourceManager {
    file_semaphore: Arc<Semaphore>,
    db_semaphore: Arc<Semaphore>,
}

impl ResourceManager {
    fn new() -> Self {
        Self {
            file_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_FILE_READS)),
            db_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_DB_WRITES)),
        }
    }
}
```

### 2. Database Batch Optimization
```rust
async fn batch_insert_financial_data(
    db_pool: &SqlitePool,
    extracted_data: Vec<ExtractedFinancialData>,
) -> Result<()> {
    // Use single transaction for all related data
    let mut tx = db_pool.begin().await?;
    
    // Prepare batch statements
    let income_stmt_query = "INSERT OR REPLACE INTO income_statements 
        (stock_id, period, year, end_date, revenue, net_income, operating_income, shares_basic, shares_diluted)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)";
        
    let balance_sheet_query = "INSERT OR REPLACE INTO balance_sheets 
        (stock_id, period, year, end_date, total_assets, total_debt, total_equity, cash_and_equivalents, shares_outstanding)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)";
    
    // Batch insert income statements
    for data in &extracted_data {
        for income_stmt in &data.income_statements {
            sqlx::query(income_stmt_query)
                .bind(income_stmt.stock_id)
                .bind(&income_stmt.period)
                .bind(income_stmt.year)
                .bind(&income_stmt.end_date)
                .bind(income_stmt.revenue)
                .bind(income_stmt.net_income)
                .bind(income_stmt.operating_income)
                .bind(income_stmt.shares_basic)
                .bind(income_stmt.shares_diluted)
                .execute(&mut *tx)
                .await?;
        }
        
        // Batch insert balance sheets
        for balance_sheet in &data.balance_sheets {
            sqlx::query(balance_sheet_query)
                .bind(balance_sheet.stock_id)
                .bind(&balance_sheet.period)
                .bind(balance_sheet.year)
                .bind(&balance_sheet.end_date)
                .bind(balance_sheet.total_assets)
                .bind(balance_sheet.total_debt)
                .bind(balance_sheet.total_equity)
                .bind(balance_sheet.cash_and_equivalents)
                .bind(balance_sheet.shares_outstanding)
                .execute(&mut *tx)
                .await?;
        }
    }
    
    tx.commit().await?;
    Ok(())
}
```

### 3. Progress Monitoring
```rust
#[derive(Debug, Clone)]
struct ConcurrentProgress {
    total_companies: usize,
    completed_companies: AtomicUsize,
    failed_companies: AtomicUsize,
    active_workers: AtomicUsize,
    start_time: DateTime<Utc>,
    worker_stats: Arc<Mutex<HashMap<usize, WorkerStats>>>,
}

#[derive(Debug, Clone)]
struct WorkerStats {
    worker_id: usize,
    processed_count: usize,
    current_company: Option<String>,
    last_update: DateTime<Utc>,
}

impl ConcurrentProgress {
    async fn display_progress(&self) {
        let completed = self.completed_companies.load(Ordering::Relaxed);
        let failed = self.failed_companies.load(Ordering::Relaxed);
        let active = self.active_workers.load(Ordering::Relaxed);
        let elapsed = Utc::now().signed_duration_since(self.start_time);
        
        let rate = if elapsed.num_seconds() > 0 {
            completed as f64 / elapsed.num_seconds() as f64 * 60.0 // per minute
        } else {
            0.0
        };
        
        println!("🏗️ Concurrent EDGAR Extraction Progress");
        println!("======================================");
        println!("Total: {} | Completed: {} | Failed: {} | Active Workers: {}", 
                 self.total_companies, completed, failed, active);
        println!("Processing Rate: {:.1} companies/minute", rate);
        println!("Elapsed: {}m {}s", elapsed.num_minutes(), elapsed.num_seconds() % 60);
        
        // Show worker details
        let worker_stats = self.worker_stats.lock().await;
        for (worker_id, stats) in worker_stats.iter() {
            println!("  Worker {}: {} processed | Current: {}", 
                     worker_id, 
                     stats.processed_count,
                     stats.current_company.as_deref().unwrap_or("idle"));
        }
    }
}
```

## 🎛️ Configuration & Tuning

### Performance Parameters
```rust
struct ExtractionConfig {
    // Concurrency settings
    max_workers: usize,              // Default: 10
    max_file_readers: usize,         // Default: 10  
    max_db_connections: usize,       // Default: 10
    
    // Batch processing
    batch_size: usize,               // Default: 50 companies
    transaction_timeout: Duration,   // Default: 30s
    
    // Resource limits
    max_memory_mb: usize,           // Default: 2048MB
    file_read_timeout: Duration,    // Default: 30s
    
    // Progress reporting
    progress_interval: Duration,    // Default: 10s
    checkpoint_interval: usize,     // Default: 100 companies
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            max_workers: 10,
            max_file_readers: 10,
            max_db_connections: 10,
            batch_size: 50,
            transaction_timeout: Duration::from_secs(30),
            max_memory_mb: 2048,
            file_read_timeout: Duration::from_secs(30),
            progress_interval: Duration::from_secs(10),
            checkpoint_interval: 100,
        }
    }
}
```

### CLI Interface
```bash
# Concurrent extraction with custom settings
cargo run --bin import-edgar-data -- extract \
    --workers 10 \
    --batch-size 50 \
    --timeout 30 \
    --progress-interval 10

# Resume interrupted extraction
cargo run --bin import-edgar-data -- resume

# Test with subset
cargo run --bin import-edgar-data -- extract \
    --symbols AAPL,MSFT,GOOGL,AMZN,NVDA \
    --workers 5

# Performance monitoring
cargo run --bin import-edgar-data -- status --watch
```

## 📈 Expected Performance

### Throughput Targets
- **Processing Rate**: 100+ companies/minute
- **Completion Time**: ~5 minutes for 500 S&P 500 companies
- **Memory Usage**: <2GB peak
- **Database Growth**: ~500MB for all financial statements

### Scalability Factors
- **File I/O**: Limited by storage read speed (~500 MB/s SSD)
- **JSON Parsing**: CPU-bound, benefits from multiple cores
- **Database Writes**: Limited by SQLite write throughput
- **Memory**: Each worker uses ~50MB for JSON processing

### Monitoring Metrics
- **Worker Utilization**: % of time workers are active
- **I/O Wait Time**: Time spent waiting for file reads
- **Database Queue Depth**: Pending database operations
- **Error Rate**: % of failed extractions
- **Memory Growth**: Track memory leaks in long runs

## 🔄 Implementation Phases

### Phase 1: Database Migration (30 minutes)
1. Create `cik_mappings_sp500` table
2. Populate from SEC company_tickers_exchange.json
3. Verify file path existence for all mappings
4. Create performance indexes

### Phase 2: Concurrent Infrastructure (2 hours)
1. Implement WorkQueueManager
2. Create worker thread pool
3. Add resource management (semaphores)
4. Implement progress tracking

### Phase 3: Worker Implementation (2 hours)
1. Refactor existing extraction logic for concurrency
2. Add batch database operations
3. Implement error handling and retry logic
4. Add memory management

### Phase 4: Testing & Validation (1 hour)
1. Test with 10-company subset
2. Validate data integrity
3. Measure performance metrics
4. Tune configuration parameters

### Phase 5: Production Run (10 minutes)
1. Execute full S&P 500 extraction
2. Monitor progress and performance
3. Validate against known benchmarks
4. Generate completion report

## 🎯 Success Criteria

### Performance Goals
- ✅ **Processing Speed**: >100 companies/minute
- ✅ **Completion Time**: <10 minutes for 500 companies
- ✅ **Resource Usage**: <2GB memory, <50% CPU
- ✅ **Reliability**: <1% error rate

### Data Quality Goals
- ✅ **Coverage**: 500+ S&P 500 companies processed
- ✅ **Accuracy**: Match Apple Q3 2024 benchmark
- ✅ **Completeness**: >90% of core fields populated
- ✅ **Integrity**: No data corruption from concurrency

This architecture provides a robust, scalable foundation for concurrent EDGAR data extraction while maintaining data integrity and optimal performance.