use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Alpha Vantage earnings data structures
#[derive(Debug, Deserialize)]
pub struct AlphaVantageEarningsResponse {
    pub symbol: String,
    #[serde(rename = "annualEarnings")]
    pub annual_earnings: Vec<AnnualEarning>,
    #[serde(rename = "quarterlyEarnings")]
    pub quarterly_earnings: Vec<QuarterlyEarning>,
}

#[derive(Debug, Deserialize)]
pub struct AnnualEarning {
    #[serde(rename = "fiscalDateEnding")]
    pub fiscal_date_ending: String,
    #[serde(rename = "reportedEPS")]
    pub reported_eps: String,
}

#[derive(Debug, Deserialize)]
pub struct QuarterlyEarning {
    #[serde(rename = "fiscalDateEnding")]
    pub fiscal_date_ending: String,
    #[serde(rename = "reportedDate")]
    pub reported_date: String,
    #[serde(rename = "reportedEPS")]
    pub reported_eps: String,
    #[serde(rename = "estimatedEPS")]
    pub estimated_eps: Option<String>,
    pub surprise: Option<String>,
    #[serde(rename = "surprisePercentage")]
    pub surprise_percentage: Option<String>,
    #[serde(rename = "reportTime")]
    pub report_time: Option<String>,
}

/// Alpha Vantage API client
pub struct AlphaVantageClient {
    api_key: String,
    base_url: String,
}

impl AlphaVantageClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://www.alphavantage.co/query".to_string(),
        }
    }

    /// Fetch earnings data for a given symbol
    pub async fn get_earnings(&self, symbol: &str) -> Result<AlphaVantageEarningsResponse, String> {
        let url = format!(
            "{}?function=EARNINGS&symbol={}&apikey={}",
            self.base_url, symbol, self.api_key
        );

        println!("DEBUG: Fetching earnings from Alpha Vantage: {}", url);

        let response = reqwest::get(&url)
            .await
            .map_err(|e| format!("Failed to fetch earnings data: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let text = response.text().await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        println!("DEBUG: Alpha Vantage response: {}", text);

        // Parse JSON response
        let earnings_data: AlphaVantageEarningsResponse = serde_json::from_str(&text)
            .map_err(|e| format!("Failed to parse earnings data: {}", e))?;

        Ok(earnings_data)
    }

    /// Print quarterly EPS data to console
    pub fn print_quarterly_eps(&self, earnings_data: &AlphaVantageEarningsResponse) {
        println!("\n=== Quarterly EPS Data for {} ===", earnings_data.symbol);
        println!("{:<15} {:<15} {:<12} {:<12} {:<15} {:<10}", 
                 "Fiscal Date", "Reported Date", "Reported EPS", "Estimated EPS", "Surprise %", "Report Time");
        println!("{}", "-".repeat(90));

        for earning in &earnings_data.quarterly_earnings {
            let estimated_eps = earning.estimated_eps.as_deref().unwrap_or("N/A");
            let surprise_pct = earning.surprise_percentage.as_deref().unwrap_or("N/A");
            let report_time = earning.report_time.as_deref().unwrap_or("N/A");

            println!("{:<15} {:<15} {:<12} {:<12} {:<15} {:<10}",
                     earning.fiscal_date_ending,
                     earning.reported_date,
                     earning.reported_eps,
                     estimated_eps,
                     surprise_pct,
                     report_time);
        }

        println!("\n=== Annual EPS Data for {} ===", earnings_data.symbol);
        println!("{:<15} {:<12}", "Fiscal Date", "Reported EPS");
        println!("{}", "-".repeat(30));

        for earning in &earnings_data.annual_earnings {
            println!("{:<15} {:<12}",
                     earning.fiscal_date_ending,
                     earning.reported_eps);
        }
    }
}
