// Benjamin Graham Value Screening Models
// Implementation of Graham's value investing principles for modern stock analysis

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Graham screening criteria based on "The Intelligent Investor" principles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrahamScreeningCriteria {
    // Core valuation filters
    pub max_pe_ratio: f64,
    pub max_pb_ratio: f64,
    pub max_pe_pb_product: f64,
    
    // Income and dividend requirements
    pub min_dividend_yield: f64,
    pub min_profit_margin: f64,
    pub require_positive_earnings: bool,
    pub require_dividend: bool,
    
    // Financial health metrics
    pub max_debt_to_equity: f64,
    pub min_current_ratio: f64,
    pub min_interest_coverage: f64,
    
    // Growth and quality filters
    pub min_revenue_growth_1y: f64,
    pub min_revenue_growth_3y: f64,
    pub min_roe: f64,
    
    // Market cap constraints
    pub min_market_cap: f64,
    pub max_market_cap: Option<f64>,
    
    // Sector exclusions
    pub excluded_sectors: Vec<String>,
}

impl Default for GrahamScreeningCriteria {
    fn default() -> Self {
        Self {
            max_pe_ratio: 15.0,
            max_pb_ratio: 1.5,
            max_pe_pb_product: 22.5,
            min_dividend_yield: 2.0,
            min_profit_margin: 5.0,
            require_positive_earnings: true,
            require_dividend: true,
            max_debt_to_equity: 1.0,
            min_current_ratio: 2.0,
            min_interest_coverage: 2.5,
            min_revenue_growth_1y: 0.0,
            min_revenue_growth_3y: 0.0,
            min_roe: 10.0,
            min_market_cap: 1_000_000_000.0, // $1B
            max_market_cap: None,
            excluded_sectors: vec![],
        }
    }
}

