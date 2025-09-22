use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};
use std::collections::HashMap;
use tokio::process::Command;
use tokio::time::sleep;
use std::time::Duration as StdDuration;
use uuid::Uuid;

use crate::tools::data_freshness_checker::{DataStatusReader, SystemFreshnessReport};
use crate::tools::ratio_calculator;
use crate::tools::date_range_calculator::DateRangeCalculator;
use crate::tools::edgar_extractor::EdgarDataExtractor;
use crate::api::schwab_client::SchwabClient;
use crate::api::StockDataProvider;
use crate::models::Config;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, clap::ValueEnum)]
pub enum RefreshMode {
    /// Market data from Schwab: prices, shares, market cap (~15min)
    Market,
    /// All EDGAR financial data: income, balance, cash flow (~90min)
    Financials,
    /// All calculated ratios: P/E, P/S, Piotroski, O'Shaughnessy (~10min, requires Market + Financials)
    Ratios,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshRequest {
    pub mode: RefreshMode,
    pub force_sources: Vec<String>,
    pub initiated_by: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshResult {
    pub session_id: String,
    pub success: bool,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_seconds: Option<i64>,
    pub sources_refreshed: Vec<String>,
    pub sources_failed: Vec<String>,
    pub total_records_processed: i64,
    pub error_message: Option<String>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshStep {
    pub name: String,
    pub data_source: String,
    pub estimated_duration_minutes: i32,
    pub command: String,
    pub dependencies: Vec<String>,
    pub priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshProgress {
    pub session_id: String,
    pub operation_type: String,
    pub start_time: DateTime<Utc>,
    pub total_steps: i32,
    pub completed_steps: i32,
    pub current_step_name: String,
    pub current_step_progress: f64,
    pub overall_progress_percent: f64,
    pub estimated_completion: Option<DateTime<Utc>>,
    pub status: String,
    pub error_details: Option<String>,
}

pub struct DataRefreshManager {
    pool: SqlitePool,
    status_reader: DataStatusReader,
    #[allow(dead_code)]
    date_calculator: DateRangeCalculator,
    edgar_extractor: EdgarDataExtractor,
    refresh_steps: HashMap<RefreshMode, Vec<RefreshStep>>,
}

impl DataRefreshManager {
    pub async fn new(pool: SqlitePool) -> Result<Self> {
        // Load .env file first to ensure environment variables are available
        dotenvy::dotenv().ok();

        let status_reader = DataStatusReader::new(pool.clone());
        let date_calculator = DateRangeCalculator::new();
        let edgar_data_path = std::env::var("EDGAR_DATA_PATH")
            .unwrap_or_else(|_| "edgar_data".to_string());
        let edgar_extractor = EdgarDataExtractor::new(&edgar_data_path, pool.clone());
        let refresh_steps = Self::define_refresh_steps();

        Ok(Self {
            pool,
            status_reader,
            date_calculator,
            edgar_extractor,
            refresh_steps,
        })
    }

    /// Execute a data refresh operation based on the request
    pub async fn execute_refresh(&self, request: RefreshRequest) -> Result<RefreshResult> {
        let session_id = request.session_id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
        let _start_time = Utc::now();

        println!("🚀 Starting data refresh session: {}", session_id);
        println!("🎯 Mode: {:?} | Initiated by: {}", request.mode, request.initiated_by);

        // Create progress tracking record
        self.create_progress_record(&session_id, &request).await?;

        let result = match self.execute_refresh_internal(session_id.clone(), request.clone()).await {
            Ok(result) => {
                self.mark_progress_complete(&session_id, true, None).await?;
                result
            }
            Err(e) => {
                self.mark_progress_complete(&session_id, false, Some(e.to_string())).await?;
                return Err(e);
            }
        };

        Ok(result)
    }

    async fn execute_refresh_internal(&self, session_id: String, request: RefreshRequest) -> Result<RefreshResult> {
        let start_time = Utc::now();
        let mut sources_refreshed = Vec::new();
        let mut sources_failed = Vec::new();
        let mut total_records_processed = 0i64;

        // 1. Check current freshness status
        println!("🔍 Checking current data freshness...");
        let freshness_report = self.status_reader.check_system_freshness().await?;
        self.update_progress(&session_id, 1, "Checking data freshness", 100.0).await?;

        // 2. Determine what needs to be refreshed
        let refresh_plan = self.create_refresh_plan(&request, &freshness_report).await?;
        println!("📋 Refresh plan: {} steps identified", refresh_plan.len());

        if refresh_plan.is_empty() {
            println!("✅ All data is current, no refresh needed");
            return Ok(RefreshResult {
                session_id,
                success: true,
                start_time,
                end_time: Some(Utc::now()),
                duration_seconds: Some(0),
                sources_refreshed: vec!["none (all current)".to_string()],
                sources_failed: Vec::new(),
                total_records_processed: 0,
                error_message: None,
                recommendations: vec!["All data sources are current".to_string()],
            });
        }

        // 3. Execute refresh steps in dependency order
        let total_steps = refresh_plan.len() as i32 + 2; // +2 for start/finish steps
        self.update_progress_total_steps(&session_id, total_steps).await?;

        for (step_index, step) in refresh_plan.iter().enumerate() {
            let step_number = step_index as i32 + 2; // +1 for zero-index, +1 for initial check
            println!("🔄 Step {}/{}: {}", step_number, total_steps, step.name);

            self.update_progress(&session_id, step_number, &step.name, 0.0).await?;

            match self.execute_refresh_step(step, &session_id).await {
                Ok(records) => {
                    sources_refreshed.push(step.data_source.clone());
                    total_records_processed += records;
                    self.update_refresh_status(&step.data_source, true, Some(records), None).await?;
                    self.update_progress(&session_id, step_number, &step.name, 100.0).await?;
                    println!("✅ {} completed successfully ({} records)", step.name, records);
                }
                Err(e) => {
                    sources_failed.push(step.data_source.clone());
                    self.update_refresh_status(&step.data_source, false, None, Some(e.to_string())).await?;
                    println!("❌ {} failed: {}", step.name, e);

                    // For critical steps, abort the entire refresh
                    if step.priority <= 2 {
                        return Err(anyhow!("Critical refresh step failed: {}", e));
                    }
                    // For non-critical steps, continue but log the failure
                }
            }
        }

        // 4. Final verification and cleanup
        self.update_progress(&session_id, total_steps, "Finalizing refresh", 100.0).await?;
        let final_report = self.status_reader.check_system_freshness().await?;

        let end_time = Utc::now();
        let duration_seconds = end_time.signed_duration_since(start_time).num_seconds();

        println!("🎉 Refresh session completed in {} seconds", duration_seconds);
        println!("✅ Refreshed: {}", sources_refreshed.join(", "));
        if !sources_failed.is_empty() {
            println!("❌ Failed: {}", sources_failed.join(", "));
        }

        Ok(RefreshResult {
            session_id,
            success: sources_failed.is_empty(),
            start_time,
            end_time: Some(end_time),
            duration_seconds: Some(duration_seconds),
            sources_refreshed,
            sources_failed,
            total_records_processed,
            error_message: None,
            recommendations: self.generate_post_refresh_recommendations(&final_report),
        })
    }

    /// Create a refresh plan based on mode and current freshness
    async fn create_refresh_plan(&self, request: &RefreshRequest, freshness_report: &SystemFreshnessReport) -> Result<Vec<RefreshStep>> {
        let available_steps = self.refresh_steps.get(&request.mode)
            .ok_or_else(|| anyhow!("Unknown refresh mode: {:?}", request.mode))?;

        let mut plan = Vec::new();

        // If force_sources is specified, only refresh those
        if !request.force_sources.is_empty() {
            for force_source in &request.force_sources {
                if let Some(step) = available_steps.iter().find(|s| s.data_source == *force_source) {
                    plan.push(step.clone());
                }
            }
        } else {
            // Otherwise, determine what needs refresh based on staleness
            for step in available_steps {
                // Check if the data source needs refresh based on the semantic fields
                let needs_refresh = match step.data_source.as_str() {
                    "daily_prices" => freshness_report.market_data.status.needs_refresh(),
                    "financial_statements" => freshness_report.financial_data.status.needs_refresh(),
                    "ps_evs_ratios" => freshness_report.calculated_ratios.status.needs_refresh(),
                    _ => true, // Unknown data source, refresh it
                };
                
                if needs_refresh {
                    plan.push(step.clone());
                }
            }
        }

        // Sort by priority and dependencies
        plan.sort_by_key(|step| step.priority);

        Ok(plan)
    }

    /// Execute a single refresh step
    async fn execute_refresh_step(&self, step: &RefreshStep, session_id: &str) -> Result<i64> {
        let start_time = Utc::now();

        // Record the start of this refresh
        self.record_refresh_start(&step.data_source).await?;

        let records_processed = match step.data_source.as_str() {
            "daily_prices" => self.refresh_market_internal(session_id).await?,
            "financial_statements" => self.refresh_financials_internal(session_id).await?,
            "ps_evs_ratios" => self.refresh_ratios_internal(session_id).await?,
            _ => return Err(anyhow!("Unknown data source: {}", step.data_source)),
        };

        let end_time = Utc::now();
        let duration_seconds = end_time.signed_duration_since(start_time).num_seconds();

        // Record the completion
        self.record_refresh_complete(&step.data_source, records_processed, duration_seconds as i32).await?;

        Ok(records_processed)
    }

    // ========================================
    // CLEAN INTERNAL FUNCTIONS (No external cargo calls)
    // ========================================

    /// Refresh market data from Schwab (prices, shares, market cap)
    async fn refresh_market_internal(&self, _session_id: &str) -> Result<i64> {
        println!("💰 Refreshing market data from Schwab...");

        // Load configuration and create Schwab client
        let config = Config::from_env()?;
        let _schwab_client = SchwabClient::new(&config)?;

        // Get today's date for end date
        let end_date = chrono::Local::now().naive_local().date();

        println!("📅 Importing market data up to {}", end_date);

        // Get only S&P 500 stocks that need price updates
        let stocks_query = r#"
            SELECT s.id, s.symbol
            FROM stocks s
            INNER JOIN sp500_symbols sp ON s.symbol = sp.symbol
            WHERE s.status = 'active'
            ORDER BY s.symbol
        "#;
        let stocks = sqlx::query_as::<_, (i64, String)>(stocks_query)
            .fetch_all(&self.pool)
            .await?;

        println!("📊 Found {} S&P 500 stocks to update", stocks.len());

        let total_stocks = stocks.len();

        // Process stocks concurrently with semaphore limiting
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(10)); // Max 10 concurrent
        let mut tasks = Vec::new();

        for (stock_id, symbol) in stocks {
            let permit = semaphore.clone();
            let pool = self.pool.clone();
            let config = config.clone();

            let task = tokio::spawn(async move {
                let _permit = permit.acquire().await.unwrap();

                // Create client inside task since SchwabClient doesn't implement Clone
                let client = match SchwabClient::new(&config) {
                    Ok(c) => c,
                    Err(e) => return Err(anyhow!("Failed to create client for {}: {}", symbol, e)),
                };

                // Get the latest date for this symbol to determine where to start
                let latest_date_query = "SELECT MAX(date) as latest FROM daily_prices WHERE stock_id = ?";
                let latest_result = sqlx::query(latest_date_query)
                    .bind(stock_id)
                    .fetch_optional(&pool)
                    .await;

                let start_update_date = if let Ok(Some(row)) = latest_result {
                    if let Ok(latest_str) = row.try_get::<String, _>("latest") {
                        if let Ok(latest_date) = chrono::NaiveDate::parse_from_str(&latest_str, "%Y-%m-%d") {
                            latest_date.succ_opt().unwrap_or(end_date)
                        } else {
                            chrono::NaiveDate::from_ymd_opt(2015, 1, 1).unwrap()
                        }
                    } else {
                        chrono::NaiveDate::from_ymd_opt(2015, 1, 1).unwrap()
                    }
                } else {
                    chrono::NaiveDate::from_ymd_opt(2015, 1, 1).unwrap()
                };

                // Skip if already up to date
                if start_update_date > end_date {
                    return Ok((symbol, 0));
                }

                // Fetch price data
                match client.get_price_history(&symbol, start_update_date, end_date).await {
                    Ok(candles) => {
                        if !candles.is_empty() {
                            let mut records_inserted = 0;
                            // Insert the candles into database
                            for candle in &candles {
                                let insert_query = r#"
                                    INSERT OR REPLACE INTO daily_prices
                                    (stock_id, date, open_price, high_price, low_price, close_price, volume, created_at)
                                    VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'))
                                "#;

                                // Convert Unix timestamp to date string
                                let datetime = DateTime::from_timestamp(candle.datetime / 1000, 0)
                                    .unwrap_or_else(|| Utc::now());
                                let date_str = datetime.format("%Y-%m-%d").to_string();

                                if let Ok(_) = sqlx::query(insert_query)
                                    .bind(stock_id)
                                    .bind(date_str)
                                    .bind(candle.open)
                                    .bind(candle.high)
                                    .bind(candle.low)
                                    .bind(candle.close)
                                    .bind(candle.volume)
                                    .execute(&pool)
                                    .await {
                                    records_inserted += 1;
                                }
                            }
                            Ok((symbol, records_inserted))
                        } else {
                            Ok((symbol, 0))
                        }
                    }
                    Err(e) => {
                        Err(anyhow!("Failed to fetch {}: {}", symbol, e))
                    }
                }
            });

            tasks.push(task);
        }

        // Wait for all tasks to complete
        println!("🚀 Processing {} stocks concurrently (max 10 parallel)...", tasks.len());

        let mut total_records = 0;
        let mut updated_symbols = 0;

        for (i, task) in tasks.into_iter().enumerate() {
            match task.await {
                Ok(Ok((symbol, records))) => {
                    total_records += records;
                    updated_symbols += 1;
                    if records > 0 {
                        println!("✅ {} - {} new price records", symbol, records);
                    }
                }
                Ok(Err(e)) => {
                    println!("⚠️ Task failed: {}", e);
                }
                Err(e) => {
                    println!("⚠️ Task {} panicked: {}", i, e);
                }
            }

            // Progress update every 50 stocks
            if updated_symbols % 50 == 0 {
                println!("📊 Progress: {}/{} stocks processed, {} total records",
                         updated_symbols, total_stocks, total_records);
            }
        }

        println!("✅ S&P 500 market data refresh completed - {} symbols, {} records", updated_symbols, total_records);
        Ok(total_records as i64)
    }

    /// Refresh all EDGAR financial data (income, balance, cash flow) - PARALLEL VERSION
    async fn refresh_financials_internal(&self, _session_id: &str) -> Result<i64> {
        println!("📈 Refreshing EDGAR financial data (parallel processing)...");

        // Get S&P 500 CIKs for processing
        let ciks_query = r#"
            SELECT cm.cik, cm.stock_id, s.symbol, s.company_name
            FROM cik_mappings_sp500 cm
            INNER JOIN stocks s ON s.symbol = cm.symbol
            WHERE s.status = 'active'
            ORDER BY s.symbol
        "#;

        let cik_data = sqlx::query_as::<_, (String, i32, String, String)>(ciks_query)
            .fetch_all(&self.pool)
            .await?;

        println!("📊 Found {} S&P 500 companies for EDGAR extraction", cik_data.len());

        // Parallel processing with semaphore
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(20)); // Max 20 concurrent
        let edgar_data_path = std::env::var("EDGAR_DATA_PATH")
            .unwrap_or_else(|_| "edgar_data".to_string());

        let mut tasks = Vec::new();
        let total_companies = cik_data.len();

        for (cik_str, stock_id, symbol, company_name) in cik_data {
            let permit = semaphore.clone();
            let pool = self.pool.clone();
            let edgar_path = edgar_data_path.clone();

            let task = tokio::spawn(async move {
                let _permit = permit.acquire().await.unwrap();

                // Parse CIK string to i32
                let cik = match cik_str.parse::<i32>() {
                    Ok(c) => c,
                    Err(e) => return Err(anyhow!("Invalid CIK format '{}': {}", cik_str, e)),
                };

                // Create extractor instance for this task
                let extractor = EdgarDataExtractor::new(&edgar_path, pool);

                // Extract and store financial data
                match extractor.extract_company_data(cik).await {
                    Ok(financial_data) => {
                        match extractor.store_financial_data(stock_id, &financial_data).await {
                            Ok(records_stored) => {
                                if records_stored > 0 {
                                    println!("✅ {} ({}) - {} financial records", symbol, company_name, records_stored);
                                }
                                Ok((symbol, records_stored))
                            }
                            Err(e) => {
                                println!("⚠️ {} - Failed to store: {}", symbol, e);
                                Err(anyhow!("Store failed for {}: {}", symbol, e))
                            }
                        }
                    }
                    Err(e) => {
                        println!("⚠️ {} - EDGAR extraction failed: {}", symbol, e);
                        Err(anyhow!("Extract failed for {}: {}", symbol, e))
                    }
                }
            });
            tasks.push(task);
        }

        // Wait for all tasks to complete
        println!("🚀 Processing {} companies concurrently (max 20 parallel)...", tasks.len());

        let mut total_records = 0;
        let mut processed_companies = 0;

        for (i, task) in tasks.into_iter().enumerate() {
            match task.await {
                Ok(Ok((_symbol, records))) => {
                    total_records += records as i64;
                    processed_companies += 1;
                }
                Ok(Err(e)) => {
                    println!("⚠️ Task failed: {}", e);
                }
                Err(e) => {
                    println!("⚠️ Task {} panicked: {}", i, e);
                }
            }

            // Progress update every 50 companies
            if processed_companies % 50 == 0 {
                println!("📊 Progress: {}/{} companies processed, {} total records",
                         processed_companies, total_companies, total_records);
            }
        }

        println!("✅ S&P 500 EDGAR financial data refresh completed - {} companies, {} records",
                processed_companies, total_records);
        Ok(total_records)
    }

    /// Calculate all ratios and metrics for all algorithms
    async fn refresh_ratios_internal(&self, _session_id: &str) -> Result<i64> {
        println!("📁 Calculating all ratios and metrics...");

        // Check prerequisites
        println!("🔍 Checking prerequisites: market data + financial data");

        // Verify market data is current
        let market_check = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM daily_prices WHERE date >= date('now', '-7 days')"
        ).fetch_one(&self.pool).await?;

        if market_check == 0 {
            return Err(anyhow!("Market data required but not current. Run 'market' refresh first."));
        }

        // Verify financial data exists
        let financial_check = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM income_statements WHERE period_type = 'TTM'"
        ).fetch_one(&self.pool).await?;

        if financial_check == 0 {
            return Err(anyhow!("Financial data required but missing. Run 'financials' refresh first."));
        }

        println!("✅ Prerequisites satisfied - calculating ratios...");

        // TODO: Implement comprehensive ratio calculation here
        // For now, use existing ratio calculation calls
        let mut total_records = 0;

        // Calculate P/E ratios
        let pe_records = self.calculate_pe_ratios_internal().await?;
        total_records += pe_records;

        // Calculate P/S and EV/S ratios
        let ps_records = self.calculate_ps_evs_ratios_internal().await?;
        total_records += ps_records;

        println!("✅ All ratio calculations completed - {} total records", total_records);
        Ok(total_records)
    }

