use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone)]
pub struct FilingFreshnessResult {
    pub cik: String,
    pub our_latest_date: Option<String>,
    pub sec_latest_date: Option<String>,
    pub is_stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DataFreshnessStatus {
    pub data_source: String,
    pub status: FreshnessStatus,
    pub latest_data_date: Option<String>, // Changed to String for TS compatibility
    pub last_refresh: Option<String>, // Changed to String for TS compatibility
    pub staleness_days: Option<i64>,
    pub records_count: i64,
    pub message: String,
    pub refresh_priority: RefreshPriority,
    // Enhanced fields for better UX
    pub data_summary: DataSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DataSummary {
    pub date_range: Option<String>,
    pub stock_count: Option<i64>,
    pub data_types: Vec<String>,
    pub key_metrics: Vec<String>,
    pub completeness_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export)]
pub enum FreshnessStatus {
    Current,
    Stale,
    Missing,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, TS)]
#[ts(export)]
pub enum RefreshPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SystemFreshnessReport {
    pub overall_status: FreshnessStatus,
    pub market_data: DataFreshnessStatus,
    pub financial_data: DataFreshnessStatus,
    pub calculated_ratios: DataFreshnessStatus,
    pub recommendations: Vec<RefreshRecommendation>,
    pub screening_readiness: ScreeningReadiness,
    pub last_check: String, // Changed to String for TS compatibility
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RefreshRecommendation {
    pub action: String,
    pub reason: String,
    pub estimated_duration: String,
    pub priority: RefreshPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ScreeningReadiness {
    pub valuation_analysis: bool,
    pub blocking_issues: Vec<String>,
}

impl FreshnessStatus {
    pub fn is_current(&self) -> bool {
        matches!(self, FreshnessStatus::Current)
    }

    pub fn needs_refresh(&self) -> bool {
        matches!(self, FreshnessStatus::Stale | FreshnessStatus::Missing | FreshnessStatus::Error)
    }
}

impl SystemFreshnessReport {
    pub fn get_stale_components(&self) -> Vec<String> {
        let mut stale_components = Vec::new();
        
        if self.market_data.status.needs_refresh() {
            stale_components.push("market_data".to_string());
        }
        if self.financial_data.status.needs_refresh() {
            stale_components.push("financial_data".to_string());
        }
        if self.calculated_ratios.status.needs_refresh() {
            stale_components.push("calculated_ratios".to_string());
        }
        
        stale_components
    }

    pub fn get_freshness_warning_message(&self) -> String {
        let stale_components = self.get_stale_components();
        if stale_components.len() == 3 {
            "All data sources need refresh - please run latest update".to_string()
        } else if !stale_components.is_empty() {
            format!("Stale data sources: {}", stale_components.join(", "))
        } else {
            "All data sources are current".to_string()
        }
    }

    pub fn get_overall_status(&self) -> FreshnessStatus {
        self.overall_status.clone()
    }

    pub fn is_data_fresh(&self) -> bool {
        self.market_data.status == FreshnessStatus::Current && self.financial_data.status == FreshnessStatus::Current
    }

    pub fn should_show_freshness_warning(&self) -> bool {
        if self.financial_data.records_count == 0 {
            return false;
        }
        
        match self.financial_data.status {
            FreshnessStatus::Stale => true,
            FreshnessStatus::Current => false,
            FreshnessStatus::Missing => false,
            FreshnessStatus::Error => false,
        }
    }

    pub fn get_freshness_specific_message(&self) -> String {
        let stale = self.get_stale_components();
        if stale.is_empty() {
            "All data sources are current".to_string()
        } else {
            format!("Stale data sources: {}", stale.join(", "))
        }
    }
}