/// Individual stock result from Graham screening analysis
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GrahamScreeningResult {
    pub id: Option<i64>,
    pub stock_id: i64,
    pub symbol: String,
    pub screening_date: String,
    
    // Core Graham metrics
    pub pe_ratio: Option<f64>,
    pub pb_ratio: Option<f64>,
    pub pe_pb_product: Option<f64>,
    pub dividend_yield: Option<f64>,
    pub debt_to_equity: Option<f64>,
    pub profit_margin: Option<f64>,
    pub revenue_growth_1y: Option<f64>,
    pub revenue_growth_3y: Option<f64>,
    
    // Additional quality metrics
    pub current_ratio: Option<f64>,
    pub quick_ratio: Option<f64>,
    pub interest_coverage_ratio: Option<f64>,
    pub return_on_equity: Option<f64>,
    pub return_on_assets: Option<f64>,
    
    // Filter results
    pub passes_earnings_filter: bool,
    pub passes_pe_filter: bool,
    pub passes_pb_filter: bool,
    pub passes_pe_pb_combined: bool,
    pub passes_dividend_filter: bool,
    pub passes_debt_filter: bool,
    pub passes_quality_filter: bool,
    pub passes_growth_filter: bool,
    pub passes_all_filters: bool,
    
    // Scores and rankings
    pub graham_score: Option<f64>,
    pub value_rank: Option<i32>,
    pub quality_score: Option<f64>,
    pub safety_score: Option<f64>,
    
    // Financial snapshot
    pub current_price: Option<f64>,
    pub market_cap: Option<f64>,
    pub shares_outstanding: Option<f64>,
    pub net_income: Option<f64>,
    pub total_equity: Option<f64>,
    pub total_debt: Option<f64>,
    pub revenue: Option<f64>,
    
    // Context
    pub reasoning: Option<String>,
    pub sector: Option<String>,
    pub industry: Option<String>,
    
    // Metadata
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Enhanced result with stock details for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrahamScreeningResultWithDetails {
    #[serde(flatten)]
    pub result: GrahamScreeningResult,
    
    // Stock details
    pub company_name: Option<String>,
    pub is_sp500: bool,
    pub exchange: Option<String>,
    
    // Additional calculated fields
    pub value_category: String, // "Deep Value", "Moderate Value", "Fair Value"
    pub safety_category: String, // "Very Safe", "Safe", "Moderate", "Risky"
    pub recommendation: String, // "Strong Buy", "Buy", "Hold", "Avoid"
    
    // Comparative metrics
    pub pe_percentile: Option<f64>, // Percentile vs S&P 500
    pub pb_percentile: Option<f64>,
    pub sector_pe_rank: Option<i32>,
    pub sector_pb_rank: Option<i32>,
}

/// Graham screening preset configuration
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GrahamScreeningPreset {
    pub id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    
    // Core criteria (flatten the criteria struct for database storage)
    pub max_pe_ratio: f64,
    pub max_pb_ratio: f64,
    pub max_pe_pb_product: f64,
    pub min_dividend_yield: f64,
    pub max_debt_to_equity: f64,
    pub min_profit_margin: f64,
    pub min_revenue_growth_1y: f64,
    pub min_revenue_growth_3y: f64,
    pub min_current_ratio: f64,
    pub min_interest_coverage: f64,
    pub min_roe: f64,
    pub require_positive_earnings: bool,
    pub require_dividend: bool,
    pub min_market_cap: f64,
    pub max_market_cap: Option<f64>,
    pub excluded_sectors: String, // JSON array
    
    pub is_default: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl From<GrahamScreeningPreset> for GrahamScreeningCriteria {
    fn from(preset: GrahamScreeningPreset) -> Self {
        let excluded_sectors: Vec<String> = serde_json::from_str(&preset.excluded_sectors)
            .unwrap_or_default();
            
        Self {
            max_pe_ratio: preset.max_pe_ratio,
            max_pb_ratio: preset.max_pb_ratio,
            max_pe_pb_product: preset.max_pe_pb_product,
            min_dividend_yield: preset.min_dividend_yield,
            min_profit_margin: preset.min_profit_margin,
            require_positive_earnings: preset.require_positive_earnings,
            require_dividend: preset.require_dividend,
            max_debt_to_equity: preset.max_debt_to_equity,
            min_current_ratio: preset.min_current_ratio,
            min_interest_coverage: preset.min_interest_coverage,
            min_revenue_growth_1y: preset.min_revenue_growth_1y,
            min_revenue_growth_3y: preset.min_revenue_growth_3y,
            min_roe: preset.min_roe,
            min_market_cap: preset.min_market_cap,
            max_market_cap: preset.max_market_cap,
            excluded_sectors,
        }
    }
}

/// Summary statistics for a Graham screening run
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GrahamScreeningStats {
    pub screening_date: String,
    pub total_screened: i64,
    pub passed_all_filters: i64,
    pub passed_earnings: i64,
    pub passed_pe: i64,
    pub passed_pb: i64,
    pub passed_dividend: i64,
    pub passed_debt: i64,
    pub passed_quality: i64,
    pub passed_growth: i64,
    pub avg_pe_ratio: Option<f64>,
    pub avg_pb_ratio: Option<f64>,
    pub avg_graham_score: Option<f64>,
    pub min_graham_score: Option<f64>,
    pub max_graham_score: Option<f64>,
}

/// Input data structure for Graham calculations
#[derive(Debug, Clone)]
pub struct StockFinancialData {
    pub stock_id: i64,
    pub symbol: String,
    pub company_name: Option<String>,
    pub sector: Option<String>,
    pub industry: Option<String>,
    pub is_sp500: bool,
    
    // Price data
    pub current_price: Option<f64>,
    pub shares_outstanding: Option<f64>,
    
    // Income statement data
    pub revenue: Option<f64>,
    pub net_income: Option<f64>,
    pub operating_income: Option<f64>,
    pub interest_expense: Option<f64>,
    
    // Balance sheet data
    pub total_assets: Option<f64>,
    pub total_equity: Option<f64>,
    pub total_debt: Option<f64>,
    pub current_assets: Option<f64>,
    pub current_liabilities: Option<f64>,
    pub cash_and_equivalents: Option<f64>,
    
    // Historical data for growth calculations
    pub revenue_1y_ago: Option<f64>,
    pub revenue_3y_ago: Option<f64>,
    pub dividend_per_share: Option<f64>,
}

/// Graham scoring weights for composite score calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrahamScoringWeights {
    pub valuation_weight: f64,     // P/E, P/B importance
    pub safety_weight: f64,        // Debt, current ratio importance
    pub quality_weight: f64,       // ROE, profit margin importance
    pub dividend_weight: f64,      // Dividend yield importance
    pub growth_weight: f64,        // Revenue growth importance
}

impl Default for GrahamScoringWeights {
    fn default() -> Self {
        Self {
            valuation_weight: 0.35,  // 35% - most important for Graham
            safety_weight: 0.25,     // 25% - financial stability
            quality_weight: 0.20,    // 20% - business quality
            dividend_weight: 0.15,   // 15% - income generation
            growth_weight: 0.05,     // 5% - least important for Graham
        }
    }
}

/// Sector-specific adjustments for Graham criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectorAdjustments {
    pub sector: String,
    pub pe_multiplier: f64,        // Adjust P/E thresholds
    pub pb_multiplier: f64,        // Adjust P/B thresholds
    pub margin_adjustment: f64,    // Adjust profit margin requirements
    pub debt_tolerance: f64,       // Adjust debt tolerance
}

