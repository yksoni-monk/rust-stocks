use sqlx::{SqlitePool, Row};
use chrono::{NaiveDate, Utc};
use indicatif::{ProgressBar, ProgressStyle};
use anyhow::{Result, anyhow};

#[derive(Debug)]
pub struct RatioCalculationStats {
    pub stocks_processed: usize,
    pub ps_ratios_calculated: usize,
    pub evs_ratios_calculated: usize,
    pub pb_ratios_calculated: usize,
    pub pcf_ratios_calculated: usize,
    pub market_caps_calculated: usize,
    pub enterprise_values_calculated: usize,
    pub errors: usize,
}

impl Default for RatioCalculationStats {
    fn default() -> Self {
        Self {
            stocks_processed: 0,
            ps_ratios_calculated: 0,
            evs_ratios_calculated: 0,
            pb_ratios_calculated: 0,
            pcf_ratios_calculated: 0,
            market_caps_calculated: 0,
            enterprise_values_calculated: 0,
            errors: 0,
        }
    }
}

#[derive(Debug)]
struct FinancialData {
    stock_id: i64,
    symbol: String,
    latest_ttm_revenue: Option<f64>,
    latest_ttm_report_date: Option<NaiveDate>,
    latest_price: Option<f64>,
    #[allow(dead_code)] // May be needed for future price date validation
    latest_price_date: Option<NaiveDate>,
    shares_outstanding: Option<f64>,
    cash_and_equivalents: Option<f64>,
    total_debt: Option<f64>,
    total_equity: Option<f64>,
    net_income: Option<f64>,
    depreciation_expense: Option<f64>,
    amortization_expense: Option<f64>,
}

#[derive(Debug)]
struct CalculatedRatios {
    market_cap: Option<f64>,
    enterprise_value: Option<f64>,
    ps_ratio_ttm: Option<f64>,
    evs_ratio_ttm: Option<f64>,
    pb_ratio_ttm: Option<f64>,
    pcf_ratio_ttm: Option<f64>,
    data_completeness_score: i32,
}

/// Calculate P/S, EV/S, P/B, and P/CF ratios for all stocks with TTM financial data
pub async fn calculate_ps_evs_pb_pcf_ratios(pool: &SqlitePool) -> Result<RatioCalculationStats> {
    println!("🧮 Starting P/S, EV/S, P/B, and P/CF ratio calculations...");
    
    let financial_data = fetch_financial_data(pool).await?;
    println!("📊 Found {} stocks with financial data", financial_data.len());
    
    if financial_data.is_empty() {
        return Ok(RatioCalculationStats::default());
    }

    let pb = ProgressBar::new(financial_data.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("#>-")
    );
    pb.set_message("Calculating ratios...");

    let mut stats = RatioCalculationStats::default();

    for data in financial_data {
        stats.stocks_processed += 1;
        
        let ratios = calculate_stock_ratios(&data);
        
        // Use the latest price date instead of today's date to avoid creating records for non-existent dates
        let calculation_date = data.latest_price_date.unwrap_or_else(|| Utc::now().date_naive());
        
        match store_calculated_ratios(pool, &data, &ratios, calculation_date).await {
            Ok(stored_stats) => {
                stats.ps_ratios_calculated += stored_stats.ps_ratios_calculated;
                stats.evs_ratios_calculated += stored_stats.evs_ratios_calculated;
                stats.pb_ratios_calculated += stored_stats.pb_ratios_calculated;
                stats.market_caps_calculated += stored_stats.market_caps_calculated;
                stats.enterprise_values_calculated += stored_stats.enterprise_values_calculated;
            }
            Err(e) => {
                eprintln!("Failed to store ratios for {}: {}", data.symbol, e);
                stats.errors += 1;
            }
        }

        pb.inc(1);
        if stats.stocks_processed % 10 == 0 {
            pb.set_message(format!("Processed {} stocks...", stats.stocks_processed));
        }
    }

    pb.finish_with_message("✅ Ratio calculations completed");
    Ok(stats)
}

