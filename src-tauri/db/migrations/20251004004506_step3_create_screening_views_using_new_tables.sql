-- Migration Step 3: Create screening views that the backend commands actually use
-- Risk: LOW (creating views, no data changes)
-- Impact: Backend commands can now use views with correct DATE column types

-- Create the views that the screening commands actually use
-- These will replace the old views in step 4

-- 1. Piotroski Screening Results View (used by piotroski_screening.rs)
CREATE VIEW piotroski_screening_results_new AS
SELECT
    s.id as stock_id,
    s.symbol,
    s.sector,
    
    -- Current financial data
    pm.current_net_income,
    pm.current_operating_cash_flow,
    pm.current_assets,
    pm.current_debt,
    pm.current_equity,
    pm.current_current_assets,
    pm.current_current_liabilities,
    pm.current_shares_outstanding_bs,
    
    -- Prior year data for comparisons
    pm.prior_net_income,
    pm.prior_operating_cash_flow,
    pm.prior_assets,
    pm.prior_debt,
    pm.prior_equity,
    pm.prior_current_assets,
    pm.prior_current_liabilities,
    pm.prior_shares_outstanding_bs,
    
    -- Calculate F-Score criteria (9 points total)
    -- 1. Positive Net Income (1 point)
    CASE WHEN pm.current_net_income > 0 THEN 1 ELSE 0 END as criterion_positive_net_income,
    
    -- 2. Positive Operating Cash Flow (1 point)
    CASE WHEN pm.current_operating_cash_flow > 0 THEN 1 ELSE 0 END as criterion_positive_operating_cash_flow,
    
    -- 3. Improving ROA (1 point)
    CASE WHEN pm.current_net_income > 0 AND pm.current_assets > 0 AND pm.prior_net_income > 0 AND pm.prior_assets > 0
         AND (pm.current_net_income / pm.current_assets) > (pm.prior_net_income / pm.prior_assets)
         THEN 1 ELSE 0 END as criterion_improving_roa,
    
    -- 4. Cash Flow Quality (1 point)
    CASE WHEN pm.current_operating_cash_flow > pm.current_net_income THEN 1 ELSE 0 END as criterion_cash_flow_quality,
    
    -- 5. Decreasing Debt Ratio (1 point)
    CASE WHEN pm.current_debt > 0 AND pm.current_equity > 0 AND pm.prior_debt > 0 AND pm.prior_equity > 0
         AND (pm.current_debt / pm.current_equity) < (pm.prior_debt / pm.prior_equity)
         THEN 1 ELSE 0 END as criterion_decreasing_debt_ratio,
    
    -- 6. Improving Current Ratio (1 point)
    CASE WHEN pm.current_current_assets > 0 AND pm.current_current_liabilities > 0 
         AND pm.prior_current_assets > 0 AND pm.prior_current_liabilities > 0
         AND (pm.current_current_assets / pm.current_current_liabilities) > (pm.prior_current_assets / pm.prior_current_liabilities)
         THEN 1 ELSE 0 END as criterion_improving_current_ratio,
    
    -- 7. No Share Dilution (1 point)
    CASE WHEN pm.current_shares_outstanding_bs <= pm.prior_shares_outstanding_bs THEN 1 ELSE 0 END as criterion_no_dilution,
    
    -- 8. Improving Net Margin (1 point)
    CASE WHEN pm.current_net_income > 0 AND pm.current_revenue > 0 AND pm.prior_net_income > 0 AND pm.prior_revenue > 0
         AND (pm.current_net_income / pm.current_revenue) > (pm.prior_net_income / pm.prior_revenue)
         THEN 1 ELSE 0 END as criterion_improving_net_margin,
    
    -- 9. Improving Asset Turnover (1 point)
    CASE WHEN pm.current_revenue > 0 AND pm.current_assets > 0 AND pm.prior_revenue > 0 AND pm.prior_assets > 0
         AND (pm.current_revenue / pm.current_assets) > (pm.prior_revenue / pm.prior_assets)
         THEN 1 ELSE 0 END as criterion_improving_asset_turnover,
    
    -- Calculate total F-Score
    (CASE WHEN pm.current_net_income > 0 THEN 1 ELSE 0 END +
     CASE WHEN pm.current_operating_cash_flow > 0 THEN 1 ELSE 0 END +
     CASE WHEN pm.current_net_income > 0 AND pm.current_assets > 0 AND pm.prior_net_income > 0 AND pm.prior_assets > 0
          AND (pm.current_net_income / pm.current_assets) > (pm.prior_net_income / pm.prior_assets)
          THEN 1 ELSE 0 END +
     CASE WHEN pm.current_operating_cash_flow > pm.current_net_income THEN 1 ELSE 0 END +
     CASE WHEN pm.current_debt > 0 AND pm.current_equity > 0 AND pm.prior_debt > 0 AND pm.prior_equity > 0
          AND (pm.current_debt / pm.current_equity) < (pm.prior_debt / pm.prior_equity)
          THEN 1 ELSE 0 END +
     CASE WHEN pm.current_current_assets > 0 AND pm.current_current_liabilities > 0 
          AND pm.prior_current_assets > 0 AND pm.prior_current_liabilities > 0
          AND (pm.current_current_assets / pm.current_current_liabilities) > (pm.prior_current_assets / pm.prior_current_liabilities)
          THEN 1 ELSE 0 END +
     CASE WHEN pm.current_shares_outstanding_bs <= pm.prior_shares_outstanding_bs THEN 1 ELSE 0 END +
     CASE WHEN pm.current_net_income > 0 AND pm.current_revenue > 0 AND pm.prior_net_income > 0 AND pm.prior_revenue > 0
          AND (pm.current_net_income / pm.current_revenue) > (pm.prior_net_income / pm.prior_revenue)
          THEN 1 ELSE 0 END +
     CASE WHEN pm.current_revenue > 0 AND pm.current_assets > 0 AND pm.prior_revenue > 0 AND pm.prior_assets > 0
          AND (pm.current_revenue / pm.current_assets) > (pm.prior_revenue / pm.prior_assets)
          THEN 1 ELSE 0 END) as f_score_complete,
    
    -- Data completeness score (0-100)
    ((CASE WHEN pm.current_net_income IS NOT NULL THEN 1 ELSE 0 END) +
     (CASE WHEN pm.current_operating_cash_flow IS NOT NULL THEN 1 ELSE 0 END) +
     (CASE WHEN pm.current_assets IS NOT NULL AND pm.prior_assets IS NOT NULL THEN 1 ELSE 0 END) +
     (CASE WHEN pm.current_operating_cash_flow IS NOT NULL AND pm.current_net_income IS NOT NULL THEN 1 ELSE 0 END) +
     (CASE WHEN pm.current_debt IS NOT NULL AND pm.current_equity IS NOT NULL AND pm.prior_debt IS NOT NULL AND pm.prior_equity IS NOT NULL THEN 1 ELSE 0 END) +
     (CASE WHEN pm.current_current_assets IS NOT NULL AND pm.current_current_liabilities IS NOT NULL AND pm.prior_current_assets IS NOT NULL AND pm.prior_current_liabilities IS NOT NULL THEN 1 ELSE 0 END) +
     (CASE WHEN pm.current_shares_outstanding_bs IS NOT NULL AND pm.prior_shares_outstanding_bs IS NOT NULL THEN 1 ELSE 0 END) +
     (CASE WHEN pm.current_net_income IS NOT NULL AND pm.current_revenue IS NOT NULL AND pm.prior_net_income IS NOT NULL AND pm.prior_revenue IS NOT NULL THEN 1 ELSE 0 END) +
     (CASE WHEN pm.current_revenue IS NOT NULL AND pm.current_assets IS NOT NULL AND pm.prior_revenue IS NOT NULL AND pm.prior_assets IS NOT NULL THEN 1 ELSE 0 END)) * 11.11 as data_completeness_score,
    
    -- Financial metrics for display
    CASE WHEN pm.current_assets > 0 THEN pm.current_net_income / pm.current_assets ELSE NULL END as current_roa,
    CASE WHEN pm.current_equity > 0 THEN pm.current_debt / pm.current_equity ELSE NULL END as current_debt_ratio,
    CASE WHEN pm.current_current_liabilities > 0 THEN pm.current_current_assets / pm.current_current_liabilities ELSE NULL END as current_current_ratio,
    CASE WHEN pm.current_revenue > 0 THEN pm.current_net_income / pm.current_revenue ELSE NULL END as current_net_margin,
    CASE WHEN pm.current_assets > 0 THEN pm.current_revenue / pm.current_assets ELSE NULL END as current_asset_turnover

