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
        DataRefreshOrchestrator, RefreshRequest, RefreshMode
    },
    data_freshness_checker::{DataFreshnessChecker, FreshnessStatus, RefreshPriority},
};

#[derive(Parser)]
#[command(
    name = "refresh_data",
    about = "🔄 Stock data refresh system",
    long_about = "Updates market data (Schwab), financial data (EDGAR), and calculated ratios. Run without options to check status."
)]
struct Cli {
    /// What to refresh: market, financials, or ratios
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
}



#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging if verbose
    if cli.verbose {
        tracing_subscriber::fmt::init();
    }

    // Auto-detect database path
    let database_path = "db/stocks.db";
    let database_url = format!("sqlite:{}?mode=rwc", database_path);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
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

    let freshness_checker = DataFreshnessChecker::new(pool.clone());
    let report = freshness_checker.check_system_freshness().await?;

    // Display overall status
    println!("📊 OVERALL STATUS: {:?}", report.overall_status);
    println!("🕐 Last check: {}", report.last_check.format("%Y-%m-%d %H:%M:%S UTC"));
    println!();

    // Display individual data sources
    println!("📋 DATA SOURCE STATUS:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut sources: Vec<_> = report.data_sources.values().collect();
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

    let garp_status = if report.screening_readiness.garp_screening { "✅ Ready" } else { "❌ Blocked" };
    let graham_status = if report.screening_readiness.graham_screening { "✅ Ready" } else { "❌ Blocked" };
    let valuation_status = if report.screening_readiness.valuation_analysis { "✅ Ready" } else { "❌ Blocked" };

    println!("GARP P/E Screening:     {}", garp_status);
    println!("Graham Value Screening: {}", graham_status);
    println!("Valuation Analysis:     {}", valuation_status);

    if !report.screening_readiness.blocking_issues.is_empty() {
        println!("\n⚠️  BLOCKING ISSUES:");
        for issue in &report.screening_readiness.blocking_issues {
            println!("   • {}", issue);
        }
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
        println!("💡 Quick fix: cargo run --bin refresh_data market");
        println!("💡 Full fix:  cargo run --bin refresh_data ratios");
    } else {
        println!("✅ All data sources are current. No refresh needed.");
    }

    Ok(())
}

/// Show what would be refreshed without executing
async fn show_refresh_plan(pool: &sqlx::SqlitePool, cli: &Cli) -> Result<()> {
    let mode = cli.mode.clone().unwrap_or(RefreshMode::Market);
    println!("🔍 Preview: What would be refreshed with {:?} mode\n", mode);

    let freshness_checker = DataFreshnessChecker::new(pool.clone());
    let _orchestrator = DataRefreshOrchestrator::new(pool.clone()).await?;

    let report = freshness_checker.check_system_freshness().await?;

    // Create a mock request to determine the plan
    let request = RefreshRequest {
        mode: mode.clone(),
        force_sources: vec![], // Simplified CLI doesn't have force option
        initiated_by: "preview".to_string(),
        session_id: None,
    };

    println!("📋 REFRESH PLAN:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Simulate the planning logic
    let available_steps = get_available_steps(&request.mode);
    let mut plan_steps = Vec::new();

    // Always check what needs refreshing based on staleness
    for (source, duration) in &available_steps {
        if let Some(source_status) = report.data_sources.get(source) {
            if source_status.status.needs_refresh() {
                plan_steps.push((source.clone(), *duration));
            }
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
async fn execute_data_refresh(pool: &sqlx::SqlitePool, _cli: &Cli, mode: RefreshMode) -> Result<()> {
    println!("🚀 Starting data refresh in {:?} mode...\n", mode);

    let orchestrator = DataRefreshOrchestrator::new(pool.clone()).await?;

    let request = RefreshRequest {
        mode,
        force_sources: vec![],
        initiated_by: "cli".to_string(),
        session_id: None,
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
        println!("💡 You can now run GARP or Graham screening with confidence.");
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
        RefreshMode::Ratios | RefreshMode::Financials => {
            steps.push(("ps_evs_ratios".to_string(), 8));
        }
        _ => {}
    }

    if matches!(mode, RefreshMode::Financials) {
        steps.push(("financial_statements".to_string(), 45));
    }

    steps
}