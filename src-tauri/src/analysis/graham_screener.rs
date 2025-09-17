// Benjamin Graham Value Screening Engine
// Implementation of "The Intelligent Investor" principles for systematic stock analysis

use anyhow::Result;
use chrono::Local;
use sqlx::{Pool, Row, Sqlite};

use crate::models::graham_value::{
    GrahamScreeningCriteria, GrahamScreeningResult, GrahamScreeningResultWithDetails,
    GrahamScreeningStats, StockFinancialData, GrahamScoringWeights, 
    get_sector_adjustments, SectorAdjustments,
};

pub struct GrahamScreener {
    db_pool: Pool<Sqlite>,
    scoring_weights: GrahamScoringWeights,
}

impl GrahamScreener {
    pub fn new(db_pool: Pool<Sqlite>) -> Self {
        Self {
            db_pool,
            scoring_weights: GrahamScoringWeights::default(),
        }
    }

    /// Run complete Graham screening analysis on S&P 500 stocks
    pub async fn run_screening(
        &self, 
        criteria: &GrahamScreeningCriteria
    ) -> Result<Vec<GrahamScreeningResultWithDetails>> {
        println!("🔍 Starting Graham value screening with {} criteria", 
                 if criteria.require_positive_earnings { "strict" } else { "relaxed" });

        // 1. Load S&P 500 stocks with complete financial data
        let stocks = self.load_stocks_with_financials().await?;
        println!("📊 Loaded {} S&P 500 stocks for analysis", stocks.len());

        // 2. Calculate Graham metrics and apply filters
        let mut results = Vec::new();
        for stock in stocks {
            if let Ok(result) = self.analyze_stock(&stock, criteria).await {
                results.push(result);
            }
        }

        println!("✅ Analyzed {} stocks, {} passed initial calculations", 
                 results.len(), 
                 results.iter().filter(|r| r.pe_ratio.is_some()).count());

        // 3. Apply all Graham screening filters
        self.apply_screening_filters(&mut results, criteria).await?;

        let passed_count = results.iter().filter(|r| r.passes_all_filters).count();
        println!("🎯 {} stocks passed all Graham screening criteria", passed_count);

        // 4. Calculate composite scores and rankings
        self.calculate_scores_and_rankings(&mut results).await?;

        // 5. Save results to database
        self.save_screening_results(&results).await?;

        // 6. Enhance results with additional details and return filtered list
        let filtered_results = results.into_iter()
            .filter(|r| r.passes_all_filters)
            .collect::<Vec<_>>();

        let enhanced_results = self.enhance_results_with_details(filtered_results).await?;
        
        println!("📈 Returning {} qualified Graham value stocks", enhanced_results.len());
        Ok(enhanced_results)
    }

