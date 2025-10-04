-- Migration Step 2: Create new views using the new tables with correct DATE types
-- Risk: LOW (creating views, no data changes)
-- Impact: Views now use tables with proper DATE column types

-- Create new views using the new tables (with _new suffix)
-- These will replace the old views in step 4

-- 1. Revenue Growth Analysis View
CREATE VIEW revenue_growth_analysis_new AS
SELECT 
    s.id as stock_id,
    s.symbol,
    
    -- Annual revenue growth
    annual_current.revenue as annual_current_revenue,
    annual_previous.revenue as annual_previous_revenue,
    CASE 
        WHEN annual_previous.revenue > 0 THEN 
            (annual_current.revenue - annual_previous.revenue) / annual_previous.revenue * 100
        ELSE NULL 
    END as annual_growth_rate,
    
    -- TTM revenue growth
    ttm_current.revenue as ttm_current_revenue,
    ttm_previous.revenue as ttm_previous_revenue,
    CASE 
        WHEN ttm_previous.revenue > 0 THEN 
            (ttm_current.revenue - ttm_previous.revenue) / ttm_previous.revenue * 100
        ELSE NULL 
    END as ttm_growth_rate,
    
    -- Quarterly revenue growth (latest quarter vs previous quarter)
    q_current.revenue as quarterly_current_revenue,
    q_previous.revenue as quarterly_previous_revenue,
    CASE 
        WHEN q_previous.revenue > 0 THEN 
            (q_current.revenue - q_previous.revenue) / q_previous.revenue * 100
        ELSE NULL 
    END as quarterly_growth_rate,
    
    -- Data quality metrics
    CASE 
        WHEN annual_current.revenue IS NOT NULL AND ttm_current.revenue IS NOT NULL AND q_current.revenue IS NOT NULL THEN 100
        WHEN annual_current.revenue IS NOT NULL AND ttm_current.revenue IS NOT NULL THEN 75
        WHEN annual_current.revenue IS NOT NULL OR ttm_current.revenue IS NOT NULL THEN 50
        ELSE 25
    END as data_completeness_score

FROM stocks s

-- Current annual revenue (most recent)
LEFT JOIN income_statements_new annual_current ON s.id = annual_current.stock_id 
    AND annual_current.period_type = 'Annual'
    AND annual_current.report_date = (
        SELECT MAX(report_date) FROM income_statements_new 
        WHERE stock_id = s.id AND period_type = 'Annual'
    )

-- Previous annual revenue (12 months earlier)
LEFT JOIN income_statements_new annual_previous ON s.id = annual_previous.stock_id 
    AND annual_previous.period_type = 'Annual'
    AND annual_previous.report_date = (
        SELECT MAX(report_date) FROM income_statements_new 
        WHERE stock_id = s.id AND period_type = 'Annual'
        AND report_date < annual_current.report_date
    )

-- Current TTM revenue (most recent)
LEFT JOIN income_statements_new ttm_current ON s.id = ttm_current.stock_id 
    AND ttm_current.period_type = 'TTM'
    AND ttm_current.report_date = (
        SELECT MAX(report_date) FROM income_statements_new 
        WHERE stock_id = s.id AND period_type = 'TTM'
    )

-- Previous TTM revenue (12 months earlier)
LEFT JOIN income_statements_new ttm_previous ON s.id = ttm_previous.stock_id 
    AND ttm_previous.period_type = 'TTM'
    AND ttm_previous.report_date = (
        SELECT MAX(report_date) FROM income_statements_new 
        WHERE stock_id = s.id AND period_type = 'TTM'
        AND report_date < ttm_current.report_date
    )

-- Current quarterly revenue (most recent quarter)
LEFT JOIN income_statements_new q_current ON s.id = q_current.stock_id 
    AND q_current.period_type = 'Quarterly'
    AND q_current.report_date = (
        SELECT MAX(report_date) FROM income_statements_new 
        WHERE stock_id = s.id AND period_type = 'Quarterly'
    )

-- Previous quarterly revenue (previous quarter)
LEFT JOIN income_statements_new q_previous ON s.id = q_previous.stock_id 
    AND q_previous.period_type = 'Quarterly'
    AND q_previous.report_date = (
        SELECT MAX(report_date) FROM income_statements_new 
        WHERE stock_id = s.id AND period_type = 'Quarterly'
        AND report_date < q_current.report_date
    );