    /// Calculate P/E ratios (internal helper) - PARALLEL VERSION
    async fn calculate_pe_ratios_internal(&self) -> Result<i64> {
        println!("📊 Calculating P/E ratios (parallel processing)...");

        // Single phase: Calculate P/E ratios directly from income_statements - parallelized
        println!("📊 PHASE 1: P/E Ratio Calculation (parallel)");
        let pe_count = self.calculate_eps_parallel().await?; // This now calculates P/E ratios directly
        println!("✅ Phase 1 Complete: {} P/E ratios calculated", pe_count);

        // Phase 2: Refresh cache table
        println!("🔄 PHASE 2: Refresh cache table");
        let _ = sqlx::query("DROP TABLE IF EXISTS sp500_pe_cache").execute(&self.pool).await;

        let create_cache = "
            CREATE TABLE sp500_pe_cache AS
            SELECT
                s.symbol,
                s.company_name,
                dp.close_price as current_price,
                dvr.pe_ratio,
                dvr.market_cap,
                dvr.price,
                dp.date as price_date
            FROM stocks s
            JOIN daily_prices dp ON s.id = dp.stock_id
            JOIN daily_valuation_ratios dvr ON s.id = dvr.stock_id AND dvr.date = dp.date
            WHERE s.status = 'active'
            AND dp.date = (SELECT MAX(date) FROM daily_prices WHERE stock_id = s.id)
            AND dvr.pe_ratio IS NOT NULL
            ORDER BY s.symbol
        ";

        match sqlx::query(create_cache).execute(&self.pool).await {
            Ok(_) => println!("✅ Phase 2 Complete: Cache table refreshed"),
            Err(e) => println!("⚠️ Phase 2 Warning: Cache refresh failed: {}", e),
        }

        println!("✅ P/E calculation completed - {} total records processed", pe_count);
        Ok(pe_count as i64)
    }

