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