-- 2. Enterprise Value Analysis View
CREATE VIEW enterprise_value_analysis_new AS
SELECT 
    s.id as stock_id,
    s.symbol,
    
    -- Latest price and market cap
    dp.close_price as latest_price,
    dp.market_cap as latest_market_cap,
    dp.date as latest_price_date,
    
    -- Balance sheet data for EV calculation
    bs_current.cash_and_equivalents as current_cash,
    bs_current.total_debt as current_debt,
    
    -- Enterprise Value = Market Cap + Total Debt - Cash
    CASE 
        WHEN dp.market_cap IS NOT NULL AND bs_current.total_debt IS NOT NULL AND bs_current.cash_and_equivalents IS NOT NULL THEN
            dp.market_cap + bs_current.total_debt - bs_current.cash_and_equivalents
        ELSE NULL
    END as enterprise_value,
    
    -- Revenue data for EV/S calculation
    rev_current.revenue as current_revenue,
    
    -- EV/S Ratio = Enterprise Value / Revenue
    CASE 
        WHEN dp.market_cap IS NOT NULL AND bs_current.total_debt IS NOT NULL AND bs_current.cash_and_equivalents IS NOT NULL AND rev_current.revenue > 0 THEN
            (dp.market_cap + bs_current.total_debt - bs_current.cash_and_equivalents) / rev_current.revenue
        ELSE NULL
    END as evs_ratio

FROM stocks s

-- Latest price data
LEFT JOIN daily_prices dp ON s.id = dp.stock_id 
    AND dp.date = (
        SELECT MAX(date) FROM daily_prices 
        WHERE stock_id = s.id
    )

-- Current balance sheet data (most recent)
LEFT JOIN balance_sheets_new bs_current ON s.id = bs_current.stock_id 
    AND bs_current.period_type = 'TTM'
    AND bs_current.report_date = (
        SELECT MAX(report_date) FROM balance_sheets_new 
        WHERE stock_id = s.id AND period_type = 'TTM'
    )

-- Current revenue data (most recent TTM)
LEFT JOIN income_statements_new rev_current ON s.id = rev_current.stock_id 
    AND rev_current.period_type = 'TTM'
    AND rev_current.report_date = (
        SELECT MAX(report_date) FROM income_statements_new 
        WHERE stock_id = s.id AND period_type = 'TTM'
    );

-- 3. Revenue Data Validation View
CREATE VIEW revenue_data_validation_new AS
SELECT 
    'Revenue Data Coverage' as validation_type,
    COUNT(DISTINCT s.id) as total_stocks,
    COUNT(DISTINCT CASE WHEN i_annual.stock_id IS NOT NULL THEN s.id END) as stocks_with_annual,
    COUNT(DISTINCT CASE WHEN i_quarterly.stock_id IS NOT NULL THEN s.id END) as stocks_with_quarterly,
    COUNT(DISTINCT CASE WHEN i_ttm.stock_id IS NOT NULL THEN s.id END) as stocks_with_ttm,
    COUNT(DISTINCT CASE WHEN i_annual.stock_id IS NOT NULL AND i_quarterly.stock_id IS NOT NULL AND i_ttm.stock_id IS NOT NULL THEN s.id END) as stocks_with_all_periods
FROM stocks s
LEFT JOIN income_statements_new i_annual ON s.id = i_annual.stock_id AND i_annual.period_type = 'Annual'
LEFT JOIN income_statements_new i_quarterly ON s.id = i_quarterly.stock_id AND i_quarterly.period_type = 'Quarterly'
LEFT JOIN income_statements_new i_ttm ON s.id = i_ttm.stock_id AND i_ttm.period_type = 'TTM';

-- 4. Balance Sheet Validation View
CREATE VIEW balance_sheet_validation_new AS
SELECT 
    'Balance Sheet Data Coverage' as validation_type,
    COUNT(DISTINCT s.id) as total_stocks,
    COUNT(DISTINCT CASE WHEN bs_annual.stock_id IS NOT NULL THEN s.id END) as stocks_with_annual_balance,
    COUNT(DISTINCT CASE WHEN bs_quarterly.stock_id IS NOT NULL THEN s.id END) as stocks_with_quarterly_balance,
    COUNT(DISTINCT CASE WHEN bs_ttm.stock_id IS NOT NULL THEN s.id END) as stocks_with_ttm_balance,
    COUNT(DISTINCT CASE WHEN bs_annual.stock_id IS NOT NULL AND bs_quarterly.stock_id IS NOT NULL AND bs_ttm.stock_id IS NOT NULL THEN s.id END) as stocks_with_all_balance_periods
