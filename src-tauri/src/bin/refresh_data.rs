/// Unified Data Refresh System
///
/// A comprehensive data refresh system that ensures all stock screening features
/// have current, complete data before analysis. Supports incremental updates,
/// progress tracking, and different refresh modes for various use cases.

use anyhow::Result;
use clap::Parser;
use sqlx::sqlite::SqlitePoolOptions;
use chrono::Local;

use rust_stocks_tauri_lib::tools::{
    data_refresh_orchestrator::{
        DataRefreshManager, RefreshRequest, RefreshMode
    },
    data_freshness_checker::{DataStatusReader, FreshnessStatus, RefreshPriority},
};

#[derive(Parser)]
#[command(
    name = "refresh_data",
    about = "🔄 Stock data refresh system",
    long_about = "Updates market data (Schwab) and financial data (EDGAR). Run without options to check status."
)]
struct Cli {
    /// What to refresh: market or financials
    #[arg(value_enum)]
    mode: Option<RefreshMode>,

    /// Show current data status (default if no mode specified)
    #[arg(long, short)]
    status: bool,

    /// Show what would be refreshed without doing it
    #[arg(long, short)]
    preview: bool,

    /// Show detailed progress information
    #[arg(long, short)]
    verbose: bool,

    /// Only process specific CIK (for debugging/testing)
    #[arg(long)]
    only_cik: Option<String>,

    /// Only process specific ticker (maps to its CIK)
    #[arg(long)]
    only_ticker: Option<String>,
}



#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging if verbose
    if cli.verbose {
        tracing_subscriber::fmt::init();
    }

    // Auto-detect database path with WAL mode optimization
    let database_path = "db/stocks.db";
    let database_url = format!("sqlite:{}?mode=rwc", database_path);

    // Optimized connection pool for parallel processing
    let pool = SqlitePoolOptions::new()
        .max_connections(50) // Increased for parallel processing
        .acquire_timeout(std::time::Duration::from_secs(30))
        .connect(&database_url)
        .await?;

    // Enable WAL mode for better concurrency
    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(&pool)
        .await?;

    // Optimize SQLite settings for performance
    sqlx::query("PRAGMA synchronous = NORMAL")
        .execute(&pool)
        .await?;

    sqlx::query("PRAGMA cache_size = 10000")
        .execute(&pool)
        .await?;

    sqlx::query("PRAGMA temp_store = memory")
        .execute(&pool)
        .await?;

    println!("🔄 Stock Data Refresh");
    println!("📅 {}", Local::now().format("%Y-%m-%d %H:%M:%S"));
    println!("══════════════════════════════════════");

    // Default behavior: show status if no mode specified
    if cli.mode.is_none() && !cli.status && !cli.preview {
        return show_data_status(&pool, &cli).await;
    }

    if cli.status {
        return show_data_status(&pool, &cli).await;
    }

    if cli.preview {
        return show_refresh_plan(&pool, &cli).await;
    }

    // Execute the refresh
    if let Some(ref mode) = cli.mode {
        execute_data_refresh(&pool, &cli, mode.clone()).await
    } else {
        show_data_status(&pool, &cli).await
    }
}

