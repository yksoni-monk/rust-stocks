use crate::models::garp_pe::{GarpPeScreeningResult, GarpPeScreeningCriteria};
use crate::database::helpers::get_database_connection;

#[tauri::command]
pub async fn get_garp_pe_screening_results(
    stock_tickers: Vec<String>, 
    criteria: Option<GarpPeScreeningCriteria>,
    limit: Option<i32>
) -> Result<Vec<GarpPeScreeningResult>, String> {
    let pool = get_database_connection().await?;
    let criteria = criteria.unwrap_or_default();
    let limit_value = limit.unwrap_or(50);
    
    if stock_tickers.is_empty() {
        return Ok(vec![]);
    }
    
    // Create placeholders for the IN clause
    let placeholders = stock_tickers.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    
    let query = format!("
        SELECT 
            pra.stock_id,
            pra.symbol,
            pra.sector,
            pra.current_pe_ratio,
            pra.peg_ratio,
            pra.current_price,
            pra.passes_positive_earnings,
            pra.passes_peg_filter,
            pra.current_eps_ttm,
            pra.current_eps_annual,
            pra.eps_growth_rate_ttm,
            pra.eps_growth_rate_annual,
            pra.current_ttm_revenue,
            pra.ttm_growth_rate,
            pra.current_annual_revenue,
            pra.annual_growth_rate,
            pra.passes_revenue_growth_filter,
            pra.current_ttm_net_income,
            pra.net_profit_margin,
            pra.passes_profitability_filter,
            pra.total_debt,
            pra.total_equity,
            pra.debt_to_equity_ratio,
            pra.passes_debt_filter,
            
            -- GARP Score: Revenue Growth % / PEG Ratio
            CASE 
                WHEN pra.peg_ratio > 0 AND pra.ttm_growth_rate IS NOT NULL THEN 
                    CAST(pra.ttm_growth_rate AS REAL) / CAST(pra.peg_ratio AS REAL)
                WHEN pra.peg_ratio > 0 AND pra.annual_growth_rate IS NOT NULL THEN 
                    CAST(pra.annual_growth_rate AS REAL) / CAST(pra.peg_ratio AS REAL)
                ELSE 0.0
            END as garp_score,
            
            -- Quality score (0-100)
            CASE 
                WHEN pra.eps_growth_rate_ttm IS NOT NULL 
                     AND pra.ttm_growth_rate IS NOT NULL 
                     AND pra.net_profit_margin IS NOT NULL 
                     AND pra.debt_to_equity_ratio IS NOT NULL THEN 100
                WHEN pra.eps_growth_rate_ttm IS NOT NULL 
                     AND pra.ttm_growth_rate IS NOT NULL 
                     AND pra.net_profit_margin IS NOT NULL THEN 75
                WHEN pra.eps_growth_rate_ttm IS NOT NULL 
                     AND (pra.ttm_growth_rate IS NOT NULL OR pra.net_profit_margin IS NOT NULL) THEN 50
                ELSE 25
            END as quality_score,
            
            -- Final GARP screening result
            CASE 
                WHEN pra.passes_positive_earnings 
                     AND pra.passes_peg_filter 
                     AND pra.passes_revenue_growth_filter 
                     AND pra.passes_profitability_filter 
                     AND pra.passes_debt_filter
                     AND (CASE 
                        WHEN pra.eps_growth_rate_ttm IS NOT NULL 
                             AND pra.ttm_growth_rate IS NOT NULL 
                             AND pra.net_profit_margin IS NOT NULL 
                             AND pra.debt_to_equity_ratio IS NOT NULL THEN 100
                        WHEN pra.eps_growth_rate_ttm IS NOT NULL 
                             AND pra.ttm_growth_rate IS NOT NULL 
                             AND pra.net_profit_margin IS NOT NULL THEN 75
                        WHEN pra.eps_growth_rate_ttm IS NOT NULL 
                             AND (pra.ttm_growth_rate IS NOT NULL OR pra.net_profit_margin IS NOT NULL) THEN 50
                        ELSE 25
                     END) >= ?
                THEN true
                ELSE false
            END as passes_garp_screening,
            
            pra.market_cap,
            pra.data_completeness_score
            
        FROM peg_ratio_analysis pra
        WHERE pra.symbol IN ({})
          AND (pra.market_cap > ? OR pra.market_cap IS NULL)
        ORDER BY 
            passes_garp_screening DESC,
            garp_score DESC,
            quality_score DESC,
            pra.peg_ratio ASC
        LIMIT ?
    ", placeholders);
    
    let mut query_builder = sqlx::query_as::<_, GarpPeScreeningResult>(&query);
    
    // Bind parameters
    query_builder = query_builder.bind(criteria.min_quality_score);
    
    // Bind stock tickers
    for ticker in &stock_tickers {
        query_builder = query_builder.bind(ticker);
    }
    
    // Bind remaining parameters
    query_builder = query_builder.bind(criteria.min_market_cap);
    query_builder = query_builder.bind(limit_value);
    
    let results = query_builder.fetch_all(&pool).await
        .map_err(|e| format!("GARP P/E screening query failed: {}", e))?;
    
    Ok(results)
}