/// Calculate P/S, EV/S, P/B, and P/CF ratios for ALL historical dates (not just recent)
pub async fn calculate_historical_ps_evs_pb_pcf_ratios(pool: &SqlitePool) -> Result<RatioCalculationStats> {
    println!("🧮 Starting HISTORICAL P/S, EV/S, P/B, and P/CF ratio calculations...");
    
    let historical_data = fetch_historical_financial_data(pool).await?;
    println!("📊 Found {} historical financial records", historical_data.len());
    
    if historical_data.is_empty() {
        return Ok(RatioCalculationStats::default());
    }

    let pb = ProgressBar::new(historical_data.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("#>-")
    );
    pb.set_message("Calculating historical ratios...");

    let mut stats = RatioCalculationStats::default();

    for data in historical_data {
        stats.stocks_processed += 1;
        
        let ratios = calculate_stock_ratios(&data);
        
        match store_calculated_ratios(pool, &data, &ratios, data.latest_ttm_report_date.unwrap()).await {
            Ok(stored_stats) => {
                stats.ps_ratios_calculated += stored_stats.ps_ratios_calculated;
                stats.evs_ratios_calculated += stored_stats.evs_ratios_calculated;
                stats.pb_ratios_calculated += stored_stats.pb_ratios_calculated;
                stats.market_caps_calculated += stored_stats.market_caps_calculated;
                stats.enterprise_values_calculated += stored_stats.enterprise_values_calculated;
            }
            Err(e) => {
                eprintln!("Failed to store ratios for {}: {}", data.symbol, e);
                stats.errors += 1;
            }
        }

        pb.inc(1);
        if stats.stocks_processed % 100 == 0 {
            pb.set_message(format!("Processed {} historical records...", stats.stocks_processed));
        }
    }

    pb.finish_with_message("✅ Historical ratio calculations completed");
    Ok(stats)
}