/// Show current data freshness status
async fn show_data_status(pool: &sqlx::SqlitePool, cli: &Cli) -> Result<()> {
    println!("🔍 Checking current data freshness status...\n");

    let freshness_checker = DataStatusReader::new(pool.clone());
    let report = freshness_checker.check_system_freshness().await?;

    // Display overall status
    println!("📊 OVERALL STATUS: {:?}", report.overall_status);
    println!("🕐 Last check: {}", report.last_check);
    println!();

    // Display individual data sources (excluding screening_readiness as it's not a data source)
    println!("📋 DATA SOURCE STATUS:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut sources = vec![&report.market_data, &report.financial_data];
    sources.sort_by(|a, b| b.refresh_priority.partial_cmp(&a.refresh_priority).unwrap());

    for source in sources {
        let status_emoji = match source.status {
            FreshnessStatus::Current => "✅",
            FreshnessStatus::Stale => "⚠️",
            FreshnessStatus::Missing => "❌",
            FreshnessStatus::Error => "🔥",
        };

        let staleness_info = if let Some(days) = source.staleness_days {
            format!("({} days old)", days)
        } else {
            "".to_string()
        };

        println!("{} {:20} {:>12} {:>10} records {}",
                status_emoji,
                source.data_source,
                format!("{:?}", source.status),
                source.records_count,
                staleness_info
        );

        if cli.verbose {
            println!("   └─ {}", source.message);
        }
    }

    println!();

    // Display screening readiness
    println!("🎯 SCREENING READINESS:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let is_financial_data_stale = report.financial_data.status.needs_refresh();
    let is_market_data_stale = report.market_data.status.needs_refresh();
    
    if is_financial_data_stale || is_market_data_stale {
        println!("❌ Valuation Analysis:     ❌ Blocked");
        println!();
        println!("Reason:");
        println!("We require fresh data for screening analysis.");
        
        let mut stale_data_types = Vec::new();
        let mut commands = Vec::new();
        
        if is_financial_data_stale {
            stale_data_types.push("financial statements");
            commands.push("cargo run --bin refresh_data financials");
        }
        
        if is_market_data_stale {
            stale_data_types.push("daily prices");  
            commands.push("cargo run --bin refresh_data market");
        }
        
        println!("We require fresh data for <{}>.", stale_data_types.join(" and "));
        println!("Please run the following command:");
        println!("{}", commands.join(" && "));
        
    } else {
        println!("✅ Valuation Analysis:     ✅ Ready");
        println!("");
        println!("All required data is current:");
        println!("   • Financial statements: Up-to-date with SEC filings");
        println!("   • Daily prices: Current market data");
        println!("   • Screening algorithms: Ready to run");
    }

    println!();

    // Display recommendations
    if !report.recommendations.is_empty() {
        println!("💡 WHAT TO DO:");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        for (i, rec) in report.recommendations.iter().enumerate() {
            let priority_emoji = match rec.priority {
                RefreshPriority::Critical => "🔥",
                RefreshPriority::High => "⚠️",
                RefreshPriority::Medium => "📋",
                RefreshPriority::Low => "📝",
            };

            println!("{}. {} {}", i + 1, priority_emoji, rec.action);
            println!("   └─ {} (Est: {})", rec.reason, rec.estimated_duration);
        }
        println!();
    } else {
        println!("✅ All data sources are current. No refresh needed.");
    }

    Ok(())
}

/// Show what would be refreshed without executing
async fn show_refresh_plan(pool: &sqlx::SqlitePool, cli: &Cli) -> Result<()> {
    let mode = cli.mode.clone().unwrap_or(RefreshMode::Market);
    println!("🔍 Preview: What would be refreshed with {:?} mode\n", mode);

    let freshness_checker = DataStatusReader::new(pool.clone());
    let _orchestrator = DataRefreshManager::new(pool.clone()).await?;

    let report = freshness_checker.check_system_freshness().await?;

    // Create a mock request to determine the plan
    let request = RefreshRequest {
        mode: mode.clone(),
        force_sources: vec![], // Simplified CLI doesn't have force option
        initiated_by: "preview".to_string(),
        session_id: None,
        only_cik: None, // Preview doesn't support single CIK filtering
    };

    println!("📋 REFRESH PLAN:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Simulate the planning logic
    let available_steps = get_available_steps(&request.mode);
    let mut plan_steps = Vec::new();

    // Always check what needs refreshing based on staleness
    for (source, duration) in &available_steps {
        let source_status = match source.as_str() {
            "daily_prices" => &report.market_data,
            "financial_statements" => &report.financial_data,
            "ps_evs_ratios" => &report.calculated_ratios,
            _ => continue,
        };
        if source_status.status.needs_refresh() {
            plan_steps.push((source.clone(), *duration));
        }
    }

    if plan_steps.is_empty() {
        println!("✅ No refresh needed - all data sources are current");
        return Ok(());
    }

    let mut total_duration = 0;
    for (i, (source, duration)) in plan_steps.iter().enumerate() {
        total_duration += *duration;
        println!("{}. {} (~{} min)", i + 1, source, duration);
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 Total steps: {} | Estimated duration: ~{} minutes", plan_steps.len(), total_duration);
    println!();
    println!("💡 To execute this plan: cargo run --bin refresh_data {:?}", mode);

    Ok(())
}

/// Execute the data refresh
async fn execute_data_refresh(pool: &sqlx::SqlitePool, cli: &Cli, mode: RefreshMode) -> Result<()> {
    // Skip freshness check if filtering by ticker (faster testing)
    if cli.only_ticker.is_none() && cli.only_cik.is_none() {
        println!("🔍 Checking data freshness before refresh...\n");

        // First, check what needs refreshing
        let freshness_checker = DataStatusReader::new(pool.clone());
        let report = freshness_checker.check_system_freshness().await?;

        // Determine what data sources need refreshing
        let mut stale_sources = Vec::new();
        let mut total_stale_count = 0;

        match mode {
            RefreshMode::Financials => {
                if report.financial_data.status.needs_refresh() {
                    stale_sources.push("financial statements");
                    total_stale_count += report.financial_data.records_count;
                }
            }
            RefreshMode::Market => {
                if report.market_data.status.needs_refresh() {
                    stale_sources.push("market data");
                    total_stale_count += report.market_data.records_count;
                }
            }
        }

        // If no stale data, inform user and exit
        if stale_sources.is_empty() {
            println!("✅ All {:?} data is current. No refresh needed.", mode);
            println!("💡 You can run screening algorithms with confidence.");
            return Ok(());
        }

        // Show what will be refreshed
        println!("📊 REFRESH PLAN:");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("🎯 Mode: {:?}", mode);
        println!("📋 Data sources to refresh: {}", stale_sources.join(", "));
        println!("📊 Total records: {}", total_stale_count);
        println!("⏱️  Estimated duration: 2-5 minutes");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    }

    // Auto-execute single-stage architecture (no confirmation)
    println!("\n🚀 Starting data refresh in {:?} mode...\n", mode);

    let orchestrator = DataRefreshManager::new(pool.clone()).await?;

    // Resolve only_ticker to CIK if provided; only_ticker overrides only_cik
    let resolved_only_cik = if let Some(tkr) = &cli.only_ticker {
        let t_upper = tkr.trim().to_uppercase();
        let cik_opt: Option<String> = sqlx::query_scalar(
            r#"SELECT cik FROM stocks WHERE UPPER(symbol) = ? AND cik IS NOT NULL AND cik != '' LIMIT 1"#
        )
        .bind(&t_upper)
        .fetch_optional(pool)
        .await?;
        if let Some(cik) = cik_opt {
            println!("🎯 --only-ticker={} resolved to CIK {}", t_upper, cik);
            Some(cik)
        } else {
            println!("❌ Could not resolve ticker '{}' to a CIK. Proceeding without filter.", t_upper);
            None
        }
    } else {
        cli.only_cik.clone()
    };

    let request = RefreshRequest {
        mode,
        force_sources: vec![],
        initiated_by: "cli".to_string(),
        session_id: None,
        only_cik: resolved_only_cik,
    };

    let result = orchestrator.execute_refresh(request).await?;

    println!("\n🎉 REFRESH COMPLETE!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📝 Session ID: {}", result.session_id);
    println!("⏱️  Duration: {} seconds", result.duration_seconds.unwrap_or(0));
    println!("📊 Records processed: {}", result.total_records_processed);

    if !result.sources_refreshed.is_empty() {
        println!("✅ Refreshed: {}", result.sources_refreshed.join(", "));
    }

    if !result.sources_failed.is_empty() {
        println!("❌ Failed: {}", result.sources_failed.join(", "));
    }

    if !result.recommendations.is_empty() {
        println!("\n💡 POST-REFRESH STATUS:");
        for rec in result.recommendations {
            println!("   • {}", rec);
        }
    }

    println!();
    if result.success {
        println!("✅ All screening features should now be ready with current data!");
        println!("💡 You can now run Piotroski F-Score or O'Shaughnessy Value screening with confidence.");
    } else {
        println!("⚠️  Some refresh operations failed. Check the details above.");
        println!("💡 You may want to retry with --force to refresh specific sources.");
    }

    Ok(())
}

/// Get available refresh steps for a mode (simplified for dry-run)
fn get_available_steps(mode: &RefreshMode) -> Vec<(String, i32)> {
    let mut steps = vec![
        ("daily_prices".to_string(), 15),
        ("pe_ratios".to_string(), 25),
        ("company_metadata".to_string(), 2),
    ];

    match mode {
        RefreshMode::Financials => {
            // Financials mode includes TTM cash flow calculation
            steps.push(("cash_flow_statements".to_string(), 8));
        }
        _ => {}
    }

    if matches!(mode, RefreshMode::Financials) {
        steps.push(("financial_statements".to_string(), 45));
    }

    steps
}
