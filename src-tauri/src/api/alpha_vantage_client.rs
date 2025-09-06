use serde::Deserialize;
use std::collections::HashMap;
use chrono::NaiveDate;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum DataFetchMode {
    Compact,  // 100 days
    Full,     // 20+ years
}

impl ToString for DataFetchMode {
    fn to_string(&self) -> String {
        match self {
            DataFetchMode::Compact => "compact".to_string(),
            DataFetchMode::Full => "full".to_string(),
        }
    }
}

impl From<String> for DataFetchMode {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "full" => DataFetchMode::Full,
            _ => DataFetchMode::Compact,
        }
    }
}

#[derive(Debug)]
pub struct ComprehensiveStockData {
    pub symbol: String,
    pub daily_prices: Vec<ConvertedDailyPrice>,
    pub earnings_data: AlphaVantageEarningsResponse,
    pub calculated_pe_ratios: Vec<DailyPERatio>,
    pub data_quality: DataQualityReport,
    pub fetch_metadata: FetchMetadata,
}

#[derive(Debug)]
pub struct DailyPERatio {
    pub date: NaiveDate,
    pub pe_ratio: Option<f64>,
    pub eps_used: Option<f64>,
    pub closing_price: f64,
    pub calculation_method: PECalculationMethod,
}

#[derive(Debug)]
pub enum PECalculationMethod {
    QuarterlyEPS(NaiveDate), // Date of the quarterly earnings used
    DefaultValue(f64),       // Default value when no EPS available
}

#[derive(Debug)]
pub struct DataQualityReport {
    pub total_records: usize,
    pub missing_data_points: Vec<NaiveDate>,
    pub pe_calculation_coverage: f64, // Percentage of dates with calculated P/E
    pub date_range_start: Option<NaiveDate>,
    pub date_range_end: Option<NaiveDate>,
}