    /// Load S&P 500 stocks with comprehensive financial data
    async fn load_stocks_with_financials(&self) -> Result<Vec<StockFinancialData>> {
        let query = r#"
            SELECT DISTINCT
                s.id as stock_id,
                s.symbol,
                s.company_name,
                s.sector,
                s.industry,
                s.is_sp500,
                
                -- Current price data
                sp.latest_price as current_price,
                inc.shares_basic as shares_outstanding,
                
                -- Income statement data (TTM)
                inc.revenue,
                inc.net_income,
                inc.operating_income,
                COALESCE(inc.interest_expense_net, 0) as interest_expense,
                
                -- Balance sheet data (TTM)
                bal.total_assets,
                bal.total_equity,
                bal.long_term_debt as total_debt,
                bal.total_current_assets as current_assets,
                bal.total_current_liabilities as current_liabilities,
                bal.cash_and_equivalents,
                
                -- Historical revenue for growth calculations
                inc_1y.revenue as revenue_1y_ago,
                inc_3y.revenue as revenue_3y_ago,
                
                -- Dividend data (estimate from yield and price)
                sp.latest_price * COALESCE(dvr.ps_ratio_ttm, 0) * 0.01 as dividend_per_share
                
            FROM stocks s
            JOIN sp500_symbols sp500 ON s.symbol = sp500.symbol
            LEFT JOIN (
                SELECT stock_id, close_price as latest_price
                FROM daily_prices dp1
                WHERE date = (SELECT MAX(date) FROM daily_prices dp2 WHERE dp2.stock_id = dp1.stock_id)
            ) sp ON s.id = sp.stock_id
            LEFT JOIN income_statements inc ON s.id = inc.stock_id 
                AND inc.fiscal_period = 'FY' 
                AND inc.fiscal_year = (SELECT MAX(fiscal_year) FROM income_statements WHERE stock_id = s.id AND fiscal_period = 'FY')
            LEFT JOIN balance_sheets bal ON s.id = bal.stock_id 
                AND bal.fiscal_period = 'FY' 
                AND bal.fiscal_year = (SELECT MAX(fiscal_year) FROM balance_sheets WHERE stock_id = s.id AND fiscal_period = 'FY')
            LEFT JOIN daily_valuation_ratios dvr ON s.id = dvr.stock_id
                AND dvr.date = (SELECT MAX(date) FROM daily_valuation_ratios WHERE stock_id = s.id)
                
            -- Historical data for growth calculations
            LEFT JOIN income_statements inc_1y ON s.id = inc_1y.stock_id 
                AND inc_1y.fiscal_year = inc.fiscal_year - 1
                AND inc_1y.fiscal_period = 'FY'
            LEFT JOIN income_statements inc_3y ON s.id = inc_3y.stock_id 
                AND inc_3y.fiscal_year = inc.fiscal_year - 3
                AND inc_3y.fiscal_period = 'FY'
                
            WHERE s.status = 'active'
                AND sp.latest_price IS NOT NULL
                AND sp.latest_price > 0
            ORDER BY s.symbol
        "#;

        let rows = sqlx::query(query).fetch_all(&self.db_pool).await?;
        
        let stocks = rows.into_iter().map(|row| {
            StockFinancialData {
                stock_id: row.get("stock_id"),
                symbol: row.get("symbol"),
                company_name: row.try_get("company_name").ok(),
                sector: row.try_get("sector").ok(),
                industry: row.try_get("industry").ok(),
                is_sp500: true, // All stocks in this query are S&P 500 by definition
                
                current_price: row.try_get("current_price").ok(),
                shares_outstanding: row.try_get("shares_outstanding").ok(),
                
                revenue: row.try_get("revenue").ok(),
                net_income: row.try_get("net_income").ok(),
                operating_income: row.try_get("operating_income").ok(),
                interest_expense: row.try_get("interest_expense").ok(),
                
                total_assets: row.try_get("total_assets").ok(),
                total_equity: row.try_get("total_equity").ok(),
                total_debt: row.try_get("total_debt").ok(),
                current_assets: row.try_get("current_assets").ok(),
                current_liabilities: row.try_get("current_liabilities").ok(),
                cash_and_equivalents: row.try_get("cash_and_equivalents").ok(),
                
                revenue_1y_ago: row.try_get("revenue_1y_ago").ok(),
                revenue_3y_ago: row.try_get("revenue_3y_ago").ok(),
                dividend_per_share: row.try_get("dividend_per_share").ok(),
            }
        }).collect();

        Ok(stocks)
    }