FROM stocks s
LEFT JOIN piotroski_multi_year_data_new pm ON s.id = pm.stock_id
WHERE s.is_sp500 = 1;

-- 2. O'Shaughnessy Ranking View (used by oshaughnessy_screening.rs)
CREATE VIEW oshaughnessy_ranking_new AS
WITH ranked AS (
  SELECT *,
    -- Rank each metric (lower rank = better value)
    RANK() OVER (ORDER BY pe_ratio ASC) as pe_rank,
    RANK() OVER (ORDER BY pb_ratio ASC) as pb_rank,
    RANK() OVER (ORDER BY ps_ratio ASC) as ps_rank,
    RANK() OVER (ORDER BY evs_ratio ASC) as evs_rank,
    RANK() OVER (ORDER BY ev_ebitda_ratio ASC) as ebitda_rank,
    RANK() OVER (ORDER BY shareholder_yield DESC) as yield_rank,
    COUNT(*) OVER () as total_stocks
  FROM oshaughnessy_value_composite_new
  WHERE pe_ratio IS NOT NULL AND pb_ratio IS NOT NULL 
    AND ps_ratio IS NOT NULL AND evs_ratio IS NOT NULL
    AND ev_ebitda_ratio IS NOT NULL AND shareholder_yield IS NOT NULL
)
SELECT *,
  -- Composite score (average of all 6 ranks)
  CAST((pe_rank + pb_rank + ps_rank + evs_rank + ebitda_rank + yield_rank) / 6.0 AS REAL) as composite_score,
  
  -- Percentile ranking (0-100)
  CAST(ROUND(((pe_rank + pb_rank + ps_rank + evs_rank + ebitda_rank + yield_rank) / 6.0 / total_stocks) * 100, 1) AS REAL) as composite_percentile,
  
  -- Overall ranking
  RANK() OVER (ORDER BY (pe_rank + pb_rank + ps_rank + evs_rank + ebitda_rank + yield_rank) / 6.0 ASC) as overall_rank,
  
  -- Pass screening if in top 10 stocks
  CASE WHEN 
    RANK() OVER (ORDER BY (pe_rank + pb_rank + ps_rank + evs_rank + ebitda_rank + yield_rank) / 6.0 ASC) <= 10
    THEN 1 ELSE 0 END as passes_screening
FROM ranked
ORDER BY composite_score ASC;