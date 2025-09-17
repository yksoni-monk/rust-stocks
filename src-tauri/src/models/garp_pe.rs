use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GarpPeScreeningResult {
    pub stock_id: i32,
    pub symbol: String,
    pub sector: Option<String>,
    
    // P/E and PEG Analysis
    pub current_pe_ratio: f64,
    pub peg_ratio: Option<f64>,
    pub current_price: f64,
    pub passes_positive_earnings: bool,
    pub passes_peg_filter: bool,
    
    // EPS Analysis
    pub current_eps_ttm: Option<f64>,
    pub current_eps_annual: Option<f64>,
    pub eps_growth_rate_ttm: Option<f64>,
    pub eps_growth_rate_annual: Option<f64>,
    
    // Revenue Growth Analysis
    pub current_ttm_revenue: Option<f64>,
    pub ttm_growth_rate: Option<f64>,
    pub current_annual_revenue: Option<f64>,
    pub annual_growth_rate: Option<f64>,
    pub passes_revenue_growth_filter: bool,
    
    // Profitability Analysis
    pub current_ttm_net_income: Option<f64>,
    pub net_profit_margin: Option<f64>,
    pub passes_profitability_filter: bool,
    
    // Debt Analysis
    pub total_debt: Option<f64>,
    pub total_equity: Option<f64>,
    pub debt_to_equity_ratio: Option<f64>,
    pub passes_debt_filter: bool,
    
    // GARP Scoring
    pub garp_score: f64,
    pub quality_score: i32,
    pub passes_garp_screening: bool,
    
    // Market Metrics
    pub market_cap: f64,
    pub data_completeness_score: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GarpPeScreeningCriteria {
    #[serde(rename = "maxPegRatio")]
    pub max_peg_ratio: f64,           // Default: 1.0
    #[serde(rename = "minRevenueGrowth")]
    pub min_revenue_growth: f64,      // Default: 15.0% (TTM) or 10.0% (Annual)
    #[serde(rename = "minProfitMargin")]
    pub min_profit_margin: f64,       // Default: 5.0%
    #[serde(rename = "maxDebtToEquity")]
    pub max_debt_to_equity: f64,      // Default: 2.0
    #[serde(rename = "minMarketCap")]
    pub min_market_cap: f64,          // Default: $500M
    #[serde(rename = "minQualityScore")]
    pub min_quality_score: i32,       // Default: 50
    #[serde(rename = "requirePositiveEarnings")]
    pub require_positive_earnings: bool, // Default: true
}

impl Default for GarpPeScreeningCriteria {
    fn default() -> Self {
        Self {
            max_peg_ratio: 1.0,              // PEG < 1.0
            min_revenue_growth: 15.0,        // 15% TTM or 10% Annual
            min_profit_margin: 5.0,          // 5% net margin
            max_debt_to_equity: 2.0,         // D/E < 2.0
            min_market_cap: 500_000_000.0,   // $500M minimum
            min_quality_score: 50,           // Minimum data quality
            require_positive_earnings: true, // Net Income > 0
        }
    }
}