    /// Analyze individual stock and calculate Graham metrics
    async fn analyze_stock(
        &self,
        stock: &StockFinancialData,
        _criteria: &GrahamScreeningCriteria,
    ) -> Result<GrahamScreeningResult> {
        let sector_adj = get_sector_adjustments(
            stock.sector.as_deref().unwrap_or("General")
        );

        // Calculate core Graham ratios
        let pe_ratio = self.calculate_pe_ratio(stock).await?;
        let pb_ratio = self.calculate_pb_ratio(stock).await?;
        let pe_pb_product = match (pe_ratio, pb_ratio) {
            (Some(pe), Some(pb)) => Some(pe * pb),
            _ => None,
        };

        // Calculate financial health metrics
        let dividend_yield = self.calculate_dividend_yield(stock).await?;
        let debt_to_equity = self.calculate_debt_to_equity(stock).await?;
        let profit_margin = self.calculate_profit_margin(stock).await?;
        let current_ratio = self.calculate_current_ratio(stock).await?;
        let interest_coverage = self.calculate_interest_coverage(stock).await?;
        let roe = self.calculate_return_on_equity(stock).await?;
        let roa = self.calculate_return_on_assets(stock).await?;

        // Calculate growth metrics
        let revenue_growth_1y = self.calculate_revenue_growth_1y(stock).await?;
        let revenue_growth_3y = self.calculate_revenue_growth_3y(stock).await?;

        // Generate reasoning
        let reasoning = self.generate_reasoning(
            stock, pe_ratio, pb_ratio, dividend_yield, debt_to_equity, 
            profit_margin, revenue_growth_1y, &sector_adj
        );

        Ok(GrahamScreeningResult {
            id: None,
            stock_id: stock.stock_id,
            symbol: stock.symbol.clone(),
            screening_date: Local::now().date_naive().to_string(),
            
            // Core metrics
            pe_ratio,
            pb_ratio,
            pe_pb_product,
            dividend_yield,
            debt_to_equity,
            profit_margin,
            revenue_growth_1y,
            revenue_growth_3y,
            
            // Additional quality metrics
            current_ratio,
            quick_ratio: None, // Would need quick assets data
            interest_coverage_ratio: interest_coverage,
            return_on_equity: roe,
            return_on_assets: roa,
            
            // Filters (will be calculated later)
            passes_earnings_filter: false,
            passes_pe_filter: false,
            passes_pb_filter: false,
            passes_pe_pb_combined: false,
            passes_dividend_filter: false,
            passes_debt_filter: false,
            passes_quality_filter: false,
            passes_growth_filter: false,
            passes_all_filters: false,
            
            // Scores (will be calculated later)
            graham_score: None,
            value_rank: None,
            quality_score: None,
            safety_score: None,
            
            // Financial snapshot
            current_price: stock.current_price,
            market_cap: stock.current_price.and_then(|price| 
                stock.shares_outstanding.map(|shares| price * shares)
            ),
            shares_outstanding: stock.shares_outstanding,
            net_income: stock.net_income,
            total_equity: stock.total_equity,
            total_debt: stock.total_debt,
            revenue: stock.revenue,
            
            // Context
            reasoning: Some(reasoning),
            sector: stock.sector.clone(),
            industry: stock.industry.clone(),
            
            // Metadata
            created_at: None,
            updated_at: None,
        })
    }

    /// Apply all Graham screening filters to results
    async fn apply_screening_filters(
        &self,
        results: &mut [GrahamScreeningResult],
        criteria: &GrahamScreeningCriteria,
    ) -> Result<()> {
        for result in results.iter_mut() {
            let sector_adj = get_sector_adjustments(
                result.sector.as_deref().unwrap_or("General")
            );

            // Apply sector-adjusted criteria
            let adjusted_max_pe = criteria.max_pe_ratio * sector_adj.pe_multiplier;
            let adjusted_max_pb = criteria.max_pb_ratio * sector_adj.pb_multiplier;
            let adjusted_min_margin = criteria.min_profit_margin + sector_adj.margin_adjustment;
            let adjusted_max_debt = criteria.max_debt_to_equity * sector_adj.debt_tolerance;

            // Positive earnings filter
            result.passes_earnings_filter = if criteria.require_positive_earnings {
                result.net_income.map_or(false, |income| income > 0.0)
            } else {
                true
            };

            // P/E ratio filter
            result.passes_pe_filter = result.pe_ratio
                .map_or(false, |pe| pe > 0.0 && pe <= adjusted_max_pe);

            // P/B ratio filter
            result.passes_pb_filter = result.pb_ratio
                .map_or(false, |pb| pb > 0.0 && pb <= adjusted_max_pb);

            // Combined P/E × P/B filter (Graham's flexible approach)
            result.passes_pe_pb_combined = if let Some(product) = result.pe_pb_product {
                product <= criteria.max_pe_pb_product || 
                (result.passes_pe_filter && result.passes_pb_filter)
            } else {
                false
            };

            // Dividend yield filter
            result.passes_dividend_filter = if criteria.require_dividend {
                result.dividend_yield
                    .map_or(false, |yield_pct| yield_pct >= criteria.min_dividend_yield)
            } else {
                true // Optional filter
            };

            // Debt-to-equity filter
            result.passes_debt_filter = result.debt_to_equity
                .map_or(true, |debt_ratio| debt_ratio <= adjusted_max_debt);

            // Quality filter (profit margin)
            result.passes_quality_filter = result.profit_margin
                .map_or(false, |margin| margin >= adjusted_min_margin);

            // Growth filter (revenue stability/growth)
            result.passes_growth_filter = result.revenue_growth_1y
                .map_or(true, |growth| growth >= criteria.min_revenue_growth_1y);

            // Market cap filter
            let market_cap_ok = if let Some(market_cap) = result.market_cap {
                market_cap >= criteria.min_market_cap &&
                criteria.max_market_cap.map_or(true, |max| market_cap <= max)
            } else {
                false
            };

            // Sector exclusion filter
            let sector_ok = result.sector.as_ref().map_or(true, |sector| 
                !criteria.excluded_sectors.contains(sector)
            );

            // Combined filter result
            result.passes_all_filters = result.passes_earnings_filter &&
                result.passes_pe_filter &&
                result.passes_pb_filter &&
                result.passes_pe_pb_combined &&
                result.passes_dividend_filter &&
                result.passes_debt_filter &&
                result.passes_quality_filter &&
                result.passes_growth_filter &&
                market_cap_ok &&
                sector_ok;
        }

        Ok(())
    }