FROM stocks s
LEFT JOIN balance_sheets_new bs_annual ON s.id = bs_annual.stock_id AND bs_annual.period_type = 'Annual'
LEFT JOIN balance_sheets_new bs_quarterly ON s.id = bs_quarterly.stock_id AND bs_quarterly.period_type = 'Quarterly'
LEFT JOIN balance_sheets_new bs_ttm ON s.id = bs_ttm.stock_id AND bs_ttm.period_type = 'TTM';

-- 5. Sector Data Quality View
CREATE VIEW sector_data_quality_new AS
SELECT
    s.gics_sector,
    COUNT(*) as total_companies,
    COUNT(CASE WHEN s.is_sp500 = 1 THEN 1 END) as sp500_companies,
    -- Balance sheet data quality
    COUNT(CASE WHEN bs.current_assets IS NOT NULL AND bs.fiscal_year >= 2019 THEN 1 END) as has_current_assets,
    COUNT(CASE WHEN bs.current_liabilities IS NOT NULL AND bs.fiscal_year >= 2019 THEN 1 END) as has_current_liabilities,
    COUNT(CASE WHEN bs.total_debt IS NOT NULL AND bs.fiscal_year >= 2019 THEN 1 END) as has_total_debt,
    COUNT(CASE WHEN bs.total_equity IS NOT NULL AND bs.fiscal_year >= 2019 THEN 1 END) as has_total_equity,
    -- Income statement data quality
    COUNT(CASE WHEN is_.net_income IS NOT NULL AND is_.fiscal_year >= 2019 THEN 1 END) as has_net_income,
    COUNT(CASE WHEN is_.revenue IS NOT NULL AND is_.fiscal_year >= 2019 THEN 1 END) as has_revenue,
    -- Cash flow data quality
    COUNT(CASE WHEN cf.operating_cash_flow IS NOT NULL AND cf.fiscal_year >= 2019 THEN 1 END) as has_operating_cf
FROM stocks s
LEFT JOIN balance_sheets_new bs ON s.id = bs.stock_id AND bs.period_type = 'Annual'
LEFT JOIN income_statements_new is_ ON s.id = is_.stock_id AND is_.period_type = 'Annual'
LEFT JOIN cash_flow_statements_new cf ON s.id = cf.stock_id AND cf.period_type = 'Annual'
WHERE s.is_sp500 = 1
GROUP BY s.gics_sector
ORDER BY total_companies DESC;

-- 6. EBITDA Calculations View
CREATE VIEW ebitda_calculations_new AS
SELECT
    i.stock_id,
    i.report_date,
    i.period_type,
    i.operating_income,
    cf.depreciation_expense,
    cf.amortization_expense,
    -- EBITDA = Operating Income + Depreciation + Amortization
    CASE
        WHEN i.operating_income IS NOT NULL AND cf.depreciation_expense IS NOT NULL
        THEN i.operating_income + COALESCE(cf.depreciation_expense, 0) + COALESCE(cf.amortization_expense, 0)
        ELSE NULL
    END as ebitda
FROM income_statements_new i
JOIN cash_flow_statements_new cf ON i.stock_id = cf.stock_id
    AND i.report_date = cf.report_date
    AND i.period_type = cf.period_type
WHERE i.period_type = 'TTM';

-- 7. Book Value Calculations View
CREATE VIEW book_value_calculations_new AS
SELECT
    stock_id,
    report_date,
    period_type,
    total_equity,
    shares_outstanding,
    -- Book Value per Share = Total Equity ÷ Shares Outstanding
    CASE
        WHEN total_equity IS NOT NULL AND shares_outstanding IS NOT NULL AND shares_outstanding > 0
        THEN total_equity / shares_outstanding
        ELSE NULL
    END as book_value_per_share
FROM balance_sheets_new
WHERE period_type = 'Annual' AND total_equity IS NOT NULL AND total_equity > 0;

