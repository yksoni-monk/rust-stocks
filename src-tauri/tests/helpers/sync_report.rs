/// Simple sync report for test purposes
/// This replaces the complex incremental sync that was causing errors
#[derive(Debug, Default, Clone)]
pub struct SyncReport {
    pub sync_strategy: String,
    pub total_duration_ms: u128,
    pub stocks_synced: usize,
    pub daily_prices_synced: usize,
    pub earnings_synced: usize,
    pub metadata_synced: usize,
    pub schema_changes_applied: usize,
}

impl SyncReport {
    /// Create a new sync report with the given strategy
    pub fn new(strategy: &str) -> Self {
        Self {
            sync_strategy: strategy.to_string(),
            total_duration_ms: 0,
            ..Default::default()
        }
    }
    
    /// Create a report indicating no sync was needed
    pub fn no_sync_needed() -> Self {
        Self::new("no_sync_needed")
    }
    
    /// Create a report for production database direct access
    pub fn production_direct() -> Self {
        Self::new("production_direct")
    }
    
    /// Create a report for sample data fallback
    pub fn sample_data_fallback() -> Self {
        Self::new("sample_data_fallback")
    }
    
    /// Create a report for simple copy fallback
    pub fn simple_copy_fallback(duration_ms: u128) -> Self {
        Self {
            sync_strategy: "simple_copy_fallback".to_string(),
            total_duration_ms: duration_ms,
            ..Default::default()
        }
    }
}