use sqlx::{SqlitePool, Row};
use std::collections::HashMap;
use tokio::time::{Duration, Instant};
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Debug)]
struct OShaughnessyRatios {
    stock_id: i64,
    symbol: String,

    // Current ratios (for reference)
    market_cap: Option<f64>,
    enterprise_value: Option<f64>,

    // New ratios to calculate
    pe_ratio: Option<f64>,
    pb_ratio: Option<f64>,
    ev_ebitda_ratio: Option<f64>,
    shareholder_yield: Option<f64>,
}

struct OShaughnessyRatioCalculator {
    pool: SqlitePool,
}

impl OShaughnessyRatioCalculator {
    fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    async fn calculate_pe_ratio(&self, stock_id: i64, symbol: &str) -> Result<Option<f64>, String> {
        let query = "
            SELECT
                dvr.price,
                inc.net_income,
                inc.shares_diluted
            FROM daily_valuation_ratios dvr
            JOIN income_statements inc ON dvr.stock_id = inc.stock_id
            WHERE dvr.stock_id = ? AND inc.period_type = 'TTM'
            ORDER BY dvr.date DESC, inc.report_date DESC LIMIT 1";

        let result = sqlx::query(query)
            .bind(stock_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| format!("P/E query failed for {}: {}", symbol, e))?;

        if let Some(row) = result {
            let price: Option<f64> = row.get("price");
            let net_income: Option<f64> = row.get("net_income");
            let shares_diluted: Option<f64> = row.get("shares_diluted");

            match (price, net_income, shares_diluted) {
                (Some(p), Some(ni), Some(shares)) if shares > 0.0 && ni > 0.0 => {
                    let eps = ni / shares;
                    if eps > 0.0 {
                        Ok(Some(p / eps))
                    } else {
                        Ok(None) // Negative earnings
                    }
                }
                _ => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    async fn calculate_pb_ratio(&self, stock_id: i64, symbol: &str) -> Result<Option<f64>, String> {
        let query = "
            SELECT
                dvr.market_cap,
                bv.book_value_per_share,
                bv.shares_outstanding
            FROM daily_valuation_ratios dvr
            JOIN book_value_calculations bv ON dvr.stock_id = bv.stock_id
            WHERE dvr.stock_id = ?
            ORDER BY dvr.date DESC, bv.report_date DESC LIMIT 1";

        let result = sqlx::query(query)
            .bind(stock_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| format!("P/B query failed for {}: {}", symbol, e))?;

        if let Some(row) = result {
            let market_cap: Option<f64> = row.get("market_cap");
            let book_value_per_share: Option<f64> = row.get("book_value_per_share");
            let shares_outstanding: Option<f64> = row.get("shares_outstanding");

            match (market_cap, book_value_per_share, shares_outstanding) {
                (Some(mc), Some(bvps), Some(shares)) if shares > 0.0 => {
                    let book_value = bvps * shares;
                    if book_value > 0.0 {
                        Ok(Some(mc / book_value))
                    } else {
                        Ok(None)
                    }
                }
                _ => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    async fn calculate_ev_ebitda(&self, stock_id: i64, symbol: &str) -> Result<Option<f64>, String> {
        let query = "
            SELECT
                dvr.enterprise_value,
                eb.ebitda
            FROM daily_valuation_ratios dvr
            JOIN ebitda_calculations eb ON dvr.stock_id = eb.stock_id
            WHERE dvr.stock_id = ? AND eb.period_type = 'TTM'
            ORDER BY dvr.date DESC LIMIT 1";

        let result = sqlx::query(query)
            .bind(stock_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| format!("EV/EBITDA query failed for {}: {}", symbol, e))?;

        if let Some(row) = result {
            let enterprise_value: Option<f64> = row.get("enterprise_value");
            let ebitda: Option<f64> = row.get("ebitda");

            match (enterprise_value, ebitda) {
                (Some(ev), Some(ebitda_val)) if ebitda_val > 0.0 => Ok(Some(ev / ebitda_val)),
                _ => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    async fn calculate_shareholder_yield(&self, stock_id: i64, symbol: &str) -> Result<Option<f64>, String> {
        let query = "
            SELECT
                dvr.market_cap,
                cf.dividends_paid,
                cf.share_repurchases
            FROM daily_valuation_ratios dvr
            JOIN cash_flow_statements cf ON dvr.stock_id = cf.stock_id
            WHERE dvr.stock_id = ? AND cf.period_type = 'TTM'
            ORDER BY dvr.date DESC LIMIT 1";

        let result = sqlx::query(query)
            .bind(stock_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| format!("Shareholder yield query failed for {}: {}", symbol, e))?;

        if let Some(row) = result {
            let market_cap: Option<f64> = row.get("market_cap");
            let dividends_paid: Option<f64> = row.get("dividends_paid");
            let share_repurchases: Option<f64> = row.get("share_repurchases");

            match market_cap {
                Some(mc) if mc > 0.0 => {
                    let total_return = dividends_paid.map(|d| d.abs()).unwrap_or(0.0) +
                                      share_repurchases.map(|r| r.abs()).unwrap_or(0.0);
                    if total_return > 0.0 {
                        Ok(Some((total_return / mc) * 100.0)) // Return as percentage
                    } else {
                        Ok(Some(0.0))
                    }
                }
                _ => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    async fn calculate_ratios_for_stock(&self, stock_id: i64, symbol: String) -> Result<OShaughnessyRatios, String> {
        let pe_ratio = self.calculate_pe_ratio(stock_id, &symbol).await?;
        let pb_ratio = self.calculate_pb_ratio(stock_id, &symbol).await?;
        let ev_ebitda_ratio = self.calculate_ev_ebitda(stock_id, &symbol).await?;
        let shareholder_yield = self.calculate_shareholder_yield(stock_id, &symbol).await?;

        // Get current market data
        let market_data_query = "
            SELECT market_cap, enterprise_value
            FROM daily_valuation_ratios
            WHERE stock_id = ?
            ORDER BY date DESC LIMIT 1";

        let market_data = sqlx::query(market_data_query)
            .bind(stock_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| format!("Market data query failed for {}: {}", symbol, e))?;

        let (market_cap, enterprise_value) = if let Some(row) = market_data {
            (row.get("market_cap"), row.get("enterprise_value"))
        } else {
            (None, None)
        };

        Ok(OShaughnessyRatios {
            stock_id,
            symbol,
            market_cap,
            enterprise_value,
            pe_ratio,
            pb_ratio,
            ev_ebitda_ratio,
            shareholder_yield,
        })
    }

    async fn update_daily_valuation_ratios(&self, ratios: &OShaughnessyRatios) -> Result<(), String> {
        let query = "
            UPDATE daily_valuation_ratios
            SET pe_ratio = ?,
                pb_ratio_ttm = ?,
                ebitda_ttm = ?,
                ev_ebitda_ratio_ttm = ?,
                shareholder_yield_ttm = ?
            WHERE stock_id = ? AND date = (
                SELECT date FROM daily_valuation_ratios WHERE stock_id = ? ORDER BY date DESC LIMIT 1
            )";

        // Get EBITDA value for storage
        let ebitda_query = "SELECT ebitda FROM ebitda_calculations WHERE stock_id = ? LIMIT 1";
        let ebitda_result = sqlx::query(ebitda_query)
            .bind(ratios.stock_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| format!("EBITDA fetch failed for {}: {}", ratios.symbol, e))?;

        let ebitda_value: Option<f64> = ebitda_result.and_then(|row| row.get("ebitda"));

        sqlx::query(query)
            .bind(ratios.pe_ratio)
            .bind(ratios.pb_ratio)
            .bind(ebitda_value)
            .bind(ratios.ev_ebitda_ratio)
            .bind(ratios.shareholder_yield)
            .bind(ratios.stock_id)
            .bind(ratios.stock_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Update failed for {}: {}", ratios.symbol, e))?;

        Ok(())
    }

    async fn process_all_stocks(&self) -> Result<(), String> {
        println!("🔍 Fetching S&P 500 stocks for O'Shaughnessy ratio calculation...");

        let stocks_query = "
            SELECT DISTINCT s.id, s.symbol
            FROM stocks s
            JOIN daily_valuation_ratios dvr ON s.id = dvr.stock_id
            WHERE s.is_sp500 = 1
            ORDER BY s.symbol";

        let stocks = sqlx::query(stocks_query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("Failed to fetch stocks: {}", e))?;

        println!("📊 Processing {} S&P 500 stocks for O'Shaughnessy ratios", stocks.len());

        let progress_bar = ProgressBar::new(stocks.len() as u64);
        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
                .unwrap()
                .progress_chars("█▓▒░ "),
        );

        let start_time = Instant::now();
        let mut successful = 0;
        let mut failed = 0;

        for stock_row in stocks {
            let stock_id: i64 = stock_row.get("id");
            let symbol: String = stock_row.get("symbol");

            progress_bar.set_message(format!("Processing {}", symbol));

            match self.calculate_ratios_for_stock(stock_id, symbol.clone()).await {
                Ok(ratios) => {
                    match self.update_daily_valuation_ratios(&ratios).await {
                        Ok(_) => {
                            successful += 1;
                            if ratios.pe_ratio.is_some() || ratios.pb_ratio.is_some() || ratios.ev_ebitda_ratio.is_some() || ratios.shareholder_yield.is_some() {
                                println!("✅ {}: P/E={:?}, P/B={:?}, EV/EBITDA={:?}, Yield={:?}%",
                                    symbol,
                                    ratios.pe_ratio.map(|r| format!("{:.2}", r)),
                                    ratios.pb_ratio.map(|r| format!("{:.2}", r)),
                                    ratios.ev_ebitda_ratio.map(|r| format!("{:.2}", r)),
                                    ratios.shareholder_yield.map(|r| format!("{:.1}", r))
                                );
                            }
                        }
                        Err(e) => {
                            failed += 1;
                            eprintln!("❌ Update failed for {}: {}", symbol, e);
                        }
                    }
                }
                Err(e) => {
                    failed += 1;
                    eprintln!("❌ Calculation failed for {}: {}", symbol, e);
                }
            }

            progress_bar.inc(1);
        }

        progress_bar.finish_with_message("Complete!");

        let duration = start_time.elapsed();
        println!("\n🎉 O'Shaughnessy Ratio Calculation Complete!");
        println!("⏱️  Total time: {:.2}s", duration.as_secs_f64());
        println!("✅ Successful: {}", successful);
        println!("❌ Failed: {}", failed);
        println!("📈 Success rate: {:.1}%", (successful as f64 / (successful + failed) as f64) * 100.0);

        Ok(())
    }
}

async fn get_database_connection() -> Result<SqlitePool, String> {
    let database_url = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_PATH"))
        .map_err(|_| "❌ DATABASE_URL or DATABASE_PATH environment variable must be set. No fallback paths allowed.".to_string())?;

    println!("🔗 Connecting to database: {}", database_url);

    SqlitePool::connect(&database_url)
        .await
        .map_err(|e| format!("Database connection failed: {}", e))
}

#[tokio::main]
async fn main() -> Result<(), String> {
    println!("🚀 Starting O'Shaughnessy Ratio Calculator...");

    let pool = get_database_connection().await?;
    let calculator = OShaughnessyRatioCalculator::new(pool);

    calculator.process_all_stocks().await?;

    println!("✅ O'Shaughnessy ratio calculation completed successfully!");

    Ok(())
}