/// Fetch financial data for all stocks with TTM data
async fn fetch_financial_data(pool: &SqlitePool) -> Result<Vec<FinancialData>> {
    println!("  📊 Fetching financial data...");
    
    let query = r#"
        SELECT DISTINCT
            s.id as stock_id,
            s.symbol,
            
            -- Latest Annual financial data (for O'Shaughnessy methodology)
            (SELECT revenue FROM income_statements i 
             WHERE i.stock_id = s.id AND i.period_type = 'Annual' 
             ORDER BY i.report_date DESC LIMIT 1) as latest_ttm_revenue,
            (SELECT report_date FROM income_statements i 
             WHERE i.stock_id = s.id AND i.period_type = 'Annual' 
             ORDER BY i.report_date DESC LIMIT 1) as latest_ttm_report_date,
             
            -- Latest Annual cash flow data for P/CF calculation
            (SELECT net_income FROM income_statements i 
             WHERE i.stock_id = s.id AND i.period_type = 'Annual' 
             ORDER BY i.report_date DESC LIMIT 1) as net_income,
            (SELECT depreciation_expense FROM cash_flow_statements c 
             WHERE c.stock_id = s.id AND c.period_type = 'Annual' 
             ORDER BY c.report_date DESC LIMIT 1) as depreciation_expense,
            (SELECT amortization_expense FROM cash_flow_statements c 
             WHERE c.stock_id = s.id AND c.period_type = 'Annual' 
             ORDER BY c.report_date DESC LIMIT 1) as amortization_expense,
             
            -- Latest stock price data
            (SELECT close_price FROM daily_prices dp 
             WHERE dp.stock_id = s.id 
             ORDER BY dp.date DESC LIMIT 1) as latest_price,
            (SELECT date FROM daily_prices dp 
             WHERE dp.stock_id = s.id 
             ORDER BY dp.date DESC LIMIT 1) as latest_price_date,
            (SELECT shares_basic FROM income_statements i 
             WHERE i.stock_id = s.id AND i.shares_basic IS NOT NULL AND i.shares_basic > 0
             ORDER BY 
                 CASE WHEN i.data_source = 'sec_edgar_json' THEN 0 ELSE 1 END,
                 i.report_date DESC 
             LIMIT 1) as shares_outstanding,
             
            -- Latest balance sheet data for EV and P/B calculation
            (SELECT cash_and_equivalents FROM balance_sheets b
             WHERE b.stock_id = s.id AND b.period_type IN ('TTM', 'Annual')
             ORDER BY 
                 CASE WHEN b.data_source = 'sec_edgar_json' THEN 0 ELSE 1 END,
                 b.report_date DESC 
             LIMIT 1) as cash_and_equivalents,
            (SELECT total_debt FROM balance_sheets b
             WHERE b.stock_id = s.id AND b.period_type IN ('TTM', 'Annual')
             ORDER BY 
                 CASE WHEN b.data_source = 'sec_edgar_json' THEN 0 ELSE 1 END,
                 b.report_date DESC 
             LIMIT 1) as total_debt,
            (SELECT total_equity FROM balance_sheets b
             WHERE b.stock_id = s.id AND b.period_type IN ('TTM', 'Annual')
             ORDER BY 
                 CASE WHEN b.data_source = 'sec_edgar_json' THEN 0 ELSE 1 END,
                 b.report_date DESC 
             LIMIT 1) as total_equity
        FROM stocks s
        WHERE s.id IN (
            SELECT DISTINCT stock_id FROM balance_sheets 
            WHERE total_equity IS NOT NULL AND total_equity > 0
        )
        AND s.id IN (
            SELECT DISTINCT stock_id FROM income_statements 
            WHERE shares_basic IS NOT NULL AND shares_basic > 0
        )
        AND s.id IN (
            SELECT DISTINCT stock_id FROM income_statements 
            WHERE period_type = 'Annual' AND net_income IS NOT NULL
        )
        AND s.id IN (
            SELECT DISTINCT stock_id FROM cash_flow_statements 
            WHERE period_type = 'Annual' AND (depreciation_expense IS NOT NULL OR amortization_expense IS NOT NULL)
        )
        ORDER BY s.symbol
    "#;

    let rows = sqlx::query(query).fetch_all(pool).await?;
    
    let mut financial_data = Vec::new();
    for row in rows {
        let data = FinancialData {
            stock_id: row.get("stock_id"),
            symbol: row.get("symbol"),
            latest_ttm_revenue: row.get("latest_ttm_revenue"),
            latest_ttm_report_date: row.get("latest_ttm_report_date"),
            latest_price: row.get("latest_price"),
            latest_price_date: row.get("latest_price_date"),
            shares_outstanding: row.get("shares_outstanding"),
            cash_and_equivalents: row.get("cash_and_equivalents"),
            total_debt: row.get("total_debt"),
            total_equity: row.get("total_equity"),
            net_income: row.get("net_income"),
            depreciation_expense: row.get("depreciation_expense"),
            amortization_expense: row.get("amortization_expense"),
        };
        financial_data.push(data);
    }
    
    println!("  ✅ Found {} stocks with balance sheet data", financial_data.len());
    Ok(financial_data)
}

/// Fetch ALL historical financial data for ratio calculations (not just latest)
async fn fetch_historical_financial_data(pool: &SqlitePool) -> Result<Vec<FinancialData>> {
    println!("  📊 Fetching ALL historical financial data...");
    
    let query = r#"
        SELECT 
            s.id as stock_id,
            s.symbol,
            i.revenue as latest_ttm_revenue,
            i.report_date as latest_ttm_report_date,
            
            -- Get price data closest to the TTM report date
            (SELECT close_price FROM daily_prices dp 
             WHERE dp.stock_id = s.id AND dp.date <= i.report_date
             ORDER BY dp.date DESC LIMIT 1) as latest_price,
            (SELECT date FROM daily_prices dp 
             WHERE dp.stock_id = s.id AND dp.date <= i.report_date
             ORDER BY dp.date DESC LIMIT 1) as latest_price_date,
            i.shares_diluted as shares_outstanding,
             
            -- Get balance sheet data closest to the TTM report date (prioritize SEC EDGAR data)
            (SELECT cash_and_equivalents FROM balance_sheets b
             WHERE b.stock_id = s.id AND b.period_type IN ('TTM', 'Annual')
             AND b.report_date <= i.report_date
             ORDER BY 
                 CASE WHEN b.data_source = 'sec_edgar_json' THEN 0 ELSE 1 END,
                 b.report_date DESC 
             LIMIT 1) as cash_and_equivalents,
            (SELECT total_debt FROM balance_sheets b
             WHERE b.stock_id = s.id AND b.period_type IN ('TTM', 'Annual')
             AND b.report_date <= i.report_date
             ORDER BY 
                 CASE WHEN b.data_source = 'sec_edgar_json' THEN 0 ELSE 1 END,
                 b.report_date DESC 
             LIMIT 1) as total_debt,
            (SELECT total_equity FROM balance_sheets b
             WHERE b.stock_id = s.id AND b.period_type IN ('TTM', 'Annual')
             AND b.report_date <= i.report_date
             ORDER BY 
                 CASE WHEN b.data_source = 'sec_edgar_json' THEN 0 ELSE 1 END,
                 b.report_date DESC 
             LIMIT 1) as total_equity
        FROM stocks s
        JOIN income_statements i ON s.id = i.stock_id
        WHERE i.period_type = 'TTM' 
        AND i.revenue IS NOT NULL 
        AND i.revenue > 0
        ORDER BY s.symbol, i.report_date DESC
    "#;

    let rows = sqlx::query(query).fetch_all(pool).await?;
    
    let mut financial_data = Vec::new();
    for row in rows {
        let data = FinancialData {
            stock_id: row.get("stock_id"),
            symbol: row.get("symbol"),
            latest_ttm_revenue: row.get("latest_ttm_revenue"),
            latest_ttm_report_date: row.get("latest_ttm_report_date"),
            latest_price: row.get("latest_price"),
            latest_price_date: row.get("latest_price_date"),
            shares_outstanding: row.get("shares_outstanding"),
            cash_and_equivalents: row.get("cash_and_equivalents"),
            total_debt: row.get("total_debt"),
            total_equity: row.get("total_equity"),
            net_income: row.get("net_income"),
            depreciation_expense: row.get("depreciation_expense"),
            amortization_expense: row.get("amortization_expense"),
        };
        financial_data.push(data);
    }
    
    println!("  ✅ Found {} historical TTM financial records", financial_data.len());
    Ok(financial_data)
}

/// Calculate ratios for a single stock
fn calculate_stock_ratios(data: &FinancialData) -> CalculatedRatios {
    let mut ratios = CalculatedRatios {
        market_cap: None,
        enterprise_value: None,
        ps_ratio_ttm: None,
        evs_ratio_ttm: None,
        pb_ratio_ttm: None,
        pcf_ratio_ttm: None,
        data_completeness_score: 0,
    };

    // Calculate Market Cap = Stock Price × Shares Outstanding
    if let (Some(price), Some(shares)) = (data.latest_price, data.shares_outstanding) {
        if price > 0.0 && shares > 0.0 {
            ratios.market_cap = Some(price * shares);
            ratios.data_completeness_score += 25; // 25 points for market cap
        }
    }

    // Calculate Enterprise Value = Market Cap + Total Debt - Cash
    if let Some(market_cap) = ratios.market_cap {
        let debt = data.total_debt.unwrap_or(0.0);
        let cash = data.cash_and_equivalents.unwrap_or(0.0);
        ratios.enterprise_value = Some(market_cap + debt - cash);
        ratios.data_completeness_score += 25; // 25 points for enterprise value
    }

    // Calculate P/S Ratio = Market Cap / TTM Revenue
    if let (Some(market_cap), Some(revenue)) = (ratios.market_cap, data.latest_ttm_revenue) {
        if revenue > 0.0 {
            ratios.ps_ratio_ttm = Some(market_cap / revenue);
            ratios.data_completeness_score += 20; // 20 points for P/S ratio
        }
    }

    // Calculate EV/S Ratio = Enterprise Value / TTM Revenue
    if let (Some(enterprise_value), Some(revenue)) = (ratios.enterprise_value, data.latest_ttm_revenue) {
        if revenue > 0.0 {
            ratios.evs_ratio_ttm = Some(enterprise_value / revenue);
            ratios.data_completeness_score += 20; // 20 points for EV/S ratio
        }
    }

    // Calculate P/B Ratio = Market Cap / Total Equity (Book Value)
    if let (Some(market_cap), Some(equity)) = (ratios.market_cap, data.total_equity) {
        if equity > 0.0 {
            ratios.pb_ratio_ttm = Some(market_cap / equity);
            ratios.data_completeness_score += 20; // 20 points for P/B ratio
        }
    }

    // Calculate P/CF Ratio = Market Cap / Cash Flow
    // Cash Flow = Net Income + Depreciation + Amortization
    if let Some(market_cap) = ratios.market_cap {
        let net_income = data.net_income.unwrap_or(0.0);
        let depreciation = data.depreciation_expense.unwrap_or(0.0);
        let amortization = data.amortization_expense.unwrap_or(0.0);
        let cash_flow = net_income + depreciation + amortization;
        
        if cash_flow > 0.0 {
            ratios.pcf_ratio_ttm = Some(market_cap / cash_flow);
            ratios.data_completeness_score += 20; // 20 points for P/CF ratio
        }
    }

    ratios
}

/// Store calculated ratios in the database
async fn store_calculated_ratios(
    pool: &SqlitePool,
    data: &FinancialData,
    ratios: &CalculatedRatios,
    calculation_date: NaiveDate,
) -> Result<RatioCalculationStats> {
    let mut stats = RatioCalculationStats::default();

    // Insert or update daily_valuation_ratios
    let result = sqlx::query(
        r#"
        INSERT OR REPLACE INTO daily_valuation_ratios (
            stock_id, date, price, market_cap, enterprise_value,
            ps_ratio_ttm, evs_ratio_ttm, pb_ratio_ttm, pcf_ratio_ttm, revenue_ttm,
            data_completeness_score, last_financial_update
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12
        )
        "#
    )
    .bind(data.stock_id)
    .bind(calculation_date)
    .bind(data.latest_price)
    .bind(ratios.market_cap)
    .bind(ratios.enterprise_value)
    .bind(ratios.ps_ratio_ttm)
    .bind(ratios.evs_ratio_ttm)
    .bind(ratios.pb_ratio_ttm)
    .bind(ratios.pcf_ratio_ttm)
    .bind(data.latest_ttm_revenue)
    .bind(ratios.data_completeness_score)
    .bind(data.latest_ttm_report_date)
    .execute(pool)
    .await;

    match result {
        Ok(_) => {
            if ratios.ps_ratio_ttm.is_some() { stats.ps_ratios_calculated = 1; }
            if ratios.evs_ratio_ttm.is_some() { stats.evs_ratios_calculated = 1; }
            if ratios.pb_ratio_ttm.is_some() { stats.pb_ratios_calculated = 1; }
            if ratios.pcf_ratio_ttm.is_some() { stats.pcf_ratios_calculated = 1; }
            if ratios.market_cap.is_some() { stats.market_caps_calculated = 1; }
            if ratios.enterprise_value.is_some() { stats.enterprise_values_calculated = 1; }
        }
        Err(e) => return Err(anyhow!("Failed to store ratios: {}", e)),
    }

    Ok(stats)
}

/// Calculate P/S and EV/S ratios for stocks with negative earnings (where P/E is invalid)
pub async fn calculate_ratios_for_negative_earnings_stocks(pool: &SqlitePool) -> Result<RatioCalculationStats> {
    println!("🔍 Identifying stocks with negative earnings where P/E ratios are invalid...");
    
    // First, identify stocks with negative net income in their latest TTM data
    let negative_earnings_query = r#"
        SELECT s.id, s.symbol, i.net_income
        FROM stocks s
        JOIN income_statements i ON s.id = i.stock_id
        WHERE i.period_type = 'TTM'
        AND i.net_income < 0
        AND i.report_date = (
            SELECT MAX(report_date) 
            FROM income_statements i2 
            WHERE i2.stock_id = s.id AND i2.period_type = 'TTM'
        )
        ORDER BY s.symbol
    "#;
    
    let negative_earnings_stocks = sqlx::query(negative_earnings_query)
        .fetch_all(pool)
        .await?;
        
    if negative_earnings_stocks.is_empty() {
        println!("✅ No stocks found with negative earnings in current dataset");
        return Ok(RatioCalculationStats::default());
    }
    
    println!("📊 Found {} stocks with negative earnings where P/E ratios are invalid:", negative_earnings_stocks.len());
    for row in &negative_earnings_stocks {
        let symbol: String = row.get("symbol");
        let net_income: f64 = row.get("net_income");
        println!("  🔴 {} (Net Income: ${:.1}M)", symbol, net_income / 1_000_000.0);
    }
    
    // Calculate P/S, EV/S, P/B, and P/CF ratios for these stocks
    println!("💡 Calculating P/S, EV/S, P/B, and P/CF ratios for negative earnings stocks...");
    calculate_ps_evs_pb_pcf_ratios(pool).await
}

/// Generate ratio calculation summary report
pub async fn generate_ratio_summary_report(pool: &SqlitePool) -> Result<()> {
    println!("\n📊 RATIO CALCULATION SUMMARY REPORT");
    println!("{}", "=".repeat(60));
    
    // Count total ratios calculated
    let ratios_count = sqlx::query(
        "SELECT 
            COUNT(*) as total_records,
            COUNT(ps_ratio_ttm) as ps_ratios,
            COUNT(evs_ratio_ttm) as evs_ratios,
            COUNT(pb_ratio_ttm) as pb_ratios,
            COUNT(market_cap) as market_caps,
            COUNT(enterprise_value) as enterprise_values,
            AVG(data_completeness_score) as avg_completeness
        FROM daily_valuation_ratios"
    )
    .fetch_one(pool)
    .await?;
    
    let total_records: i64 = ratios_count.get("total_records");
    let ps_ratios: i64 = ratios_count.get("ps_ratios");
    let evs_ratios: i64 = ratios_count.get("evs_ratios");
    let pb_ratios: i64 = ratios_count.get("pb_ratios");
    let market_caps: i64 = ratios_count.get("market_caps");
    let enterprise_values: i64 = ratios_count.get("enterprise_values");
    let avg_completeness: Option<f64> = ratios_count.get("avg_completeness");
    
    println!("📈 CALCULATION RESULTS:");
    println!("  Total Records: {}", total_records);
    println!("  P/S Ratios Calculated: {}", ps_ratios);
    println!("  EV/S Ratios Calculated: {}", evs_ratios);
    println!("  P/B Ratios Calculated: {}", pb_ratios);
    println!("  Market Caps Calculated: {}", market_caps);
    println!("  Enterprise Values Calculated: {}", enterprise_values);
    println!("  Average Data Completeness: {:.1}%", avg_completeness.unwrap_or(0.0));
    
    // Show sample ratios
    println!("\n💰 SAMPLE P/S AND EV/S RATIOS:");
    let sample_ratios = sqlx::query(
        r#"
        SELECT s.symbol, dvr.ps_ratio_ttm, dvr.evs_ratio_ttm, dvr.market_cap, dvr.revenue_ttm
        FROM daily_valuation_ratios dvr
        JOIN stocks s ON s.id = dvr.stock_id
        WHERE dvr.ps_ratio_ttm IS NOT NULL
        ORDER BY s.symbol
        LIMIT 10
        "#
    )
    .fetch_all(pool)
    .await?;
    
    for row in sample_ratios {
        let symbol: String = row.get("symbol");
        let ps_ratio: Option<f64> = row.get("ps_ratio_ttm");
        let evs_ratio: Option<f64> = row.get("evs_ratio_ttm");
        let market_cap: Option<f64> = row.get("market_cap");
        let revenue: Option<f64> = row.get("revenue_ttm");
        
        println!("  {} - P/S: {:.2}, EV/S: {:.2}, Market Cap: ${:.1}B, Revenue: ${:.1}B", 
            symbol,
            ps_ratio.unwrap_or(0.0),
            evs_ratio.unwrap_or(0.0),
            market_cap.unwrap_or(0.0) / 1_000_000_000.0,
            revenue.unwrap_or(0.0) / 1_000_000_000.0
        );
    }
    
    println!("{}", "=".repeat(60));
    println!("✅ P/S and EV/S ratio system is now operational!");
    println!("💡 Use these ratios for value investing analysis when P/E ratios are invalid");
    
    Ok(())
}