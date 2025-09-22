use serde::{Deserialize, Serialize};

/// Centralized configuration for all data sources
/// This prevents name mismatches between different components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceConfig {
    pub name: String,
    pub display_name: String,
    pub table_name: String,
    pub refresh_mode: RefreshMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RefreshMode {
    Market,
    Financials,
    Ratios,
}

impl DataSourceConfig {
    /// Get all data source configurations
    pub fn all() -> Vec<DataSourceConfig> {
        vec![
            DataSourceConfig {
                name: "daily_prices".to_string(),
                display_name: "Market Data".to_string(),
                table_name: "daily_prices".to_string(),
                refresh_mode: RefreshMode::Market,
            },
            DataSourceConfig {
                name: "financial_statements".to_string(),
                display_name: "Financial Data".to_string(),
                table_name: "financial_statements".to_string(),
                refresh_mode: RefreshMode::Financials,
            },
            DataSourceConfig {
                name: "ps_evs_ratios".to_string(),
                display_name: "Calculated Ratios".to_string(),
                table_name: "ps_evs_ratios".to_string(),
                refresh_mode: RefreshMode::Ratios,
            },
        ]
    }

    /// Get data source by name
    pub fn by_name(name: &str) -> Option<&DataSourceConfig> {
        Self::all().iter().find(|ds| ds.name == name)
    }

    /// Get data sources by refresh mode
    pub fn by_refresh_mode(mode: &RefreshMode) -> Vec<&DataSourceConfig> {
        Self::all().iter().filter(|ds| std::mem::discriminant(&ds.refresh_mode) == std::mem::discriminant(mode)).collect()
    }
}