#[derive(Debug)]
pub struct FetchMetadata {
    pub fetch_mode: DataFetchMode,
    pub api_calls_made: u32,
    pub fetch_duration: Duration,
    pub data_source: String,
}

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

        // Check for rate limit or error messages
        if text.contains("rate limit") || text.contains("Information") {
            return Err(format!("Alpha Vantage API rate limit exceeded or error: {}", text));
        }

        // Parse JSON response
        let earnings_data: AlphaVantageEarningsResponse = serde_json::from_str(&text)
            .map_err(|e| format!("Failed to parse earnings data: {} | Response: {}", e, text))?;

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

        // Check for rate limit or error messages
        if text.contains("rate limit") || text.contains("Information") {
            return Err(format!("Alpha Vantage API rate limit exceeded or error: {}", text));
        }

        // Parse JSON response
        let daily_data: AlphaVantageDailyResponse = serde_json::from_str(&text)
            .map_err(|e| format!("Failed to parse daily data: {} | Response: {}", e, text.chars().take(200).collect::<String>()))?;

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

    /// Get the latest EPS for a given date from earnings data
    pub fn get_eps_for_date(&self, earnings_data: &AlphaVantageEarningsResponse, target_date: NaiveDate) -> Result<f64, String> {
        // Parse quarterly earnings and sort by fiscal date (most recent first)
        let mut quarterly_eps: Vec<(NaiveDate, f64)> = Vec::new();
        
        for earning in &earnings_data.quarterly_earnings {
            if let Ok(fiscal_date) = NaiveDate::parse_from_str(&earning.fiscal_date_ending, "%Y-%m-%d") {
                if let Ok(eps) = earning.reported_eps.parse::<f64>() {
                    quarterly_eps.push((fiscal_date, eps));
                }
            }
        }
        
        // Sort by fiscal date (most recent first)
        quarterly_eps.sort_by(|a, b| b.0.cmp(&a.0));
        
        // Find the latest EPS that is <= target_date
        for (fiscal_date, eps) in quarterly_eps {
            if fiscal_date <= target_date {
                return Ok(eps);
            }
        }
        
        Err(format!("No EPS data found for {} on or before {}", earnings_data.symbol, target_date))
    }

    /// Get closing price for a specific date from daily data
    pub fn get_closing_price_for_date(&self, daily_data: &AlphaVantageDailyResponse, target_date: NaiveDate) -> Result<f64, String> {
        let date_str = target_date.format("%Y-%m-%d").to_string();
        
        if let Some(price_data) = daily_data.time_series.get(&date_str) {
            price_data.close.parse::<f64>()
                .map_err(|e| format!("Failed to parse close price '{}': {}", price_data.close, e))
        } else {
            Err(format!("No price data found for {} on {}", daily_data.meta_data.symbol, date_str))
        }
    }

    /// Calculate daily P/E ratio for a given symbol and date
    pub async fn calculate_daily_pe_ratio(&self, symbol: &str, date: NaiveDate) -> Result<f64, String> {
        println!("DEBUG: Calculating P/E ratio for {} on {}", symbol, date.format("%Y-%m-%d"));
        
        // 1. Get earnings data
        let earnings_data = self.get_earnings(symbol).await
            .map_err(|e| format!("Failed to fetch earnings data: {}", e))?;
        
        // 2. Find latest EPS for the date
        let eps = self.get_eps_for_date(&earnings_data, date)
            .map_err(|e| format!("Failed to get EPS for date: {}", e))?;
        
        println!("DEBUG: Found EPS {} for {} on {}", eps, symbol, date.format("%Y-%m-%d"));
        
        // 3. Get daily price data
        let daily_data = self.get_daily_data(symbol, Some("compact")).await
            .map_err(|e| format!("Failed to fetch daily data: {}", e))?;
        
        // 4. Find closing price for the date
        let closing_price = self.get_closing_price_for_date(&daily_data, date)
            .map_err(|e| format!("Failed to get closing price: {}", e))?;
        
        println!("DEBUG: Found closing price {} for {} on {}", closing_price, symbol, date.format("%Y-%m-%d"));
        
        // 5. Calculate P/E ratio
        let pe_ratio = closing_price / eps;
        
        println!("DEBUG: Calculated P/E ratio: {:.2} for {} on {}", pe_ratio, symbol, date.format("%Y-%m-%d"));
        
        Ok(pe_ratio)
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

    /// Comprehensive data fetching with P/E calculations
    pub async fn fetch_comprehensive_daily_data(&self, symbol: &str, fetch_mode: DataFetchMode) -> Result<ComprehensiveStockData, String> {
        use std::time::Instant;
        
        let start_time = Instant::now();
        let mut api_calls = 0;
        
        println!("DEBUG: Starting comprehensive data fetch for {} in {:?} mode", symbol, fetch_mode);
        
        // 1. Fetch daily price data
        let daily_data = self.get_daily_data(symbol, Some(&fetch_mode.to_string())).await?;
        api_calls += 1;
        
        // 2. Convert daily data
        let converted_prices = self.convert_daily_data(&daily_data)?;
        
        // 3. Fetch earnings data
        let earnings_data = self.get_earnings(symbol).await?;
        api_calls += 1;
        
        // 4. Calculate P/E ratios for all dates
        let pe_ratios = self.calculate_pe_ratios_for_price_data(&earnings_data, &converted_prices)?;
        
        // 5. Generate data quality report
        let data_quality = self.generate_data_quality_report(&converted_prices, &pe_ratios);
        
        // 6. Create metadata
        let fetch_metadata = FetchMetadata {
            fetch_mode: fetch_mode.clone(),
            api_calls_made: api_calls,
            fetch_duration: start_time.elapsed(),
            data_source: "alpha_vantage".to_string(),
        };
        
        println!("DEBUG: Comprehensive fetch completed for {} - {} price records, {} P/E calculations", 
                symbol, converted_prices.len(), pe_ratios.len());
        
        Ok(ComprehensiveStockData {
            symbol: symbol.to_string(),
            daily_prices: converted_prices,
            earnings_data,
            calculated_pe_ratios: pe_ratios,
            data_quality,
            fetch_metadata,
        })
    }
    
    /// Calculate P/E ratios for a range of price data
    pub fn calculate_pe_ratios_for_price_data(&self, earnings_data: &AlphaVantageEarningsResponse, price_data: &[ConvertedDailyPrice]) -> Result<Vec<DailyPERatio>, String> {
        let mut pe_ratios = Vec::new();
        
        for price_point in price_data {
            let pe_ratio = match self.get_eps_for_date(earnings_data, price_point.date) {
                Ok(eps) => {
                    if eps > 0.0 {
                        let pe = price_point.close / eps;
                        Some(pe)
                    } else {
                        None // Negative earnings
                    }
                }
                Err(_) => None, // No EPS data available
            };
            
            let calculation_method = match self.get_latest_eps_date_for_date(earnings_data, price_point.date) {
                Ok(eps_date) => PECalculationMethod::QuarterlyEPS(eps_date),
                Err(_) => PECalculationMethod::DefaultValue(0.0),
            };
            
            pe_ratios.push(DailyPERatio {
                date: price_point.date,
                pe_ratio,
                eps_used: self.get_eps_for_date(earnings_data, price_point.date).ok(),
                closing_price: price_point.close,
                calculation_method,
            });
        }
        
        Ok(pe_ratios)
    }
    
    /// Get the date of the latest EPS used for a given date
    pub fn get_latest_eps_date_for_date(&self, earnings_data: &AlphaVantageEarningsResponse, target_date: NaiveDate) -> Result<NaiveDate, String> {
        // Parse quarterly earnings and sort by fiscal date (most recent first)
        let mut quarterly_eps: Vec<NaiveDate> = Vec::new();
        
        for earning in &earnings_data.quarterly_earnings {
            if let Ok(fiscal_date) = NaiveDate::parse_from_str(&earning.fiscal_date_ending, "%Y-%m-%d") {
                quarterly_eps.push(fiscal_date);
            }
        }
        
        // Sort by fiscal date (most recent first)
        quarterly_eps.sort_by(|a, b| b.cmp(&a));
        
        // Find the latest EPS date that is <= target_date
        for fiscal_date in quarterly_eps {
            if fiscal_date <= target_date {
                return Ok(fiscal_date);
            }
        }
        
        Err(format!("No EPS date found for {} on or before {}", earnings_data.symbol, target_date))
    }
    
    /// Generate data quality report
    pub fn generate_data_quality_report(&self, price_data: &[ConvertedDailyPrice], pe_ratios: &[DailyPERatio]) -> DataQualityReport {
        let total_records = price_data.len();
        let successful_pe_calculations = pe_ratios.iter().filter(|pe| pe.pe_ratio.is_some()).count();
        let pe_coverage = if total_records > 0 {
            successful_pe_calculations as f64 / total_records as f64
        } else {
            0.0
        };
        
        let date_range_start = price_data.first().map(|p| p.date);
        let date_range_end = price_data.last().map(|p| p.date);
        
        // Find missing data points (if any gaps in date sequence)
        let missing_data_points = Vec::new(); // TODO: Implement gap detection
        
        DataQualityReport {
            total_records,
            missing_data_points,
            pe_calculation_coverage: pe_coverage,
            date_range_start,
            date_range_end,
        }
    }

    /// Enhanced daily data fetching with mode parameter
    pub async fn get_daily_data_with_mode(&self, symbol: &str, fetch_mode: DataFetchMode) -> Result<AlphaVantageDailyResponse, String> {
        self.get_daily_data(symbol, Some(&fetch_mode.to_string())).await
    }
}
