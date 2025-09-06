use crate::api::AlphaVantageClient;
use crate::models::Config;

/// Test command to fetch and print earnings data from Alpha Vantage
#[tauri::command]
pub async fn test_alpha_vantage_earnings(symbol: String) -> Result<String, String> {
    println!("Testing Alpha Vantage earnings API for symbol: {}", symbol);
    
    // Load configuration
    let config = Config::from_env()
        .map_err(|e| format!("Failed to load configuration: {}", e))?;
    
    // Create Alpha Vantage client
    let client = AlphaVantageClient::new(config.alpha_vantage_api_key);
    
    // Fetch earnings data
    let earnings_data = client.get_earnings(&symbol).await
        .map_err(|e| format!("Failed to fetch earnings data: {}", e))?;
    
    // Print quarterly EPS data to console
    client.print_quarterly_eps(&earnings_data);
    
    // Return summary for frontend
    let quarterly_count = earnings_data.quarterly_earnings.len();
    let annual_count = earnings_data.annual_earnings.len();
    
    Ok(format!(
        "Successfully fetched earnings data for {}. Found {} quarterly and {} annual earnings records. Check console for detailed output.",
        symbol, quarterly_count, annual_count
    ))
}

/// Test command to fetch and print daily price data from Alpha Vantage
#[tauri::command]
pub async fn test_alpha_vantage_daily(symbol: String, output_size: Option<String>) -> Result<String, String> {
    println!("Testing Alpha Vantage daily data API for symbol: {}", symbol);
    
    // Load configuration
    let config = Config::from_env()
        .map_err(|e| format!("Failed to load configuration: {}", e))?;
    
    // Create Alpha Vantage client
    let client = AlphaVantageClient::new(config.alpha_vantage_api_key);
    
    // Fetch daily data
    let daily_data = client.get_daily_data(&symbol, output_size.as_deref()).await
        .map_err(|e| format!("Failed to fetch daily data: {}", e))?;
    
    // Convert to internal format
    let converted_data = client.convert_daily_data(&daily_data)
        .map_err(|e| format!("Failed to convert daily data: {}", e))?;
    
    // Print daily data to console
    client.print_daily_data(&daily_data, &converted_data);
    
    // Return summary for frontend
    let total_records = converted_data.len();
    let date_range = if total_records > 0 {
        format!("{} to {}", 
                converted_data[0].date.format("%Y-%m-%d"),
                converted_data[total_records-1].date.format("%Y-%m-%d"))
    } else {
        "No data".to_string()
    };
    
    Ok(format!(
        "Successfully fetched daily data for {}. Found {} records from {}. Check console for detailed output.",
        symbol, total_records, date_range
    ))
}