-- 8. Piotroski Multi Year Data View
CREATE VIEW piotroski_multi_year_data_new AS
SELECT
    s.id as stock_id,
    s.symbol,
    s.sector,
    s.industry,

    -- Current Annual data (most recent fiscal year)
    current_income.net_income as current_net_income,
    current_income.revenue as current_revenue,
    current_income.gross_profit as current_gross_profit,
    current_income.cost_of_revenue as current_cost_of_revenue,
    current_income.operating_income as current_operating_income,
    current_income.shares_diluted as current_shares,
    current_income.shares_diluted as current_shares_outstanding,

    -- Current Annual balance data (most recent with current assets/liabilities)
    current_balance.total_assets as current_assets,
    current_balance.total_debt as current_debt,
    current_balance.total_equity as current_equity,
    current_balance.current_assets as current_current_assets,
    current_balance.current_liabilities as current_current_liabilities,
    current_balance.shares_outstanding as current_shares_outstanding_bs,

    -- Prior year Annual data (for year-over-year comparisons)
    prior_income.net_income as prior_net_income,
    prior_income.revenue as prior_revenue,
    prior_income.gross_profit as prior_gross_profit,
    prior_income.cost_of_revenue as prior_cost_of_revenue,
    prior_income.operating_income as prior_operating_income,
    prior_income.shares_diluted as prior_shares,
    prior_income.shares_diluted as prior_shares_outstanding,

    -- Prior year balance data (second most recent, regardless of current assets/liabilities)
    prior_balance.total_assets as prior_assets,
    prior_balance.total_debt as prior_debt,
    prior_balance.total_equity as prior_equity,
    prior_balance.current_assets as prior_current_assets,
    prior_balance.current_liabilities as prior_current_liabilities,
    prior_balance.shares_outstanding as prior_shares_outstanding_bs,

    -- Cash flow data (Annual)
    current_cashflow.operating_cash_flow as current_operating_cash_flow,
    prior_cashflow.operating_cash_flow as prior_operating_cash_flow

FROM stocks s

-- Current Annual income data
LEFT JOIN (
    SELECT stock_id, net_income, revenue, gross_profit, cost_of_revenue,
           operating_income, shares_diluted, report_date,
           ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
    FROM income_statements_new
    WHERE period_type = 'Annual'
) current_income ON s.id = current_income.stock_id AND current_income.rn = 1

-- Prior Annual income data (previous fiscal year)
LEFT JOIN (
    SELECT stock_id, net_income, revenue, gross_profit, cost_of_revenue,
           operating_income, shares_diluted, report_date,
           ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
    FROM income_statements_new
    WHERE period_type = 'Annual'
) prior_income ON s.id = prior_income.stock_id AND prior_income.rn = 2

-- Current Annual balance data (most recent with current assets/liabilities)
LEFT JOIN (
    SELECT stock_id, total_assets, total_debt, total_equity,
           current_assets, current_liabilities, shares_outstanding, report_date,
           ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY 
               CASE WHEN current_assets IS NOT NULL AND current_liabilities IS NOT NULL THEN 0 ELSE 1 END,
               report_date DESC) as rn
    FROM balance_sheets_new
    WHERE period_type = 'Annual'
) current_balance ON s.id = current_balance.stock_id AND current_balance.rn = 1

-- Prior Annual balance data (second most recent, regardless of current assets/liabilities)
LEFT JOIN (
    SELECT stock_id, total_assets, total_debt, total_equity,
           current_assets, current_liabilities, shares_outstanding, report_date,
           ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
    FROM balance_sheets_new
    WHERE period_type = 'Annual'
) prior_balance ON s.id = prior_balance.stock_id AND prior_balance.rn = 2

-- Current Annual cash flow data
LEFT JOIN (
    SELECT stock_id, operating_cash_flow, report_date,
           ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
    FROM cash_flow_statements_new
    WHERE period_type = 'Annual'
) current_cashflow ON s.id = current_cashflow.stock_id AND current_cashflow.rn = 1

-- Prior Annual cash flow data
LEFT JOIN (
    SELECT stock_id, operating_cash_flow, report_date,
           ROW_NUMBER() OVER (PARTITION BY stock_id ORDER BY report_date DESC) as rn
    FROM cash_flow_statements_new
    WHERE period_type = 'Annual'
) prior_cashflow ON s.id = prior_cashflow.stock_id AND prior_cashflow.rn = 2;