    /// Parallel EPS calculation by stock using income_statements table
    async fn calculate_eps_parallel(&self) -> Result<usize> {
        println!("🧮 Parallel EPS calculation starting...");

        // Get stocks that need EPS calculation in income_statements
        let stocks_query = r#"
            SELECT DISTINCT i.stock_id, s.symbol
            FROM income_statements i
            JOIN stocks s ON s.id = i.stock_id
            WHERE i.net_income IS NOT NULL
            AND i.shares_diluted IS NOT NULL
            AND i.shares_diluted > 0
            AND i.period_type = 'TTM'
            ORDER BY s.symbol
        "#;

        let stocks = sqlx::query_as::<_, (i32, String)>(stocks_query)
            .fetch_all(&self.pool)
            .await?;

        if stocks.is_empty() {
            println!("📊 No stocks need EPS calculation");
            return Ok(0);
        }

        println!("📊 Found {} stocks for EPS calculation", stocks.len());

        // Parallel processing with semaphore
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(30)); // Max 30 concurrent
        let mut tasks = Vec::new();

        for (stock_id, symbol) in stocks {
            let permit = semaphore.clone();
            let pool = self.pool.clone();

            let task = tokio::spawn(async move {
                let _permit = permit.acquire().await.unwrap();

                // Calculate and store P/E ratios directly (this replaces separate EPS calculation)
                let pe_calculation = r#"
                    INSERT OR REPLACE INTO daily_valuation_ratios
                    (stock_id, date, pe_ratio, market_cap, price)
                    SELECT
                        dp.stock_id,
                        dp.date,
                        CASE
                            WHEN i.shares_diluted > 0 AND i.net_income > 0
                            THEN dp.close_price / (i.net_income / i.shares_diluted)
                            ELSE NULL
                        END as pe_ratio,
                        CASE
                            WHEN i.shares_diluted > 0
                            THEN dp.close_price * i.shares_diluted
                            ELSE NULL
                        END as market_cap,
                        dp.close_price as price
                    FROM daily_prices dp
                    JOIN (
                        SELECT stock_id, net_income, shares_diluted, fiscal_year, report_date,
                               ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
                        FROM income_statements
                        WHERE stock_id = ?
                        AND period_type = 'TTM'
                        AND net_income IS NOT NULL
                        AND shares_diluted IS NOT NULL
                        AND shares_diluted > 0
                    ) i ON i.stock_id = dp.stock_id AND i.rn = 1
                    WHERE dp.stock_id = ?
                    AND dp.close_price IS NOT NULL
                    AND dp.close_price > 0
                "#;

                match sqlx::query(pe_calculation)
                    .bind(stock_id)
                    .bind(stock_id)
                    .execute(&pool)
                    .await
                {
                    Ok(result) => {
                        let rows_updated = result.rows_affected();
                        if rows_updated > 0 {
                            println!("✅ {} - {} P/E ratios calculated", symbol, rows_updated);
                        }
                        Ok((symbol, rows_updated))
                    }
                    Err(e) => {
                        println!("⚠️ {} - P/E calculation failed: {}", symbol, e);
                        Err(anyhow!("P/E failed for {}: {}", symbol, e))
                    }
                }
            });
            tasks.push(task);
        }