    /// Calculate composite Graham scores and rankings
    async fn calculate_scores_and_rankings(&self, results: &mut [GrahamScreeningResult]) -> Result<()> {
        // Calculate individual component scores
        for result in results.iter_mut() {
            result.graham_score = self.calculate_graham_composite_score(result);
            result.quality_score = self.calculate_quality_score(result);
            result.safety_score = self.calculate_safety_score(result);
        }

        // Sort by Graham score for ranking
        let mut passing_results: Vec<_> = results.iter_mut()
            .filter(|r| r.passes_all_filters)
            .collect();
        
        passing_results.sort_by(|a, b| {
            b.graham_score.partial_cmp(&a.graham_score).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Assign rankings
        for (rank, result) in passing_results.iter_mut().enumerate() {
            result.value_rank = Some((rank + 1) as i32);
        }

        Ok(())
    }

    /// Save screening results to database
    async fn save_screening_results(&self, results: &[GrahamScreeningResult]) -> Result<()> {
        let today = Local::now().date_naive().to_string();

        // Clear existing results for today
        sqlx::query("DELETE FROM graham_screening_results WHERE screening_date = ?")
            .bind(&today)
            .execute(&self.db_pool)
            .await?;

        // Insert new results
        for result in results {
            sqlx::query(r#"
                INSERT INTO graham_screening_results (
                    stock_id, symbol, screening_date,
                    pe_ratio, pb_ratio, pe_pb_product, dividend_yield, debt_to_equity,
                    profit_margin, revenue_growth_1y, revenue_growth_3y,
                    current_ratio, interest_coverage_ratio, return_on_equity, return_on_assets,
                    passes_earnings_filter, passes_pe_filter, passes_pb_filter, passes_pe_pb_combined,
                    passes_dividend_filter, passes_debt_filter, passes_quality_filter, passes_growth_filter,
                    passes_all_filters, graham_score, value_rank, quality_score, safety_score,
                    current_price, market_cap, shares_outstanding, net_income, total_equity, total_debt, revenue,
                    reasoning, sector, industry
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#)
            .bind(result.stock_id)
            .bind(&result.symbol)
            .bind(&result.screening_date)
            .bind(result.pe_ratio)
            .bind(result.pb_ratio)
            .bind(result.pe_pb_product)
            .bind(result.dividend_yield)
            .bind(result.debt_to_equity)
            .bind(result.profit_margin)
            .bind(result.revenue_growth_1y)
            .bind(result.revenue_growth_3y)
            .bind(result.current_ratio)
            .bind(result.interest_coverage_ratio)
            .bind(result.return_on_equity)
            .bind(result.return_on_assets)
            .bind(result.passes_earnings_filter)
            .bind(result.passes_pe_filter)
            .bind(result.passes_pb_filter)
            .bind(result.passes_pe_pb_combined)
            .bind(result.passes_dividend_filter)
            .bind(result.passes_debt_filter)
            .bind(result.passes_quality_filter)
            .bind(result.passes_growth_filter)
            .bind(result.passes_all_filters)
            .bind(result.graham_score)
            .bind(result.value_rank)
            .bind(result.quality_score)
            .bind(result.safety_score)
            .bind(result.current_price)
            .bind(result.market_cap)
            .bind(result.shares_outstanding)
            .bind(result.net_income)
            .bind(result.total_equity)
            .bind(result.total_debt)
            .bind(result.revenue)
            .bind(&result.reasoning)
            .bind(&result.sector)
            .bind(&result.industry)
            .execute(&self.db_pool)
            .await?;
        }

        println!("💾 Saved {} Graham screening results to database", results.len());
        Ok(())
    }

    /// Enhance results with additional details for frontend display
    async fn enhance_results_with_details(
        &self,
        results: Vec<GrahamScreeningResult>,
    ) -> Result<Vec<GrahamScreeningResultWithDetails>> {
        let mut enhanced_results = Vec::new();

        for result in results {
            let enhanced = GrahamScreeningResultWithDetails {
                company_name: None, // Would need to join with stocks table
                is_sp500: true,     // All our results are S&P 500
                exchange: None,     // Would need additional data
                
                value_category: Self::categorize_value_level(&result),
                safety_category: Self::categorize_safety_level(&result),
                recommendation: Self::generate_recommendation(&result),
                
                pe_percentile: None,  // Would need percentile calculations
                pb_percentile: None,
                sector_pe_rank: None, // Would need sector comparisons
                sector_pb_rank: None,
                
                result,
            };
            enhanced_results.push(enhanced);
        }

        Ok(enhanced_results)
    }

    // ============================================================================
    // Calculation Methods
    // ============================================================================

    async fn calculate_pe_ratio(&self, stock: &StockFinancialData) -> Result<Option<f64>> {
        if let (Some(price), Some(net_income), Some(shares)) = 
            (stock.current_price, stock.net_income, stock.shares_outstanding) {
            if net_income > 0.0 && shares > 0.0 {
                let eps = net_income / shares;
                Ok(Some(price / eps))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    async fn calculate_pb_ratio(&self, stock: &StockFinancialData) -> Result<Option<f64>> {
        if let (Some(price), Some(equity), Some(shares)) = 
            (stock.current_price, stock.total_equity, stock.shares_outstanding) {
            if equity > 0.0 && shares > 0.0 {
                let book_value_per_share = equity / shares;
                Ok(Some(price / book_value_per_share))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    async fn calculate_dividend_yield(&self, stock: &StockFinancialData) -> Result<Option<f64>> {
        if let (Some(dividend), Some(price)) = (stock.dividend_per_share, stock.current_price) {
            if price > 0.0 {
                Ok(Some((dividend / price) * 100.0))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    async fn calculate_debt_to_equity(&self, stock: &StockFinancialData) -> Result<Option<f64>> {
        if let (Some(debt), Some(equity)) = (stock.total_debt, stock.total_equity) {
            if equity > 0.0 {
                Ok(Some(debt / equity))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    async fn calculate_profit_margin(&self, stock: &StockFinancialData) -> Result<Option<f64>> {
        if let (Some(net_income), Some(revenue)) = (stock.net_income, stock.revenue) {
            if revenue > 0.0 {
                Ok(Some((net_income / revenue) * 100.0))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    async fn calculate_current_ratio(&self, stock: &StockFinancialData) -> Result<Option<f64>> {
        if let (Some(current_assets), Some(current_liabilities)) = 
            (stock.current_assets, stock.current_liabilities) {
            if current_liabilities > 0.0 {
                Ok(Some(current_assets / current_liabilities))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    async fn calculate_interest_coverage(&self, stock: &StockFinancialData) -> Result<Option<f64>> {
        if let (Some(operating_income), Some(interest_expense)) = 
            (stock.operating_income, stock.interest_expense) {
            if interest_expense > 0.0 {
                Ok(Some(operating_income / interest_expense))
            } else {
                Ok(Some(f64::INFINITY)) // No interest expense = infinite coverage
            }
        } else {
            Ok(None)
        }
    }

    async fn calculate_return_on_equity(&self, stock: &StockFinancialData) -> Result<Option<f64>> {
        if let (Some(net_income), Some(equity)) = (stock.net_income, stock.total_equity) {
            if equity > 0.0 {
                Ok(Some((net_income / equity) * 100.0))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    async fn calculate_return_on_assets(&self, stock: &StockFinancialData) -> Result<Option<f64>> {
        if let (Some(net_income), Some(assets)) = (stock.net_income, stock.total_assets) {
            if assets > 0.0 {
                Ok(Some((net_income / assets) * 100.0))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    async fn calculate_revenue_growth_1y(&self, stock: &StockFinancialData) -> Result<Option<f64>> {
        if let (Some(current), Some(previous)) = (stock.revenue, stock.revenue_1y_ago) {
            if previous > 0.0 {
                Ok(Some(((current - previous) / previous) * 100.0))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    async fn calculate_revenue_growth_3y(&self, stock: &StockFinancialData) -> Result<Option<f64>> {
        if let (Some(current), Some(three_years_ago)) = (stock.revenue, stock.revenue_3y_ago) {
            if three_years_ago > 0.0 {
                // Calculate compound annual growth rate
                let years = 3.0;
                let growth_rate = ((current / three_years_ago).powf(1.0 / years) - 1.0) * 100.0;
                Ok(Some(growth_rate))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    // ============================================================================
    // Scoring and Categorization Methods
    // ============================================================================

    fn calculate_graham_composite_score(&self, result: &GrahamScreeningResult) -> Option<f64> {
        if !result.passes_all_filters {
            return Some(0.0);
        }

        let mut score = 0.0;
        let mut total_weight = 0.0;

        // Valuation component (35% - lower P/E and P/B are better)
        if let (Some(pe), Some(pb)) = (result.pe_ratio, result.pb_ratio) {
            let valuation_score = 100.0 - ((pe / 25.0 + pb / 3.0) * 25.0).min(100.0);
            score += valuation_score * self.scoring_weights.valuation_weight;
            total_weight += self.scoring_weights.valuation_weight;
        }

        // Safety component (25% - lower debt, higher current ratio)
        if let Some(debt_ratio) = result.debt_to_equity {
            let debt_score = (100.0 - (debt_ratio * 50.0)).max(0.0);
            let safety_score = if let Some(current_ratio) = result.current_ratio {
                (debt_score + (current_ratio * 25.0).min(100.0)) / 2.0
            } else {
                debt_score
            };
            score += safety_score * self.scoring_weights.safety_weight;
            total_weight += self.scoring_weights.safety_weight;
        }

        // Quality component (20% - ROE and profit margins)
        if let Some(margin) = result.profit_margin {
            let quality_score = if let Some(roe) = result.return_on_equity {
                (margin * 5.0 + roe * 3.0).min(100.0)
            } else {
                (margin * 8.0).min(100.0)
            };
            score += quality_score * self.scoring_weights.quality_weight;
            total_weight += self.scoring_weights.quality_weight;
        }

        // Dividend component (15% - higher yield is better)
        if let Some(dividend_yield) = result.dividend_yield {
            let dividend_score = (dividend_yield * 20.0).min(100.0);
            score += dividend_score * self.scoring_weights.dividend_weight;
            total_weight += self.scoring_weights.dividend_weight;
        }

        // Growth component (5% - modest positive growth)
        if let Some(growth) = result.revenue_growth_1y {
            let growth_score = if growth >= 0.0 {
                (50.0 + growth * 2.0).min(100.0)
            } else {
                (50.0 + growth * 5.0).max(0.0) // Penalize decline more
            };
            score += growth_score * self.scoring_weights.growth_weight;
            total_weight += self.scoring_weights.growth_weight;
        }

        if total_weight > 0.0 {
            Some(score / total_weight)
        } else {
            None
        }
    }

    fn calculate_quality_score(&self, result: &GrahamScreeningResult) -> Option<f64> {
        let mut components = Vec::new();

        if let Some(margin) = result.profit_margin {
            components.push((margin / 20.0 * 100.0).min(100.0));
        }

        if let Some(roe) = result.return_on_equity {
            components.push((roe / 25.0 * 100.0).min(100.0));
        }

        if let Some(roa) = result.return_on_assets {
            components.push((roa / 15.0 * 100.0).min(100.0));
        }

        if let Some(interest_cov) = result.interest_coverage_ratio {
            let cov_score = if interest_cov.is_infinite() {
                100.0
            } else {
                (interest_cov / 10.0 * 100.0).min(100.0)
            };
            components.push(cov_score);
        }

        if !components.is_empty() {
            Some(components.iter().sum::<f64>() / components.len() as f64)
        } else {
            None
        }
    }

    fn calculate_safety_score(&self, result: &GrahamScreeningResult) -> Option<f64> {
        let mut safety_score: f64 = 50.0; // Base score

        // Debt safety
        if let Some(debt_ratio) = result.debt_to_equity {
            safety_score += match debt_ratio {
                x if x <= 0.3 => 25.0,
                x if x <= 0.6 => 15.0,
                x if x <= 1.0 => 5.0,
                _ => -10.0,
            };
        }

        // Liquidity safety
        if let Some(current_ratio) = result.current_ratio {
            safety_score += match current_ratio {
                x if x >= 2.5 => 25.0,
                x if x >= 2.0 => 15.0,
                x if x >= 1.5 => 10.0,
                x if x >= 1.0 => 0.0,
                _ => -15.0,
            };
        }

        Some(safety_score.max(0.0).min(100.0))
    }

    pub fn categorize_value_level(result: &GrahamScreeningResult) -> String {
        if let (Some(pe), Some(pb)) = (result.pe_ratio, result.pb_ratio) {
            if pe < 10.0 && pb < 1.0 {
                "Deep Value".to_string()
            } else if pe < 15.0 && pb < 1.5 {
                "Moderate Value".to_string()
            } else {
                "Fair Value".to_string()
            }
        } else {
            "Unknown".to_string()
        }
    }

    pub fn categorize_safety_level(result: &GrahamScreeningResult) -> String {
        if let Some(safety_score) = result.safety_score {
            match safety_score {
                x if x >= 80.0 => "Very Safe".to_string(),
                x if x >= 65.0 => "Safe".to_string(),
                x if x >= 50.0 => "Moderate".to_string(),
                _ => "Risky".to_string(),
            }
        } else {
            "Unknown".to_string()
        }
    }

    pub fn generate_recommendation(result: &GrahamScreeningResult) -> String {
        if let Some(graham_score) = result.graham_score {
            match graham_score {
                x if x >= 85.0 => "Strong Buy".to_string(),
                x if x >= 70.0 => "Buy".to_string(),
                x if x >= 55.0 => "Hold".to_string(),
                _ => "Avoid".to_string(),
            }
        } else {
            "No Rating".to_string()
        }
    }

    fn generate_reasoning(
        &self,
        stock: &StockFinancialData,
        pe_ratio: Option<f64>,
        pb_ratio: Option<f64>,
        dividend_yield: Option<f64>,
        debt_to_equity: Option<f64>,
        profit_margin: Option<f64>,
        revenue_growth: Option<f64>,
        sector_adj: &SectorAdjustments,
    ) -> String {
        let mut reasons = Vec::new();

        if let Some(pe) = pe_ratio {
            if pe <= 12.0 {
                reasons.push(format!("Excellent P/E of {:.1}", pe));
            } else if pe <= 15.0 {
                reasons.push(format!("Good P/E of {:.1}", pe));
            } else {
                reasons.push(format!("High P/E of {:.1}", pe));
            }
        }

        if let Some(pb) = pb_ratio {
            if pb <= 1.0 {
                reasons.push(format!("Trading below book value (P/B: {:.1})", pb));
            } else if pb <= 1.5 {
                reasons.push(format!("Reasonable P/B of {:.1}", pb));
            }
        }

        if let Some(yield_pct) = dividend_yield {
            if yield_pct >= 3.0 {
                reasons.push(format!("Strong dividend yield of {:.1}%", yield_pct));
            } else if yield_pct >= 2.0 {
                reasons.push(format!("Decent dividend yield of {:.1}%", yield_pct));
            }
        }

        if let Some(debt) = debt_to_equity {
            if debt <= 0.5 {
                reasons.push("Low debt levels".to_string());
            } else if debt <= 1.0 {
                reasons.push("Moderate debt levels".to_string());
            } else {
                reasons.push("High debt concern".to_string());
            }
        }

        if let Some(margin) = profit_margin {
            if margin >= 10.0 {
                reasons.push(format!("Strong profit margins ({:.1}%)", margin));
            } else if margin >= 5.0 {
                reasons.push(format!("Decent profit margins ({:.1}%)", margin));
            }
        }

        if let Some(growth) = revenue_growth {
            if growth >= 10.0 {
                reasons.push(format!("Strong revenue growth ({:.1}%)", growth));
            } else if growth >= 0.0 {
                reasons.push("Stable revenue".to_string());
            } else {
                reasons.push(format!("Revenue decline ({:.1}%)", growth));
            }
        }

        if sector_adj.sector != "General" {
            reasons.push(format!("Sector-adjusted criteria for {}", sector_adj.sector));
        }

        if reasons.is_empty() {
            format!("Limited financial data available for {}", stock.symbol)
        } else {
            reasons.join("; ")
        }
    }

    /// Get screening statistics for dashboard display
    pub async fn get_screening_stats(&self, date: Option<String>) -> Result<Option<GrahamScreeningStats>> {
        let screening_date = date.unwrap_or_else(|| Local::now().date_naive().to_string());
        
        let row = sqlx::query("SELECT * FROM v_graham_screening_stats WHERE screening_date = ?")
            .bind(&screening_date)
            .fetch_optional(&self.db_pool)
            .await?;

        if let Some(row) = row {
            Ok(Some(GrahamScreeningStats {
                screening_date: row.get("screening_date"),
                total_screened: row.get("total_screened"),
                passed_all_filters: row.get("passed_all_filters"),
                passed_earnings: row.get("passed_earnings"),
                passed_pe: row.get("passed_pe"),
                passed_pb: row.get("passed_pb"),
                passed_dividend: row.get("passed_dividend"),
                passed_debt: row.get("passed_debt"),
                passed_quality: row.get("passed_quality"),
                passed_growth: row.get("passed_growth"),
                avg_pe_ratio: row.try_get("avg_pe_ratio").ok(),
                avg_pb_ratio: row.try_get("avg_pb_ratio").ok(),
                avg_graham_score: row.try_get("avg_graham_score").ok(),
                min_graham_score: row.try_get("min_graham_score").ok(),
                max_graham_score: row.try_get("max_graham_score").ok(),
            }))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sector_adjustments() {
        let tech_adj = get_sector_adjustments("Technology");
        assert_eq!(tech_adj.pe_multiplier, 1.5);
        
        let util_adj = get_sector_adjustments("Utilities");
        assert_eq!(util_adj.pe_multiplier, 0.8);
    }

    #[test]
    fn test_value_categorization() {
        let mut result = GrahamScreeningResult {
            pe_ratio: Some(8.0),
            pb_ratio: Some(0.8),
            ..Default::default()
        };
        
        assert_eq!(GrahamScreener::categorize_value_level(&result), "Deep Value");
        
        result.pe_ratio = Some(18.0);
        result.pb_ratio = Some(2.0);
        assert_eq!(GrahamScreener::categorize_value_level(&result), "Fair Value");
    }

    // Additional tests for calculation methods would go here
}