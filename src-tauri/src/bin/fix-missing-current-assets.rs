/// Fix missing current assets and current liabilities for S&P 500 stocks
/// Re-extracts balance sheet data from EDGAR for stocks with missing fields

use rust_stocks_tauri_lib::tools::sec_edgar_client::SecEdgarClient;
use sqlx::{Row, SqlitePool};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🔧 Fix Missing Current Assets/Liabilities for Piotroski F-Score");
    println!("================================================================\n");

    // Get database URL from environment or use default
    let database_url = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_PATH").map(|p| format!("sqlite:{}", p)))
        .unwrap_or_else(|_| "sqlite:src-tauri/db/stocks.db".to_string());

    println!("📂 Connecting to database: {}", database_url);
    let pool = SqlitePool::connect(&database_url).await?;

    // Find S&P 500 stocks with missing current assets or current liabilities
    let query = r#"
        SELECT DISTINCT s.id, s.symbol
        FROM stocks s
        WHERE s.is_sp500 = 1
        AND s.id IN (
            SELECT pf.stock_id
            FROM piotroski_f_score_complete pf
            WHERE pf.current_current_assets IS NULL
            OR pf.current_current_liabilities IS NULL
        )
        ORDER BY s.symbol
    "#;

    let missing_stocks = sqlx::query(query)
        .fetch_all(&pool)
        .await?;

    println!("📊 Found {} S&P 500 stocks missing current assets/liabilities\n", missing_stocks.len());

    if missing_stocks.is_empty() {
        println!("✅ No stocks need fixing!");
        return Ok(());
    }

    // Get CIKs for these stocks
    let mut stocks_to_fix = Vec::new();
    for row in missing_stocks {
        let stock_id: i64 = row.get("id");
        let symbol: String = row.get("symbol");

        // Get CIK
        let cik_query = "SELECT cik FROM cik_mappings WHERE symbol = ?";
        if let Ok(cik_row) = sqlx::query(cik_query)
            .bind(&symbol)
            .fetch_one(&pool)
            .await
        {
            let cik: String = cik_row.get("cik");
            stocks_to_fix.push((stock_id, symbol, cik));
        } else {
            println!("⚠️  No CIK found for {}", symbol);
        }
    }

    println!("🔍 Will re-extract balance sheet data for {} stocks", stocks_to_fix.len());
    println!("   Stocks: {:?}\n", stocks_to_fix.iter().map(|(_, s, _)| s).collect::<Vec<_>>());

    // Re-extract balance sheet data
    let mut edgar_client = SecEdgarClient::new(pool.clone());
    let mut success_count = 0;
    let mut failed_count = 0;

    for (i, (stock_id, symbol, cik)) in stocks_to_fix.iter().enumerate() {
        println!("\n[{}/{}] Processing {} (CIK: {})", i + 1, stocks_to_fix.len(), symbol, cik);

        match edgar_client.extract_balance_sheet_data(cik, *stock_id, symbol).await {
            Ok(Some(_)) => {
                println!("  ✅ Successfully extracted balance sheet data for {}", symbol);
                success_count += 1;
            }
            Ok(None) => {
                println!("  ⚠️  No balance sheet data found for {}", symbol);
                failed_count += 1;
            }
            Err(e) => {
                println!("  ❌ Error extracting data for {}: {}", symbol, e);
                failed_count += 1;
            }
        }

        // Be nice to SEC servers
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
    }

    println!("\n================================================================");
    println!("✅ Re-extraction complete!");
    println!("   Success: {} stocks", success_count);
    println!("   Failed:  {} stocks", failed_count);

    // Check results
    let updated_missing = sqlx::query(
        "SELECT COUNT(*) as count FROM piotroski_f_score_complete 
         WHERE stock_id IN (SELECT id FROM stocks WHERE is_sp500 = 1)
         AND (current_current_assets IS NULL OR current_current_liabilities IS NULL)"
    )
    .fetch_one(&pool)
    .await?;

    let remaining_missing: i64 = updated_missing.get("count");
    println!("   Remaining missing: {} stocks", remaining_missing);

    Ok(())
}