        // Wait for all tasks to complete
        println!("🚀 Processing {} stocks concurrently (max 30 parallel)...", tasks.len());

        let mut total_ratios_calculated = 0;
        let mut processed_stocks = 0;

        for task in tasks {
            match task.await {
                Ok(Ok((_symbol, ratio_count))) => {
                    total_ratios_calculated += ratio_count as usize;
                    processed_stocks += 1;
                }
                Ok(Err(e)) => {
                    println!("⚠️ P/E task failed: {}", e);
                }
                Err(e) => {
                    println!("⚠️ P/E task panicked: {}", e);
                }
            }
        }

        println!("✅ P/E ratio calculation completed - {} stocks, {} P/E ratios",
                processed_stocks, total_ratios_calculated);
        Ok(total_ratios_calculated)
    }


    /// Calculate P/S and EV/S ratios (internal helper)
    async fn calculate_ps_evs_ratios_internal(&self) -> Result<i64> {
        println!("📊 Calculating P/S and EV/S ratios...");

        // Use internal ratio calculator directly
        let stats = ratio_calculator::calculate_ps_and_evs_ratios(&self.pool).await?;

        let total_ratios = stats.ps_ratios_calculated + stats.evs_ratios_calculated;
        println!("✅ P/S and EV/S ratios calculated - {} stocks processed, {} ratios calculated",
                 stats.stocks_processed, total_ratios);
        Ok(total_ratios as i64)
    }

    // ========================================
    // OLD FUNCTIONS (TO BE REMOVED)
    // ========================================

    #[allow(dead_code)]
    /// OLD: Refresh daily price data using incremental updates
    async fn _old_refresh_daily_prices(&self, _session_id: &str) -> Result<i64> {

        // Start the command but don't wait for completion
        let mut child = Command::new("cargo")
            .args(&["run", "--bin", "import-schwab-prices"])
            .spawn()?;

        // Show periodic progress while running
        let mut elapsed = 0;
        while let Ok(None) = child.try_wait() {
            sleep(StdDuration::from_secs(30)).await;
            elapsed += 30;
            println!("⏱️  Price refresh running... {} seconds elapsed", elapsed);
        }

        // Wait for final completion
        let output = child.wait().await?;

        if !output.success() {
            return Err(anyhow!("Price refresh failed"));
        }

        // Check how many records were actually updated by querying the database
        let result = sqlx::query("SELECT COUNT(*) as count FROM daily_prices WHERE date >= date('now', '-30 days')")
            .fetch_one(&self.pool)
            .await?;
        let recent_records: i64 = result.get("count");

        println!("✅ Price refresh completed - {} recent records", recent_records);
        Ok(recent_records)
    }

    /// Refresh P/E ratios
    async fn refresh_pe_ratios(&self, _session_id: &str) -> Result<i64> {
        println!("📊 Refreshing P/E ratios...");

        // Start the command but don't wait for completion
        let mut child = Command::new("cargo")
            .args(&["run", "--bin", "run_pe_calculation"])
            .spawn()?;

        // Show periodic progress while running
        let mut elapsed = 0;
        while let Ok(None) = child.try_wait() {
            sleep(StdDuration::from_secs(30)).await;
            elapsed += 30;
            println!("⏱️  P/E calculation running... {} seconds elapsed", elapsed);
        }

        // Wait for final completion
        let output = child.wait().await?;

        if !output.success() {
            return Err(anyhow!("P/E ratio refresh failed"));
        }

        // Check how many P/E records were updated
        let result = sqlx::query("SELECT COUNT(*) as count FROM daily_valuation_ratios WHERE pe_ratio_ttm IS NOT NULL AND date >= date('now', '-30 days')")
            .fetch_one(&self.pool)
            .await?;
        let pe_records: i64 = result.get("count");

        println!("✅ P/E ratio refresh completed - {} records", pe_records);
        Ok(pe_records)
    }

    /// Refresh P/S and EV/S ratios
    async fn refresh_ps_evs_ratios(&self, _session_id: &str) -> Result<i64> {
        println!("📊 Refreshing P/S and EV/S ratios...");

        let output = Command::new("cargo")
            .args(&["run", "--bin", "calculate-ratios"])
            .output()
            .await?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("P/S and EV/S ratio refresh failed: {}", error));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        let records = self.parse_records_from_output(&output_str, "ratios");

        Ok(records)
    }

    /// Refresh financial statements (future implementation)
    async fn refresh_financial_statements(&self, _session_id: &str) -> Result<i64> {
        println!("📋 Financial statement refresh not yet implemented");
        // TODO: Implement direct EDGAR API refresh
        Ok(0)
    }

    /// Refresh company metadata
    async fn refresh_company_metadata(&self, _session_id: &str) -> Result<i64> {
        println!("🏢 Refreshing company metadata...");

        // Update coverage information based on current price data
        let query = r#"
            UPDATE company_metadata
            SET
                earliest_data_date = (
                    SELECT MIN(date) FROM daily_prices WHERE stock_id = company_metadata.stock_id
                ),
                latest_data_date = (
                    SELECT MAX(date) FROM daily_prices WHERE stock_id = company_metadata.stock_id
                ),
                total_trading_days = (
                    SELECT COUNT(*) FROM daily_prices WHERE stock_id = company_metadata.stock_id
                ),
                updated_at = CURRENT_TIMESTAMP
            WHERE stock_id IN (SELECT id FROM stocks WHERE is_sp500 = 1)
        "#;

        let result = sqlx::query(query).execute(&self.pool).await?;
        Ok(result.rows_affected() as i64)
    }

    /// Refresh EDGAR cash flow data for S&P 500 stocks
    async fn refresh_edgar_cash_flow(&self, _session_id: &str) -> Result<i64> {
        println!("💰 Extracting EDGAR cash flow data...");

        let edgar_extractor = EdgarDataExtractor::new("edgar_data", self.pool.clone());
        let mut total_records = 0;

        // Get S&P 500 CIKs
        let ciks = self.get_sp500_ciks().await?;
        println!("🔍 Processing {} S&P 500 companies for cash flow data", ciks.len());

        for (i, (stock_id, cik)) in ciks.iter().enumerate() {
            if i % 50 == 0 {
                println!("📊 Progress: {}/{} companies processed", i, ciks.len());
            }

            match edgar_extractor.extract_company_data(*cik).await {
                Ok(financial_data) => {
                    if !financial_data.cash_flow_data.is_empty() {
                        match edgar_extractor.store_financial_data(*stock_id, &financial_data).await {
                            Ok(records) => total_records += records,
                            Err(e) => eprintln!("⚠️ Failed to store data for CIK {}: {}", cik, e),
                        }
                    }
                }
                Err(e) => {
                    eprintln!("⚠️ Failed to extract EDGAR data for CIK {}: {}", cik, e);
                }
            }
        }

        println!("✅ EDGAR cash flow extraction complete: {} records processed", total_records);
        Ok(total_records as i64)
    }

    /// Refresh complete EDGAR financial data for S&P 500 stocks
    async fn refresh_edgar_full_extraction(&self, _session_id: &str) -> Result<i64> {
        println!("📚 Extracting complete EDGAR financial data...");

        let edgar_extractor = EdgarDataExtractor::new("edgar_data", self.pool.clone());
        let mut total_records = 0;

        // Get S&P 500 CIKs
        let ciks = self.get_sp500_ciks().await?;
        println!("🔍 Processing {} S&P 500 companies for complete EDGAR data", ciks.len());

        for (i, (stock_id, cik)) in ciks.iter().enumerate() {
            if i % 25 == 0 {
                println!("📊 Progress: {}/{} companies processed", i, ciks.len());
            }

            match edgar_extractor.extract_company_data(*cik).await {
                Ok(financial_data) => {
                    match edgar_extractor.store_financial_data(*stock_id, &financial_data).await {
                        Ok(records) => total_records += records,
                        Err(e) => eprintln!("⚠️ Failed to store data for CIK {}: {}", cik, e),
                    }
                }
                Err(e) => {
                    eprintln!("⚠️ Failed to extract EDGAR data for CIK {}: {}", cik, e);
                }
            }
        }

        println!("✅ Complete EDGAR extraction complete: {} records processed", total_records);
        Ok(total_records as i64)
    }

    /// Get S&P 500 CIKs for EDGAR extraction
    async fn get_sp500_ciks(&self) -> Result<Vec<(i32, i32)>> {
        let query = r#"
            SELECT s.id, c.cik
            FROM stocks s
            JOIN cik_mappings_sp500 c ON s.symbol = c.symbol
            WHERE s.is_sp500 = 1 AND c.cik IS NOT NULL
            ORDER BY s.symbol
        "#;

        let rows = sqlx::query(query).fetch_all(&self.pool).await?;

        let mut ciks = Vec::new();
        for row in rows {
            let stock_id: i32 = row.get("id");
            let cik: i32 = row.get("cik");
            ciks.push((stock_id, cik));
        }

        Ok(ciks)
    }

    /// Parse record counts from command output
    fn parse_records_from_output(&self, output: &str, record_type: &str) -> i64 {
        // Simple parsing - look for patterns like "123 price bars" or "456 records"
        for line in output.lines() {
            if line.contains(record_type) || line.contains("records") {
                // Extract number from the line
                let words: Vec<&str> = line.split_whitespace().collect();
                for word in words {
                    if let Ok(num) = word.replace(",", "").parse::<i64>() {
                        if num > 0 {
                            return num;
                        }
                    }
                }
            }
        }
        0 // Default if parsing fails
    }

    /// Define refresh steps for each mode (Clean 3-option architecture)
    fn define_refresh_steps() -> HashMap<RefreshMode, Vec<RefreshStep>> {
        let mut steps = HashMap::new();

        // Market mode: Schwab market data (independent)
        steps.insert(RefreshMode::Market, vec![
            RefreshStep {
                name: "Update market data".to_string(),
                data_source: "daily_prices".to_string(),
                estimated_duration_minutes: 15,
                command: "internal".to_string(), // Internal function call
                dependencies: vec![],
                priority: 1,
            },
        ]);

        // Financials mode: All EDGAR data (independent)
        steps.insert(RefreshMode::Financials, vec![
            RefreshStep {
                name: "Extract EDGAR financial data".to_string(),
                data_source: "financial_statements".to_string(),
                estimated_duration_minutes: 90,
                command: "internal".to_string(), // Internal function call
                dependencies: vec![],
                priority: 1,
            },
        ]);

        // Ratios mode: All calculated ratios (depends on Market + Financials)
        steps.insert(RefreshMode::Ratios, vec![
            RefreshStep {
                name: "Calculate all ratios and metrics".to_string(),
                data_source: "ps_evs_ratios".to_string(),
                estimated_duration_minutes: 10,
                command: "internal".to_string(), // Internal function call
                dependencies: vec!["daily_prices".to_string(), "financial_statements".to_string()],
                priority: 1,
            },
        ]);


        steps
    }

    /// Create progress tracking record
    async fn create_progress_record(&self, session_id: &str, request: &RefreshRequest) -> Result<()> {
        let steps = self.refresh_steps.get(&request.mode)
            .ok_or_else(|| anyhow!("Unknown refresh mode"))?;

        let query = r#"
            INSERT INTO refresh_progress (
                session_id, operation_type, total_steps, current_step_name,
                initiated_by, data_sources_refreshed
            ) VALUES (?, ?, ?, ?, ?, ?)
        "#;

        let data_sources_json = serde_json::to_string(&steps.iter().map(|s| &s.data_source).collect::<Vec<_>>())?;

        sqlx::query(query)
            .bind(session_id)
            .bind(match request.mode {
                RefreshMode::Market => "market",
                RefreshMode::Financials => "financials",
                RefreshMode::Ratios => "ratios",
            })
            .bind(steps.len() as i32 + 2) // +2 for start/finish
            .bind("Initializing")
            .bind(&request.initiated_by)
            .bind(data_sources_json)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Update progress for a specific step
    async fn update_progress(&self, session_id: &str, step_number: i32, step_name: &str, step_progress: f64) -> Result<()> {
        let query = r#"
            UPDATE refresh_progress
            SET completed_steps = ?, current_step_name = ?, current_step_progress = ?
            WHERE session_id = ?
        "#;

        sqlx::query(query)
            .bind(step_number)
            .bind(step_name)
            .bind(step_progress)
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Update total steps (used when plan is finalized)
    async fn update_progress_total_steps(&self, session_id: &str, total_steps: i32) -> Result<()> {
        let query = "UPDATE refresh_progress SET total_steps = ? WHERE session_id = ?";

        sqlx::query(query)
            .bind(total_steps)
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Mark progress as complete
    async fn mark_progress_complete(&self, session_id: &str, success: bool, error_details: Option<String>) -> Result<()> {
        let status = if success { "completed" } else { "error" };

        let query = r#"
            UPDATE refresh_progress
            SET end_time = CURRENT_TIMESTAMP, status = ?, error_details = ?
            WHERE session_id = ?
        "#;

        sqlx::query(query)
            .bind(status)
            .bind(error_details)
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Record the start of a data source refresh
    async fn record_refresh_start(&self, data_source: &str) -> Result<()> {
        let query = r#"
            UPDATE data_refresh_status
            SET last_refresh_start = CURRENT_TIMESTAMP, refresh_status = 'refreshing'
            WHERE data_source = ?
        "#;

        sqlx::query(query)
            .bind(data_source)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Record the completion of a data source refresh
    async fn record_refresh_complete(&self, data_source: &str, records_updated: i64, _duration_seconds: i32) -> Result<()> {
        println!("📝 Recording completion for {}: {} records", data_source, records_updated);
        self.update_refresh_status(data_source, true, Some(records_updated), None).await?;
        Ok(())
    }

    /// Update refresh status for a data source
    async fn update_refresh_status(&self, data_source: &str, success: bool, records: Option<i64>, error: Option<String>) -> Result<()> {
        let status = if success { "current" } else { "error" };

        println!("🔄 Updating refresh status for {}: success={}, records={:?}, status={}", data_source, success, records, status);

        let query = r#"
            UPDATE data_refresh_status
            SET
                last_successful_refresh = CASE WHEN ? THEN CURRENT_TIMESTAMP ELSE last_successful_refresh END,
                refresh_status = ?,
                records_updated = COALESCE(?, records_updated),
                latest_data_date = CASE WHEN ? THEN DATE('now') ELSE latest_data_date END,
                error_message = ?,
                updated_at = CURRENT_TIMESTAMP
            WHERE data_source = ?
        "#;

        let result = sqlx::query(query)
            .bind(success)
            .bind(status)
            .bind(records)
            .bind(success)  // For latest_data_date update
            .bind(error)
            .bind(data_source)
            .execute(&self.pool)
            .await?;

        println!("✅ Database update result: {} rows affected", result.rows_affected());

        Ok(())
    }

    /// Generate recommendations after refresh
    fn generate_post_refresh_recommendations(&self, freshness_report: &SystemFreshnessReport) -> Vec<String> {
        let mut recommendations = Vec::new();

        if freshness_report.screening_readiness.garp_screening {
            recommendations.push("GARP screening is now ready with current data".to_string());
        }

        if freshness_report.screening_readiness.graham_screening {
            recommendations.push("Graham value screening is now ready with current data".to_string());
        }

        if freshness_report.screening_readiness.valuation_analysis {
            recommendations.push("Valuation analysis is now ready with current ratios".to_string());
        }

        if !freshness_report.screening_readiness.blocking_issues.is_empty() {
            recommendations.push("Some screening features may still have data issues".to_string());
        }

        if recommendations.is_empty() {
            recommendations.push("All major data sources have been updated".to_string());
        }

        recommendations
    }

    /// Get current progress for a session
    pub async fn get_progress(&self, session_id: &str) -> Result<Option<RefreshProgress>> {
        let query = r#"
            SELECT
                session_id, operation_type, start_time, total_steps, completed_steps,
                current_step_name, current_step_progress, status, error_details
            FROM refresh_progress
            WHERE session_id = ?
        "#;

        let row = sqlx::query(query)
            .bind(session_id)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            let completed_steps: i32 = row.get("completed_steps");
            let total_steps: i32 = row.get("total_steps");
            let overall_progress = if total_steps > 0 {
                (completed_steps as f64 / total_steps as f64) * 100.0
            } else {
                0.0
            };

            Ok(Some(RefreshProgress {
                session_id: row.get("session_id"),
                operation_type: row.get("operation_type"),
                start_time: row.get::<String, _>("start_time").parse()?,
                total_steps,
                completed_steps,
                current_step_name: row.get("current_step_name"),
                current_step_progress: row.get("current_step_progress"),
                overall_progress_percent: overall_progress,
                estimated_completion: None, // TODO: Calculate based on progress and timing
                status: row.get("status"),
                error_details: row.get("error_details"),
            }))
        } else {
            Ok(None)
        }
    }

    /// Get system freshness status
    pub async fn get_system_status(&self) -> Result<SystemFreshnessReport> {
        self.status_reader.check_system_freshness().await
    }
}