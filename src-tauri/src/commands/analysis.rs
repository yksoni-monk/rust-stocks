use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceData {
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: i64,
    pub pe_ratio: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRangeInfo {
    pub symbol: String,
    pub earliest_date: String,
    pub latest_date: String,
    pub total_records: i64,
    pub data_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValuationRatios {
    pub stock_id: i64,
    pub symbol: String,
    pub date: String,
    pub price: Option<f64>,
    pub market_cap: Option<f64>,
    pub enterprise_value: Option<f64>,
    pub ps_ratio_ttm: Option<f64>,
    pub evs_ratio_ttm: Option<f64>,
    pub revenue_ttm: Option<f64>,
    pub data_completeness_score: i32,
    pub last_financial_update: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValuationExtremes {
    pub symbol: String,
    pub min_pe_ratio: Option<f64>,
    pub max_pe_ratio: Option<f64>,
    pub min_ps_ratio: Option<f64>,
    pub max_ps_ratio: Option<f64>,
    pub min_evs_ratio: Option<f64>,
    pub max_evs_ratio: Option<f64>,
}

async fn get_database_connection() -> Result<SqlitePool, String> {
    let database_url = "sqlite:db/stocks.db";
    SqlitePool::connect(database_url).await
        .map_err(|e| format!("Database connection failed: {}", e))
}

#[tauri::command]
pub async fn get_price_history(symbol: String, start_date: String, end_date: String) -> Result<Vec<PriceData>, String> {
    let pool = get_database_connection().await?;
    
    // Validate date format but use as strings since database stores DATE format
    chrono::NaiveDate::parse_from_str(&start_date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid start date format: {}", e))?;
    
    chrono::NaiveDate::parse_from_str(&end_date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid end date format: {}", e))?;
    
    let query = "
        SELECT dp.date, dp.open_price, dp.high_price, dp.low_price, dp.close_price, dp.volume, dp.pe_ratio 
        FROM daily_prices dp
        JOIN stocks s ON dp.stock_id = s.id
        WHERE s.symbol = ?1 AND dp.date BETWEEN ?2 AND ?3 
        ORDER BY dp.date ASC
        LIMIT 1000
    ";
    
    match sqlx::query(query)
        .bind(&symbol)
        .bind(&start_date)
        .bind(&end_date)
        .fetch_all(&pool).await 
    {
        Ok(rows) => {
            let price_data: Vec<PriceData> = rows.into_iter().map(|row| {
                // Date is stored as DATE string in database, not timestamp
                let date_string: String = row.get("date");
                
                PriceData {
                    date: date_string,
                    open: row.get::<f64, _>("open_price"),
                    high: row.get::<f64, _>("high_price"),
                    low: row.get::<f64, _>("low_price"),
                    close: row.get::<f64, _>("close_price"),
                    volume: row.try_get::<Option<i64>, _>("volume").unwrap_or(None).unwrap_or(0),
                    pe_ratio: row.try_get::<Option<f64>, _>("pe_ratio").unwrap_or(None),
                }
            }).collect();
            
            Ok(price_data)
        }
        Err(e) => {
            eprintln!("Price history query error: {}", e);
            Err(format!("Database query failed: {}", e))
        }
    }
}

#[tauri::command]
pub async fn get_stock_date_range(symbol: String) -> Result<DateRangeInfo, String> {
    let pool = get_database_connection().await?;
    
    let result = sqlx::query("
        SELECT s.symbol, MIN(dp.date) as earliest_date, MAX(dp.date) as latest_date, 
               COUNT(*) as total_records, COALESCE(dp.data_source, 'simfin') as data_source
        FROM daily_prices dp
        JOIN stocks s ON dp.stock_id = s.id
        WHERE s.symbol = ?1
        GROUP BY s.symbol, dp.data_source")
        .bind(&symbol)
        .fetch_optional(&pool).await;
    
    match result {
        Ok(Some(row)) => {
            // Convert date strings to proper format
            let earliest_date: String = row.get("earliest_date");
            let latest_date: String = row.get("latest_date");
            
            Ok(DateRangeInfo {
                symbol: row.get("symbol"),
                earliest_date,
                latest_date,
                total_records: row.get("total_records"),
                data_source: row.get("data_source"),
            })
        }
        Ok(None) => {
            Err(format!("No data found for symbol: {}", symbol))
        }
        Err(e) => {
            Err(format!("Database error: {}", e))
        }
    }
}

#[tauri::command]
pub async fn get_valuation_ratios(symbol: String) -> Result<Option<ValuationRatios>, String> {
    let pool = get_database_connection().await?;
    
    let query = "
        SELECT 
            dvr.stock_id,
            s.symbol,
            dvr.date,
            dvr.price,
            dvr.market_cap,
            dvr.enterprise_value,
            dvr.ps_ratio_ttm,
            dvr.evs_ratio_ttm,
            dvr.revenue_ttm,
            dvr.data_completeness_score,
            dvr.last_financial_update
        FROM daily_valuation_ratios dvr
        JOIN stocks s ON dvr.stock_id = s.id
        WHERE s.symbol = ?1
        ORDER BY dvr.date DESC
        LIMIT 1
    ";
    
    match sqlx::query(query)
        .bind(&symbol)
        .fetch_optional(&pool).await 
    {
        Ok(Some(row)) => {
            let ratios = ValuationRatios {
                stock_id: row.get("stock_id"),
                symbol: row.get("symbol"),
                date: row.get("date"),
                price: row.get("price"),
                market_cap: row.get("market_cap"),
                enterprise_value: row.get("enterprise_value"),
                ps_ratio_ttm: row.get("ps_ratio_ttm"),
                evs_ratio_ttm: row.get("evs_ratio_ttm"),
                revenue_ttm: row.get("revenue_ttm"),
                data_completeness_score: row.get("data_completeness_score"),
                last_financial_update: row.get("last_financial_update"),
            };
            Ok(Some(ratios))
        }
        Ok(None) => Ok(None),
        Err(e) => {
            eprintln!("Valuation ratios query error: {}", e);
            Err(format!("Database query failed: {}", e))
        }
    }
}

#[tauri::command]
pub async fn get_ps_evs_history(symbol: String, start_date: String, end_date: String) -> Result<Vec<ValuationRatios>, String> {
    let pool = get_database_connection().await?;
    
    // Validate date format
    chrono::NaiveDate::parse_from_str(&start_date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid start date format: {}", e))?;
    
    chrono::NaiveDate::parse_from_str(&end_date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid end date format: {}", e))?;
    
    let query = "
        SELECT 
            dvr.stock_id,
            s.symbol,
            dvr.date,
            dvr.price,
            dvr.market_cap,
            dvr.enterprise_value,
            dvr.ps_ratio_ttm,
            dvr.evs_ratio_ttm,
            dvr.revenue_ttm,
            dvr.data_completeness_score,
            dvr.last_financial_update
        FROM daily_valuation_ratios dvr
        JOIN stocks s ON dvr.stock_id = s.id
        WHERE s.symbol = ?1 AND dvr.date BETWEEN ?2 AND ?3
        ORDER BY dvr.date ASC
        LIMIT 1000
    ";
    
    match sqlx::query(query)
        .bind(&symbol)
        .bind(&start_date)
        .bind(&end_date)
        .fetch_all(&pool).await 
    {
        Ok(rows) => {
            let ratios_data: Vec<ValuationRatios> = rows.into_iter().map(|row| {
                ValuationRatios {
                    stock_id: row.get("stock_id"),
                    symbol: row.get("symbol"),
                    date: row.get("date"),
                    price: row.get("price"),
                    market_cap: row.get("market_cap"),
                    enterprise_value: row.get("enterprise_value"),
                    ps_ratio_ttm: row.get("ps_ratio_ttm"),
                    evs_ratio_ttm: row.get("evs_ratio_ttm"),
                    revenue_ttm: row.get("revenue_ttm"),
                    data_completeness_score: row.get("data_completeness_score"),
                    last_financial_update: row.get("last_financial_update"),
                }
            }).collect();
            
            Ok(ratios_data)
        }
        Err(e) => {
            eprintln!("P/S EV/S history query error: {}", e);
            Err(format!("Database query failed: {}", e))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SmartUndervaluedStock {
    pub stock_id: i32,
    pub symbol: String,
    pub current_ps: f64,
    pub historical_mean: f64,
    pub historical_median: f64,
    pub historical_min: f64,
    pub historical_max: f64,
    pub historical_variance: f64,
    pub z_score: f64,
    pub is_undervalued: bool,
    pub market_cap: f64,
    pub price: f64,
    pub data_completeness_score: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PsRevenueGrowthStock {
    pub stock_id: i32,
    pub symbol: String,
    pub current_ps: f64,
    // Historical P/S statistics
    pub historical_mean: f64,
    pub historical_median: f64,
    pub historical_stddev: f64,
    pub historical_min: f64,
    pub historical_max: f64,
    pub data_points: i32,
    // Revenue growth metrics
    pub current_ttm_revenue: Option<f64>,
    pub ttm_growth_rate: Option<f64>,
    pub current_annual_revenue: Option<f64>,
    pub annual_growth_rate: Option<f64>,
    // Screening criteria
    pub z_score: f64,
    pub quality_score: i32,
    pub undervalued_flag: bool,
    // Market metrics
    pub market_cap: f64,
    pub price: f64,
    pub data_completeness_score: i32,
}

#[tauri::command]
pub async fn get_undervalued_stocks_by_ps(
    stock_tickers: Vec<String>, 
    limit: Option<i32>, 
    min_market_cap: Option<f64>
) -> Result<Vec<SmartUndervaluedStock>, String> {
    let pool = get_database_connection().await?;
    let limit_value = limit.unwrap_or(50);
    let min_market_cap_value = min_market_cap.unwrap_or(500_000_000.0); // Default $500M
    
    if stock_tickers.is_empty() {
        return Ok(vec![]);
    }
    
    // Create placeholders for the IN clause
    let placeholders = stock_tickers.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    
    // Smart P/S screening algorithm - calculate everything on-the-fly
    let query = format!("
        WITH sp500_stocks AS (
            SELECT s.id, s.symbol
            FROM stocks s
            WHERE s.symbol IN ({})
        ),
        historical_ps_data AS (
            SELECT 
                s.id as stock_id,
                s.symbol,
                dvr.ps_ratio_ttm,
                dvr.date,
                dvr.price,
                dvr.market_cap,
                dvr.data_completeness_score,
                ROW_NUMBER() OVER (PARTITION BY s.id ORDER BY dvr.date DESC) as rn
            FROM sp500_stocks s
            JOIN daily_valuation_ratios dvr ON s.id = dvr.stock_id
            WHERE dvr.ps_ratio_ttm IS NOT NULL 
              AND dvr.ps_ratio_ttm > 0.01
              AND dvr.market_cap > ?
        ),
        current_data AS (
            SELECT * FROM historical_ps_data WHERE rn = 1
        ),
        historical_stats AS (
            SELECT 
                stock_id,
                AVG(ps_ratio_ttm) as hist_mean,
                MIN(ps_ratio_ttm) as hist_min,
                MAX(ps_ratio_ttm) as hist_max,
                COUNT(*) as data_points
            FROM historical_ps_data 
            WHERE rn > 1  -- Exclude current data point for historical analysis
            GROUP BY stock_id
            HAVING COUNT(*) >= 20  -- Require at least 20 historical data points (roughly 1 month)
        ),
        variance_calc AS (
            SELECT 
                h.stock_id,
                AVG((s.ps_ratio_ttm - h.hist_mean) * (s.ps_ratio_ttm - h.hist_mean)) as hist_variance
            FROM historical_ps_data s
            JOIN historical_stats h ON s.stock_id = h.stock_id
            WHERE s.rn > 1  -- Exclude current data point
            GROUP BY h.stock_id
        ),
        median_calc AS (
            SELECT 
                stock_id,
                ps_ratio_ttm,
                ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY ps_ratio_ttm) as rn,
                COUNT(*) OVER (PARTITION BY stock_id) as total_count
            FROM historical_ps_data 
            WHERE rn > 1  -- Exclude current data point
        ),
        median_data AS (
            SELECT 
                stock_id,
                AVG(ps_ratio_ttm) as hist_median
            FROM median_calc
            WHERE rn IN ((total_count + 1) / 2, (total_count + 2) / 2)
            GROUP BY stock_id
        ),
        market_mean AS (
            SELECT AVG(ps_ratio_ttm) as market_mean FROM current_data
        ),
        market_variance AS (
            SELECT 
                AVG((c.ps_ratio_ttm - m.market_mean) * (c.ps_ratio_ttm - m.market_mean)) as market_variance
            FROM current_data c
            CROSS JOIN market_mean m
        )
        SELECT 
            c.stock_id,
            c.symbol,
            c.ps_ratio_ttm as current_ps,
            COALESCE(h.hist_mean, 0.0) as historical_mean,
            COALESCE(m.hist_median, 0.0) as historical_median,
            COALESCE(h.hist_min, 0.0) as historical_min,
            COALESCE(h.hist_max, 0.0) as historical_max,
            COALESCE(v.hist_variance, 0.0) as historical_variance,
            CASE 
                WHEN v.hist_variance > 0 THEN (c.ps_ratio_ttm - h.hist_mean) / v.hist_variance
                ELSE 0.0
            END as z_score,
            CASE 
                WHEN h.hist_mean > 0 AND v.hist_variance > 0 AND h.data_points >= 20 THEN
                    -- Stock is undervalued if current P/S is significantly below historical mean
                    -- Using a simple threshold: current P/S < mean - 0.5 * variance
                    c.ps_ratio_ttm < (h.hist_mean - 0.5 * v.hist_variance) AND
                    -- And also below historical median
                    c.ps_ratio_ttm < m.hist_median
                ELSE false
            END as is_undervalued,
            c.market_cap,
            c.price,
            c.data_completeness_score
        FROM current_data c
        LEFT JOIN historical_stats h ON c.stock_id = h.stock_id
        LEFT JOIN variance_calc v ON c.stock_id = v.stock_id
        LEFT JOIN median_data m ON c.stock_id = m.stock_id
        CROSS JOIN market_mean mm
        CROSS JOIN market_variance mv
        WHERE c.market_cap > ?
        ORDER BY 
            CASE 
                WHEN h.hist_mean > 0 AND v.hist_variance > 0 AND h.data_points >= 20 THEN
                    c.ps_ratio_ttm < (h.hist_mean - 0.5 * v.hist_variance) AND
                    c.ps_ratio_ttm < m.hist_median
                ELSE false
            END DESC,
            c.ps_ratio_ttm ASC
        LIMIT ?
    ", placeholders);
    
    let mut query_builder = sqlx::query_as::<_, SmartUndervaluedStock>(&query);
    
    // Bind stock tickers
    for ticker in &stock_tickers {
        query_builder = query_builder.bind(ticker);
    }
    
    // Bind min market cap (used twice in the query)
    query_builder = query_builder.bind(min_market_cap_value);
    query_builder = query_builder.bind(min_market_cap_value);
    query_builder = query_builder.bind(limit_value);
    
    match query_builder.fetch_all(&pool).await {
        Ok(stocks) => {
            // Filter to only return truly undervalued stocks
            let undervalued_stocks: Vec<SmartUndervaluedStock> = stocks
                .into_iter()
                .filter(|stock| stock.is_undervalued)
                .take(limit_value as usize)
                .collect();
            
            Ok(undervalued_stocks)
        }
        Err(e) => {
            eprintln!("Smart undervalued stocks query error: {}", e);
            Err(format!("Database query failed: {}", e))
        }
    }
}

#[tauri::command]
pub async fn get_ps_screening_with_revenue_growth(
    stock_tickers: Vec<String>, 
    limit: Option<i32>, 
    min_market_cap: Option<f64>
) -> Result<Vec<PsRevenueGrowthStock>, String> {
    let pool = get_database_connection().await?;
    let limit_value = limit.unwrap_or(50);
    let min_market_cap_value = min_market_cap.unwrap_or(500_000_000.0); // Default $500M
    
    if stock_tickers.is_empty() {
        return Ok(vec![]);
    }
    
    // Create placeholders for the IN clause
    let placeholders = stock_tickers.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    
    // P/S screening with revenue growth algorithm
    let query = format!("
        WITH sp500_stocks AS (
            SELECT s.id, s.symbol
            FROM stocks s
            WHERE s.symbol IN ({})
        ),
        historical_ps_data AS (
            SELECT 
                s.id as stock_id,
                s.symbol,
                dvr.ps_ratio_ttm,
                dvr.date,
                dvr.price,
                dvr.market_cap,
                dvr.data_completeness_score,
                ROW_NUMBER() OVER (PARTITION BY s.id ORDER BY dvr.date DESC) as rn
            FROM sp500_stocks s
            JOIN daily_valuation_ratios dvr ON s.id = dvr.stock_id
            WHERE dvr.ps_ratio_ttm IS NOT NULL 
              AND dvr.ps_ratio_ttm > 0.01
              AND dvr.market_cap > ?
        ),
        current_data AS (
            SELECT * FROM historical_ps_data WHERE rn = 1
        ),
        historical_stats AS (
            SELECT 
                stock_id,
                AVG(ps_ratio_ttm) as hist_mean,
                MIN(ps_ratio_ttm) as hist_min,
                MAX(ps_ratio_ttm) as hist_max,
                COUNT(*) as data_points
            FROM historical_ps_data 
            WHERE rn > 1  -- Exclude current data point for historical analysis
            GROUP BY stock_id
            HAVING COUNT(*) >= 10  -- Require at least 10 historical data points
        ),
        variance_calc AS (
            SELECT 
                h.stock_id,
                AVG((s.ps_ratio_ttm - h.hist_mean) * (s.ps_ratio_ttm - h.hist_mean)) as hist_variance
            FROM historical_ps_data s
            JOIN historical_stats h ON s.stock_id = h.stock_id
            WHERE s.rn > 1  -- Exclude current data point
            GROUP BY h.stock_id
        ),
        median_calc AS (
            SELECT 
                stock_id,
                ps_ratio_ttm,
                ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY ps_ratio_ttm) as rn,
                COUNT(*) OVER (PARTITION BY stock_id) as total_count
            FROM historical_ps_data 
            WHERE rn > 1  -- Exclude current data point
        ),
        median_data AS (
            SELECT 
                stock_id,
                AVG(ps_ratio_ttm) as hist_median
            FROM median_calc
            WHERE rn IN ((total_count + 1) / 2, (total_count + 2) / 2)
            GROUP BY stock_id
        ),
        stddev_calc AS (
            SELECT 
                h.stock_id,
                v.hist_variance as hist_stddev
            FROM historical_stats h
            JOIN variance_calc v ON h.stock_id = v.stock_id
        ),
        -- Revenue data for TTM growth (simplified)
        ttm_growth AS (
            SELECT 
                c.stock_id,
                current_ttm.revenue as current_ttm_revenue,
                CASE 
                    WHEN prev_ttm.revenue > 0 THEN 
                        ((current_ttm.revenue - prev_ttm.revenue) / prev_ttm.revenue) * 100
                    ELSE NULL
                END as ttm_growth_rate
            FROM current_data c
            LEFT JOIN (
                SELECT stock_id, revenue, ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
                FROM income_statements 
                WHERE period_type = 'TTM'
            ) current_ttm ON c.stock_id = current_ttm.stock_id AND current_ttm.rn = 1
            LEFT JOIN (
                SELECT stock_id, revenue, ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
                FROM income_statements 
                WHERE period_type = 'TTM'
            ) prev_ttm ON c.stock_id = prev_ttm.stock_id AND prev_ttm.rn = 2
        ),
        -- Revenue data for Annual growth (simplified)
        annual_growth AS (
            SELECT 
                c.stock_id,
                current_annual.revenue as current_annual_revenue,
                CASE 
                    WHEN prev_annual.revenue > 0 THEN 
                        ((current_annual.revenue - prev_annual.revenue) / prev_annual.revenue) * 100
                    ELSE NULL
                END as annual_growth_rate
            FROM current_data c
            LEFT JOIN (
                SELECT stock_id, revenue, ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY fiscal_year DESC) as rn
                FROM income_statements 
                WHERE period_type = 'Annual'
            ) current_annual ON c.stock_id = current_annual.stock_id AND current_annual.rn = 1
            LEFT JOIN (
                SELECT stock_id, revenue, ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY fiscal_year DESC) as rn
                FROM income_statements 
                WHERE period_type = 'Annual'
            ) prev_annual ON c.stock_id = prev_annual.stock_id AND prev_annual.rn = 2
        )
        SELECT 
            c.stock_id,
            c.symbol,
            c.ps_ratio_ttm as current_ps,
            COALESCE(h.hist_mean, 0.0) as historical_mean,
            COALESCE(m.hist_median, 0.0) as historical_median,
            COALESCE(s.hist_stddev, 0.0) as historical_stddev,
            COALESCE(h.hist_min, 0.0) as historical_min,
            COALESCE(h.hist_max, 0.0) as historical_max,
            COALESCE(h.data_points, 0) as data_points,
            tg.current_ttm_revenue,
            tg.ttm_growth_rate,
            ag.current_annual_revenue,
            ag.annual_growth_rate,
            CASE 
                WHEN s.hist_stddev > 0 THEN (c.ps_ratio_ttm - h.hist_mean) / s.hist_stddev
                ELSE 0.0
            END as z_score,
            c.data_completeness_score as quality_score,
            CASE 
                WHEN h.hist_mean > 0 AND s.hist_stddev > 0 AND h.data_points >= 10 THEN
                    -- Stock is undervalued if ALL THREE conditions are met:
                    -- 1. Current P/S < (Historical Median - 1.0 × Std Dev)  -- Statistical undervaluation
                    -- 2. Revenue Growth > 0% (TTM OR Annual)               -- Growth requirement
                    -- 3. Quality Score >= 50                               -- Data quality filter
                    c.ps_ratio_ttm < (m.hist_median - 1.0 * s.hist_stddev) AND
                    (tg.ttm_growth_rate > 0 OR ag.annual_growth_rate > 0) AND
                    c.data_completeness_score >= 50
                ELSE false
            END as undervalued_flag,
            c.market_cap,
            c.price,
            c.data_completeness_score
        FROM current_data c
        LEFT JOIN historical_stats h ON c.stock_id = h.stock_id
        LEFT JOIN variance_calc v ON c.stock_id = v.stock_id
        LEFT JOIN median_data m ON c.stock_id = m.stock_id
        LEFT JOIN stddev_calc s ON c.stock_id = s.stock_id
        LEFT JOIN ttm_growth tg ON c.stock_id = tg.stock_id
        LEFT JOIN annual_growth ag ON c.stock_id = ag.stock_id
        WHERE c.market_cap > ?
        ORDER BY 
            undervalued_flag DESC,
            c.ps_ratio_ttm ASC
        LIMIT ?
    ", placeholders);
    
    let mut query_builder = sqlx::query_as::<_, PsRevenueGrowthStock>(&query);
    
    // Bind stock tickers
    for ticker in &stock_tickers {
        query_builder = query_builder.bind(ticker);
    }
    
    // Bind min market cap (used twice in the query)
    query_builder = query_builder.bind(min_market_cap_value);
    query_builder = query_builder.bind(min_market_cap_value);
    query_builder = query_builder.bind(limit_value);
    
    match query_builder.fetch_all(&pool).await {
        Ok(stocks) => {
            // Filter to only return truly undervalued stocks
            // Filter to only undervalued stocks
            let undervalued_stocks: Vec<PsRevenueGrowthStock> = stocks
                .into_iter()
                .filter(|stock| stock.undervalued_flag)
                .take(limit_value as usize)
                .collect();
            
            Ok(undervalued_stocks)
        }
        Err(e) => {
            eprintln!("P/S screening with revenue growth query error: {}", e);
            Err(format!("Database query failed: {}", e))
        }
    }
}

#[tauri::command]
pub async fn get_valuation_extremes(symbol: String) -> Result<ValuationExtremes, String> {
    let pool = get_database_connection().await?;
    
    // Get P/E ratio extremes
    let pe_extremes = sqlx::query_as::<_, (Option<f64>, Option<f64>)>(
        "
        SELECT 
            MIN(CASE WHEN pe_ratio > 0 THEN pe_ratio END) as min_pe,
            MAX(CASE WHEN pe_ratio > 0 THEN pe_ratio END) as max_pe
        FROM daily_prices dp
        JOIN stocks s ON dp.stock_id = s.id
        WHERE s.symbol = ?1 AND pe_ratio IS NOT NULL AND pe_ratio > 0
        "
    )
    .bind(&symbol)
    .fetch_one(&pool)
    .await
    .map_err(|e| format!("Failed to fetch P/E extremes: {}", e))?;
    
    // Get P/S ratio extremes
    let ps_extremes = sqlx::query_as::<_, (Option<f64>, Option<f64>)>(
        "
        SELECT 
            MIN(CASE WHEN ps_ratio_ttm > 0 THEN ps_ratio_ttm END) as min_ps,
            MAX(CASE WHEN ps_ratio_ttm > 0 THEN ps_ratio_ttm END) as max_ps
        FROM daily_valuation_ratios dvr
        JOIN stocks s ON dvr.stock_id = s.id
        WHERE s.symbol = ?1 AND ps_ratio_ttm IS NOT NULL AND ps_ratio_ttm > 0
        "
    )
    .bind(&symbol)
    .fetch_one(&pool)
    .await
    .map_err(|e| format!("Failed to fetch P/S extremes: {}", e))?;
    
    // Get EV/S ratio extremes
    let evs_extremes = sqlx::query_as::<_, (Option<f64>, Option<f64>)>(
        "
        SELECT 
            MIN(CASE WHEN evs_ratio_ttm > 0 THEN evs_ratio_ttm END) as min_evs,
            MAX(CASE WHEN evs_ratio_ttm > 0 THEN evs_ratio_ttm END) as max_evs
        FROM daily_valuation_ratios dvr
        JOIN stocks s ON dvr.stock_id = s.id
        WHERE s.symbol = ?1 AND evs_ratio_ttm IS NOT NULL AND evs_ratio_ttm > 0
        "
    )
    .bind(&symbol)
    .fetch_one(&pool)
    .await
    .map_err(|e| format!("Failed to fetch EV/S extremes: {}", e))?;
    
    Ok(ValuationExtremes {
        symbol,
        min_pe_ratio: pe_extremes.0,
        max_pe_ratio: pe_extremes.1,
        min_ps_ratio: ps_extremes.0,
        max_ps_ratio: ps_extremes.1,
        min_evs_ratio: evs_extremes.0,
        max_evs_ratio: evs_extremes.1,
    })
}