impl Default for SectorAdjustments {
    fn default() -> Self {
        Self {
            sector: "General".to_string(),
            pe_multiplier: 1.0,
            pb_multiplier: 1.0,
            margin_adjustment: 0.0,
            debt_tolerance: 1.0,
        }
    }
}

/// Standard sector adjustments based on industry characteristics
pub fn get_sector_adjustments(sector: &str) -> SectorAdjustments {
    match sector.to_lowercase().as_str() {
        "technology" => SectorAdjustments {
            sector: "Technology".to_string(),
            pe_multiplier: 1.5,     // Higher P/E tolerance
            pb_multiplier: 2.0,     // Higher P/B tolerance  
            margin_adjustment: 5.0, // Higher margin expectations
            debt_tolerance: 0.5,    // Lower debt tolerance
        },
        "utilities" => SectorAdjustments {
            sector: "Utilities".to_string(),
            pe_multiplier: 0.8,     // Lower P/E tolerance
            pb_multiplier: 0.8,     // Lower P/B tolerance
            margin_adjustment: -2.0, // Lower margin requirements
            debt_tolerance: 2.0,    // Higher debt tolerance
        },
        "financials" | "financial services" => SectorAdjustments {
            sector: "Financials".to_string(),
            pe_multiplier: 0.9,     // Slightly lower P/E
            pb_multiplier: 0.7,     // Much lower P/B (asset-heavy)
            margin_adjustment: 0.0,
            debt_tolerance: 3.0,    // Much higher debt tolerance
        },
        "energy" => SectorAdjustments {
            sector: "Energy".to_string(),
            pe_multiplier: 1.2,     // Higher P/E (cyclical)
            pb_multiplier: 1.1,     
            margin_adjustment: -5.0, // Lower margin requirements (cyclical)
            debt_tolerance: 1.5,    // Higher debt tolerance
        },
        "real estate" => SectorAdjustments {
            sector: "Real Estate".to_string(),
            pe_multiplier: 1.0,
            pb_multiplier: 0.9,     // Asset-heavy industry
            margin_adjustment: -3.0,
            debt_tolerance: 2.5,    // High debt tolerance (REITs)
        },
        _ => SectorAdjustments::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_criteria() {
        let criteria = GrahamScreeningCriteria::default();
        assert_eq!(criteria.max_pe_ratio, 15.0);
        assert_eq!(criteria.max_pb_ratio, 1.5);
        assert_eq!(criteria.max_pe_pb_product, 22.5);
        assert!(criteria.require_positive_earnings);
    }

    #[test]
    fn test_sector_adjustments() {
        let tech_adj = get_sector_adjustments("Technology");
        assert_eq!(tech_adj.pe_multiplier, 1.5);
        assert_eq!(tech_adj.pb_multiplier, 2.0);
        
        let util_adj = get_sector_adjustments("Utilities");
        assert_eq!(util_adj.pe_multiplier, 0.8);
        assert_eq!(util_adj.debt_tolerance, 2.0);
    }

    #[test]
    fn test_preset_to_criteria_conversion() {
        let preset = GrahamScreeningPreset {
            id: Some(1),
            name: "Test".to_string(),
            description: None,
            max_pe_ratio: 20.0,
            max_pb_ratio: 2.0,
            max_pe_pb_product: 30.0,
            min_dividend_yield: 1.5,
            max_debt_to_equity: 1.5,
            min_profit_margin: 3.0,
            min_revenue_growth_1y: 5.0,
            min_revenue_growth_3y: 10.0,
            min_current_ratio: 1.5,
            min_interest_coverage: 2.0,
            min_roe: 8.0,
            require_positive_earnings: true,
            require_dividend: false,
            min_market_cap: 500_000_000.0,
            max_market_cap: Some(10_000_000_000.0),
            excluded_sectors: "[]".to_string(),
            is_default: false,
            created_at: None,
            updated_at: None,
        };

        let criteria: GrahamScreeningCriteria = preset.into();
        assert_eq!(criteria.max_pe_ratio, 20.0);
        assert_eq!(criteria.min_revenue_growth_1y, 5.0);
        assert!(!criteria.require_dividend);
    }
}