-- 9. O'Shaughnessy Value Composite View
CREATE VIEW oshaughnessy_value_composite_new AS
WITH latest_data AS (
  SELECT 
    s.id as stock_id,
    s.symbol,
    s.sector,
    
    -- Latest price data
    dp.price as current_price,
    dp.market_cap,
    
    -- Latest income statement data (Annual)
    i.net_income,
    i.revenue,
    i.shares_diluted,
    i.operating_income,
    
    -- Latest balance sheet data (Annual)
    b.total_equity,
    b.shares_outstanding,
    b.total_debt,
    b.cash_and_equivalents,
    
    -- Latest cash flow data (Annual)
    cf.dividends_paid,
    cf.share_repurchases,
    cf.depreciation_expense,
    cf.amortization_expense,
    
    -- Calculate enterprise value: Market Cap + Total Debt - Cash
    (dp.market_cap + COALESCE(b.total_debt, 0) - COALESCE(b.cash_and_equivalents, 0)) as enterprise_value,
    
    -- Calculate EBITDA: Operating Income + Depreciation + Amortization
    (COALESCE(i.operating_income, 0) + COALESCE(cf.depreciation_expense, 0) + COALESCE(cf.amortization_expense, 0)) as ebitda,
    
    -- Row numbers for latest data selection
    ROW_NUMBER() OVER (PARTITION BY s.id ORDER BY dp.date DESC) as price_rn,
    ROW_NUMBER() OVER (PARTITION BY s.id ORDER BY i.report_date DESC) as income_rn,
    ROW_NUMBER() OVER (PARTITION BY s.id ORDER BY b.report_date DESC) as balance_rn,
    ROW_NUMBER() OVER (PARTITION BY s.id ORDER BY cf.report_date DESC) as cashflow_rn
    
  FROM stocks s
  LEFT JOIN daily_prices dp ON s.id = dp.stock_id
  LEFT JOIN income_statements_new i ON s.id = i.stock_id AND i.period_type = 'Annual'
  LEFT JOIN balance_sheets_new b ON s.id = b.stock_id AND b.period_type = 'Annual'
  LEFT JOIN cash_flow_statements_new cf ON s.id = cf.stock_id AND cf.period_type = 'Annual'
  WHERE s.is_sp500 = 1
)
SELECT 
  stock_id,
  symbol,
  sector,
  current_price,
  market_cap,
  enterprise_value,
  
  -- Calculate all 6 O'Shaughnessy metrics on-demand
  
  -- 1. P/E Ratio: Price / (Net Income / Shares Diluted)
  CASE WHEN net_income > 0 AND shares_diluted > 0 
       THEN current_price / (net_income / shares_diluted) 
       ELSE NULL END as pe_ratio,
  
  -- 2. P/B Ratio: Price / (Total Equity / Shares Outstanding)
  CASE WHEN total_equity > 0 AND shares_outstanding > 0 
       THEN current_price / (total_equity / shares_outstanding) 
       ELSE NULL END as pb_ratio,
  
  -- 3. P/S Ratio: Market Cap / Revenue
  CASE WHEN revenue > 0 
       THEN market_cap / revenue 
       ELSE NULL END as ps_ratio,
  
  -- 4. EV/S Ratio: Enterprise Value / Revenue
  CASE WHEN revenue > 0 
       THEN enterprise_value / revenue 
       ELSE NULL END as evs_ratio,
  
  -- 5. EV/EBITDA Ratio: Enterprise Value / EBITDA
  CASE WHEN ebitda > 0 
       THEN enterprise_value / ebitda 
       ELSE NULL END as ev_ebitda_ratio,
  
  -- 6. Shareholder Yield: (Dividends + Share Repurchases) / Market Cap
  CASE WHEN market_cap > 0 
       THEN (COALESCE(dividends_paid, 0) + COALESCE(share_repurchases, 0)) / market_cap 
       ELSE NULL END as shareholder_yield,
  
  -- Data completeness score (0-100 based on available metrics)
  ((CASE WHEN net_income > 0 AND shares_diluted > 0 THEN 1 ELSE 0 END) +
   (CASE WHEN total_equity > 0 AND shares_outstanding > 0 THEN 1 ELSE 0 END) +
   (CASE WHEN revenue > 0 THEN 1 ELSE 0 END) +
   (CASE WHEN revenue > 0 THEN 1 ELSE 0 END) +
   (CASE WHEN ebitda > 0 THEN 1 ELSE 0 END) +
   (CASE WHEN market_cap > 0 THEN 1 ELSE 0 END)) * 16.67 as data_completeness_score

FROM latest_data
WHERE price_rn = 1 AND income_rn = 1 AND balance_rn = 1 AND cashflow_rn = 1
  AND market_cap > 200000000;