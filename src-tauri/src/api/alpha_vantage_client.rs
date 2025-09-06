use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{NaiveDate, NaiveDateTime};

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

/// Alpha Vantage daily data structures
#[derive(Debug, Deserialize)]
pub struct AlphaVantageDailyResponse {
    #[serde(rename = "Meta Data")]
    pub meta_data: DailyMetaData,
    #[serde(rename = "Time Series (Daily)")]
    pub time_series: HashMap<String, DailyPriceData>,
}

#[derive(Debug, Deserialize)]
pub struct DailyMetaData {
    #[serde(rename = "1. Information")]
    pub information: String,
    #[serde(rename = "2. Symbol")]
    pub symbol: String,
    #[serde(rename = "3. Last Refreshed")]
    pub last_refreshed: String,
    #[serde(rename = "4. Output Size")]
    pub output_size: String,
    #[serde(rename = "5. Time Zone")]
    pub time_zone: String,
}

#[derive(Debug, Deserialize)]
pub struct DailyPriceData {
    #[serde(rename = "1. open")]
    pub open: String,
    #[serde(rename = "2. high")]
    pub high: String,
    #[serde(rename = "3. low")]
    pub low: String,
    #[serde(rename = "4. close")]
    pub close: String,
    #[serde(rename = "5. volume")]
    pub volume: String,
}

/// Converted daily price data for internal use
#[derive(Debug, Clone)]
pub struct ConvertedDailyPrice {
    pub date: NaiveDate,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: i64,
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

    /// Fetch daily price data for a given symbol
    pub async fn get_daily_data(&self, symbol: &str, output_size: Option<&str>) -> Result<AlphaVantageDailyResponse, String> {
        let output_size_param = output_size.unwrap_or("compact");
        let url = format!(
            "{}?function=TIME_SERIES_DAILY&symbol={}&outputsize={}&apikey={}",
            self.base_url, symbol, output_size_param, self.api_key
        );

        println!("DEBUG: Fetching daily data from Alpha Vantage: {}", url);

        let response = reqwest::get(&url)
            .await
            .map_err(|e| format!("Failed to fetch daily data: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let text = response.text().await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        println!("DEBUG: Alpha Vantage daily response length: {} characters", text.len());

        // Parse JSON response
        let daily_data: AlphaVantageDailyResponse = serde_json::from_str(&text)
            .map_err(|e| format!("Failed to parse daily data: {}", e))?;

        Ok(daily_data)
    }

    /// Convert Alpha Vantage daily data to internal format
    pub fn convert_daily_data(&self, daily_data: &AlphaVantageDailyResponse) -> Result<Vec<ConvertedDailyPrice>, String> {
        let mut converted_data = Vec::new();

        for (date_str, price_data) in &daily_data.time_series {
            // Parse date
            let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                .map_err(|e| format!("Failed to parse date '{}': {}", date_str, e))?;

            // Parse price values
            let open = price_data.open.parse::<f64>()
                .map_err(|e| format!("Failed to parse open price '{}': {}", price_data.open, e))?;
            let high = price_data.high.parse::<f64>()
                .map_err(|e| format!("Failed to parse high price '{}': {}", price_data.high, e))?;
            let low = price_data.low.parse::<f64>()
                .map_err(|e| format!("Failed to parse low price '{}': {}", price_data.low, e))?;
            let close = price_data.close.parse::<f64>()
                .map_err(|e| format!("Failed to parse close price '{}': {}", price_data.close, e))?;
            let volume = price_data.volume.parse::<i64>()
                .map_err(|e| format!("Failed to parse volume '{}': {}", price_data.volume, e))?;

            converted_data.push(ConvertedDailyPrice {
                date,
                open,
                high,
                low,
                close,
                volume,
            });
        }

        // Sort by date (oldest first)
        converted_data.sort_by_key(|d| d.date);

        Ok(converted_data)
    }

    /// Print daily price data to console
    pub fn print_daily_data(&self, daily_data: &AlphaVantageDailyResponse, converted_data: &[ConvertedDailyPrice]) {
        println!("\n=== Daily Price Data for {} ===", daily_data.meta_data.symbol);
        println!("Information: {}", daily_data.meta_data.information);
        println!("Last Refreshed: {}", daily_data.meta_data.last_refreshed);
        println!("Output Size: {}", daily_data.meta_data.output_size);
        println!("Time Zone: {}", daily_data.meta_data.time_zone);
        println!("Total Records: {}", converted_data.len());

        println!("\n{:<12} {:<10} {:<10} {:<10} {:<10} {:<12}", 
                 "Date", "Open", "High", "Low", "Close", "Volume");
        println!("{}", "-".repeat(70));

        // Show first 10 and last 10 records
        let show_count = 10;
        let total = converted_data.len();
        
        if total <= show_count * 2 {
            // Show all if we have few records
            for data in converted_data {
                println!("{:<12} {:<10.2} {:<10.2} {:<10.2} {:<10.2} {:<12}",
                         data.date.format("%Y-%m-%d"),
                         data.open, data.high, data.low, data.close, data.volume);
            }
        } else {
            // Show first 10
            for data in &converted_data[..show_count] {
                println!("{:<12} {:<10.2} {:<10.2} {:<10.2} {:<10.2} {:<12}",
                         data.date.format("%Y-%m-%d"),
                         data.open, data.high, data.low, data.close, data.volume);
            }
            
            println!("... ({} more records) ...", total - show_count * 2);
            
            // Show last 10
            for data in &converted_data[total-show_count..] {
                println!("{:<12} {:<10.2} {:<10.2} {:<10.2} {:<10.2} {:<12}",
                         data.date.format("%Y-%m-%d"),
                         data.open, data.high, data.low, data.close, data.volume);
            }
        }
    }
}
