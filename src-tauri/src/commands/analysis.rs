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
pub async fn export_data(symbol: String, format: String) -> Result<String, String> {
    let pool = get_database_connection().await?;
    
    // Get count of records for this stock by symbol
    let record_count = match sqlx::query("
        SELECT COUNT(*) as count 
        FROM daily_prices dp
        JOIN stocks s ON dp.stock_id = s.id
        WHERE s.symbol = ?1")
        .bind(&symbol)
        .fetch_one(&pool).await 
    {
        Ok(row) => row.get::<i64, _>("count"),
        Err(_) => 0,
    };
    
    let message = format!(
        "Export simulation: {} records for {} in {} format. \
        This feature will be enhanced to generate actual files in the next phase.",
        record_count,
        symbol,
        format
    );
    
    Ok(message)
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

#[tauri::command]
pub async fn get_undervalued_stocks_by_ps(max_ps_ratio: f64, limit: Option<i32>) -> Result<Vec<ValuationRatios>, String> {
    let pool = get_database_connection().await?;
    
    let limit_value = limit.unwrap_or(50);
    
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
        WHERE dvr.ps_ratio_ttm IS NOT NULL 
          AND dvr.ps_ratio_ttm > 0 
          AND dvr.ps_ratio_ttm <= ?1
        ORDER BY dvr.ps_ratio_ttm ASC
        LIMIT ?2
    ";
    
    match sqlx::query(query)
        .bind(max_ps_ratio)
        .bind(limit_value)
        .fetch_all(&pool).await 
    {
        Ok(rows) => {
            let undervalued_stocks: Vec<ValuationRatios> = rows.into_iter().map(|row| {
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
            
            Ok(undervalued_stocks)
        }
        Err(e) => {
            eprintln!("Undervalued stocks query error: {}", e);
            Err(format!("Database query failed: {}", e))
        }
    }
}