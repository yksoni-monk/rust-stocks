use serde::{Deserialize, Serialize};
use ts_rs::TS;

// Re-export types from other modules for ts-rs generation
pub use crate::tools::data_freshness_checker::{SystemFreshnessReport, DataFreshnessStatus, FreshnessStatus, RefreshPriority, RefreshRecommendation, ScreeningReadiness};
pub use crate::commands::piotroski_screening::{PiotoskiFScoreResult, PiotroskilScreeningCriteria};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum RefreshMode {
    #[serde(rename = "market")]
    Market,
    #[serde(rename = "financials")]
    Financials,
    #[serde(rename = "ratios")]
    Ratios,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RefreshRequestDto {
    pub mode: RefreshMode,
    pub force_sources: Option<Vec<String>>,
    pub initiated_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum RefreshStatus {
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RefreshProgressDto {
    pub session_id: String,
    pub operation_type: RefreshMode,
    pub start_time: String,
    pub total_steps: i32,
    pub completed_steps: i32,
    pub current_step_name: Option<String>,
    pub current_step_progress: f64,
    pub overall_progress_percent: f64,
    pub estimated_completion: Option<String>,
    pub status: RefreshStatus,
    pub initiated_by: String,
    pub elapsed_minutes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RefreshCompletedEvent {
    pub mode: RefreshMode,
    pub session_id: String,
    pub status: RefreshStatus,
}

#[cfg(test)]
mod ts_bindings_export_tests {
    use super::*;

    #[test]
    fn generate_bindings() {
        RefreshMode::export().unwrap();
        RefreshStatus::export().unwrap();
        RefreshRequestDto::export().unwrap();
        RefreshProgressDto::export().unwrap();
        RefreshCompletedEvent::export().unwrap();
        SystemFreshnessReport::export().unwrap();
        DataFreshnessStatus::export().unwrap();
        FreshnessStatus::export().unwrap();
        RefreshPriority::export().unwrap();
        RefreshRecommendation::export().unwrap();
        ScreeningReadiness::export().unwrap();

        // Piotroski F-Score types
        PiotoskiFScoreResult::export().unwrap();
        PiotroskilScreeningCriteria::export().unwrap();
    }
}

