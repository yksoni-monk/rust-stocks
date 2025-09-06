use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose};
use chrono::{DateTime, NaiveDate, Utc};
use reqwest::{Client, header::{HeaderMap, HeaderValue}};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn, debug};

use crate::models::{Config, SchwabQuote, SchwabPriceBar, FundamentalData};
use super::{ApiRateLimiter, StockDataProvider};

/// Schwab OAuth token response
#[derive(Debug, Deserialize, Serialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
    token_type: String,
}

/// Token file format (matches Python script output)
#[derive(Debug, Deserialize)]
struct TokenFile {
    #[allow(dead_code)]
    creation_timestamp: f64,  // Changed from i64 to f64 to handle floating point timestamps
    token: TokenData,
}

#[derive(Debug, Deserialize)]
struct TokenData {
    access_token: String,
    refresh_token: String,
    expires_at: i64,
    #[allow(dead_code)]
    expires_in: i64,
    #[allow(dead_code)]
    token_type: String,
    #[allow(dead_code)]
    scope: String,
    #[allow(dead_code)]
    id_token: String,
}

/// Stored token information (internal format)
#[derive(Debug, Clone, Deserialize, Serialize)]
struct StoredTokens {
    access_token: String,
    refresh_token: String,
    expires_at: DateTime<Utc>,
}

/// Alternative token file format that matches the nested structure
#[derive(Debug, Deserialize)]
struct NestedTokenFile {
    #[allow(dead_code)]
    creation_timestamp: f64,
    token: TokenData,
}

/// Schwab API client
pub struct SchwabClient {
    client: Client,
    api_key: String,
    app_secret: String,
    #[allow(dead_code)]
    callback_url: String,
    token_path: String,
    rate_limiter: ApiRateLimiter,
    current_tokens: Arc<Mutex<Option<StoredTokens>>>,
}

impl SchwabClient {
    /// Create a new Schwab client
    pub fn new(config: &Config) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("rust-stocks/1.0")
            .build()?;

        let rate_limiter = ApiRateLimiter::new(config.rate_limit_per_minute);

        let schwab_client = Self {
            client,
            api_key: config.schwab_api_key.clone(),
            app_secret: config.schwab_app_secret.clone(),
            callback_url: config.schwab_callback_url.clone(),
            token_path: config.schwab_token_path.clone(),
            rate_limiter,
            current_tokens: Arc::new(Mutex::new(None)),
        };

        Ok(schwab_client)
    }

    /// Load tokens from file
    async fn load_tokens(&self) -> Result<()> {
        debug!("DEBUG: Attempting to load tokens from path: {}", self.token_path);
        debug!("DEBUG: Current working directory: {:?}", std::env::current_dir());
        debug!("DEBUG: Token file exists: {}", std::path::Path::new(&self.token_path).exists());
        
        if !std::path::Path::new(&self.token_path).exists() {
            debug!("DEBUG: Token file does not exist at: {}", self.token_path);
            return Err(anyhow!("Token file does not exist: {}", self.token_path));
        }

        debug!("DEBUG: Reading token file content...");
        let content = fs::read_to_string(&self.token_path)?;
        debug!("DEBUG: Token file content length: {} bytes", content.len());
        debug!("DEBUG: Token file content preview: {}", &content[..content.len().min(200)]);
        
        // Try to parse the Python-generated token file format first
        debug!("DEBUG: Attempting to parse TokenFile format...");
        let tokens = match serde_json::from_str::<TokenFile>(&content) {
            Ok(token_file) => {
                debug!("DEBUG: Successfully parsed TokenFile format");
                debug!("DEBUG: Access token length: {}", token_file.token.access_token.len());
                debug!("DEBUG: Expires at timestamp: {}", token_file.token.expires_at);
                StoredTokens {
                    access_token: token_file.token.access_token,
                    refresh_token: token_file.token.refresh_token,
                    expires_at: DateTime::from_timestamp(token_file.token.expires_at, 0)
                        .unwrap_or_else(|| Utc::now()),
                }
            }
            Err(e) => {
                debug!("DEBUG: Failed to parse TokenFile format: {}", e);
                debug!("DEBUG: Trying NestedTokenFile format...");
                match serde_json::from_str::<NestedTokenFile>(&content) {
                    Ok(nested_file) => {
                        debug!("DEBUG: Successfully parsed NestedTokenFile format");
                        debug!("DEBUG: Access token length: {}", nested_file.token.access_token.len());
                        debug!("DEBUG: Expires at timestamp: {}", nested_file.token.expires_at);
                        StoredTokens {
                            access_token: nested_file.token.access_token,
                            refresh_token: nested_file.token.refresh_token,
                            expires_at: DateTime::from_timestamp(nested_file.token.expires_at, 0)
                                .unwrap_or_else(|| Utc::now()),
                        }
                    }
                    Err(e2) => {
                        debug!("DEBUG: Failed to parse NestedTokenFile format: {}", e2);
                        debug!("DEBUG: Trying StoredTokens format...");
                        match serde_json::from_str::<StoredTokens>(&content) {
                            Ok(tokens) => {
                                debug!("DEBUG: Successfully parsed StoredTokens format");
                                tokens
                            }
                            Err(e3) => {
                                debug!("DEBUG: Failed to parse StoredTokens format: {}", e3);
                                return Err(anyhow!("Failed to parse token file in all formats: TokenFile: {}, NestedTokenFile: {}, StoredTokens: {}", e, e2, e3));
                            }
                        }
                    }
                }
            }
        };

        // Check if tokens are still valid
        if tokens.expires_at <= Utc::now() {
            warn!("Tokens have expired, will need to refresh");
        } else {
            let time_left = tokens.expires_at - Utc::now();
            info!("Tokens valid for {} more minutes", time_left.num_minutes());
        }

        *self.current_tokens.lock().await = Some(tokens);
        info!("Loaded tokens from {}", self.token_path);
        Ok(())
    }

    /// Save tokens to file
    fn save_tokens(&self, tokens: &StoredTokens) -> Result<()> {
        let content = serde_json::to_string_pretty(tokens)?;
        fs::write(&self.token_path, content)?;
        info!("Saved tokens to {}", self.token_path);
        Ok(())
    }

    /// Get access token, refreshing if necessary
    async fn get_access_token(&self) -> Result<String> {
        debug!("DEBUG: get_access_token called");
        
        // Try to load tokens if we don't have any yet
        {
            let tokens_guard = self.current_tokens.lock().await;
            debug!("DEBUG: Current tokens loaded: {}", tokens_guard.is_some());
            if tokens_guard.is_none() {
                drop(tokens_guard);
                debug!("DEBUG: No tokens loaded, attempting to load from file");
                match self.load_tokens().await {
                    Ok(_) => debug!("DEBUG: Successfully loaded tokens"),
                    Err(e) => debug!("DEBUG: Failed to load tokens: {}", e),
                }
            }
        }

        let tokens_guard = self.current_tokens.lock().await;
        if let Some(tokens) = &*tokens_guard {
            debug!("DEBUG: Found tokens, checking expiration");
            debug!("DEBUG: Token expires at: {}", tokens.expires_at);
            debug!("DEBUG: Current time: {}", Utc::now());
            debug!("DEBUG: Token is valid: {}", tokens.expires_at > Utc::now() + chrono::Duration::minutes(5));
            
            if tokens.expires_at > Utc::now() + chrono::Duration::minutes(5) {
                debug!("DEBUG: Returning valid access token");
                return Ok(tokens.access_token.clone());
            }

            debug!("DEBUG: Token expired or expiring soon, attempting refresh");
            // Try to refresh the token
            let refresh_token = tokens.refresh_token.clone();
            drop(tokens_guard); // Release the lock before async call
            
            match self.refresh_access_token(&refresh_token).await {
                Ok(new_tokens) => {
                    debug!("DEBUG: Successfully refreshed token");
                    *self.current_tokens.lock().await = Some(new_tokens.clone());
                    self.save_tokens(&new_tokens)?;
                    return Ok(new_tokens.access_token);
                }
                Err(e) => {
                    warn!("Failed to refresh token: {}", e);
                }
            }
        } else {
            debug!("DEBUG: No tokens available in memory");
        }

        debug!("DEBUG: Returning error - no valid access token");
        Err(anyhow!("No valid access token available. Please run initial authentication."))
    }

    /// Refresh access token using refresh token
    async fn refresh_access_token(&self, refresh_token: &str) -> Result<StoredTokens> {
        let auth_header = general_purpose::STANDARD.encode(format!("{}:{}", self.api_key, self.app_secret));
        
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", HeaderValue::from_str(&format!("Basic {}", auth_header))?);
        headers.insert("Content-Type", HeaderValue::from_str("application/x-www-form-urlencoded")?);

        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ];

        self.rate_limiter.wait().await;
        
        let response = self.client
            .post("https://api.schwabapi.com/v1/oauth/token")
            .headers(headers)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Token refresh failed: {}", error_text));
        }

        let token_response: TokenResponse = response.json().await?;
        
        let expires_at = Utc::now() + chrono::Duration::seconds(token_response.expires_in - 60); // 1 minute buffer

        Ok(StoredTokens {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_at,
        })
    }

    /// Make authenticated request to Schwab API
    async fn make_request(&self, url: &str) -> Result<Value> {
        let access_token = self.get_access_token().await?;
        
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", HeaderValue::from_str(&format!("Bearer {}", access_token))?);
        headers.insert("Accept", HeaderValue::from_str("application/json")?);

        self.rate_limiter.wait().await;

        debug!("Making request to: {}", url);
        
        let response = self.client
            .get(url)
            .headers(headers)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(anyhow!("API request failed with status {}: {}", status, error_text));
        }

        let json: Value = response.json().await?;
        debug!("API response received: {} bytes", 
               serde_json::to_string(&json).unwrap_or_default().len());
        
        Ok(json)
    }

    /// Get comprehensive fundamental data for a symbol
    pub async fn get_fundamentals(&self, symbol: &str) -> Result<FundamentalData> {
        let url = format!("https://api.schwabapi.com/marketdata/v1/instruments?symbol={}&projection=fundamental", symbol);
        let data = self.make_request(&url).await?;
        
        let mut fundamental_data = FundamentalData {
            symbol: symbol.to_string(),
            pe_ratio: None,
            pe_ratio_forward: None,
            market_cap: None,
            dividend_yield: None,
            dividend_per_share: None,
            eps: None,
            eps_forward: None,
            beta: None,
            week_52_high: None,
            week_52_low: None,
            pb_ratio: None,
            ps_ratio: None,
            shares_outstanding: None,
            float_shares: None,
            revenue_ttm: None,
            profit_margin: None,
            operating_margin: None,
            return_on_equity: None,
            return_on_assets: None,
            debt_to_equity: None,
        };
        
        // Parse the Schwab API response structure: {"instruments": [{"fundamental": {...}}]}
        if let Some(instruments_array) = data.get("instruments") {
            if let Some(instruments) = instruments_array.as_array() {
                if let Some(first_instrument) = instruments.first() {
                    if let Some(fundamental) = first_instrument.get("fundamental") {
                        if let Some(fund_obj) = fundamental.as_object() {
                            debug!("DEBUG: Parsing fundamental data for {}", symbol);
                            debug!("DEBUG: Available fields: {:?}", fund_obj.keys().collect::<Vec<_>>());
                            
                            // Map Schwab API fields to our FundamentalData structure
                            // Core metrics
                            fundamental_data.pe_ratio = fund_obj.get("peRatio").and_then(|v| v.as_f64());
                            fundamental_data.market_cap = fund_obj.get("marketCap").and_then(|v| v.as_f64());
                            fundamental_data.dividend_yield = fund_obj.get("dividendYield").and_then(|v| v.as_f64());
                            fundamental_data.dividend_per_share = fund_obj.get("dividendAmount").and_then(|v| v.as_f64());
                            fundamental_data.eps = fund_obj.get("eps").and_then(|v| v.as_f64());
                            fundamental_data.beta = fund_obj.get("beta").and_then(|v| v.as_f64());
                            
                            // 52-week high/low
                            fundamental_data.week_52_high = fund_obj.get("high52").and_then(|v| v.as_f64());
                            fundamental_data.week_52_low = fund_obj.get("low52").and_then(|v| v.as_f64());
                            
                            // Ratios
                            fundamental_data.pb_ratio = fund_obj.get("pbRatio").and_then(|v| v.as_f64());
                            fundamental_data.ps_ratio = fund_obj.get("prRatio").and_then(|v| v.as_f64());
                            
                            // Shares
                            fundamental_data.shares_outstanding = fund_obj.get("sharesOutstanding").and_then(|v| v.as_f64());
                            
                            // Margins and returns
                            fundamental_data.profit_margin = fund_obj.get("netProfitMarginTTM").and_then(|v| v.as_f64());
                            fundamental_data.operating_margin = fund_obj.get("operatingMarginTTM").and_then(|v| v.as_f64());
                            fundamental_data.return_on_equity = fund_obj.get("returnOnEquity").and_then(|v| v.as_f64());
                            fundamental_data.return_on_assets = fund_obj.get("returnOnAssets").and_then(|v| v.as_f64());
                            
                            // Debt ratios
                            fundamental_data.debt_to_equity = fund_obj.get("totalDebtToEquity").and_then(|v| v.as_f64());
                            
                            debug!("DEBUG: Parsed fundamental data: P/E={:?}, Market Cap={:?}, Div Yield={:?}", 
                                   fundamental_data.pe_ratio, fundamental_data.market_cap, fundamental_data.dividend_yield);
                        }
                    }
                }
            }
        }
        
        Ok(fundamental_data)
    }

    /// Get instrument data by symbol
    #[allow(dead_code)]
    pub async fn get_instrument(&self, symbol: &str) -> Result<Value> {
        let url = format!("https://api.schwabapi.com/marketdata/v1/instruments?symbol={}&projection=symbol-search", symbol);
        self.make_request(&url).await
    }

    /// Get current market hours
    #[allow(dead_code)]
    pub async fn get_market_hours(&self, market: &str) -> Result<Value> {
        let url = format!("https://api.schwabapi.com/marketdata/v1/markets/{}", market);
        self.make_request(&url).await
    }
    
    /// Get market hours for a specific date
    #[allow(dead_code)]
    pub async fn get_market_hours_for_date(&self, market: &str, date: &str) -> Result<Value> {
        let url = format!("https://api.schwabapi.com/marketdata/v1/markets?markets={}&date={}", market, date);
        self.make_request(&url).await
    }
    
    /// Get enhanced quotes with additional fundamental fields
    pub async fn get_enhanced_quotes(&self, symbols: &[String]) -> Result<Vec<SchwabQuote>> {
        if symbols.is_empty() {
            return Ok(Vec::new());
        }

        let symbols_str = symbols.join(",");
        let url = format!(
            "https://api.schwabapi.com/marketdata/v1/quotes?symbols={}&fields=quote,fundamental", 
            symbols_str
        );
        
        let data = self.make_request(&url).await?;
        let mut quotes = Vec::new();

        if let Some(quotes_obj) = data.as_object() {
            for (symbol, quote_data) in quotes_obj {
                if let Some(quote_obj) = quote_data.as_object() {
                    // Get basic quote data
                    let mut quote = SchwabQuote {
                        symbol: symbol.clone(),
                        last_price: quote_obj.get("lastPrice")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0),
                        open_price: quote_obj.get("openPrice")
                            .and_then(|v| v.as_f64()),
                        high_price: quote_obj.get("highPrice")
                            .and_then(|v| v.as_f64()),
                        low_price: quote_obj.get("lowPrice")
                            .and_then(|v| v.as_f64()),
                        close_price: quote_obj.get("closePrice")
                            .and_then(|v| v.as_f64()),
                        volume: quote_obj.get("totalVolume")
                            .and_then(|v| v.as_i64()),
                        pe_ratio: quote_obj.get("peRatio")
                            .and_then(|v| v.as_f64()),
                        market_cap: quote_obj.get("marketCap")
                            .and_then(|v| v.as_f64()),
                        dividend_yield: quote_obj.get("divYield")
                            .and_then(|v| v.as_f64()),
                    };
                    
                    // Try to get additional fundamental data if available
                    if let Some(fundamental) = quote_obj.get("fundamental") {
                        if let Some(fund_obj) = fundamental.as_object() {
                            // Override with more detailed fundamental data if available
                            if let Some(pe) = fund_obj.get("peRatio").and_then(|v| v.as_f64()) {
                                quote.pe_ratio = Some(pe);
                            }
                            if let Some(mc) = fund_obj.get("marketCap").and_then(|v| v.as_f64()) {
                                quote.market_cap = Some(mc);
                            }
                            if let Some(div_yield) = fund_obj.get("divYield").and_then(|v| v.as_f64()) {
                                quote.dividend_yield = Some(div_yield);
                            }
                        }
                    }
                    
                    quotes.push(quote);
                }
            }
        }

        debug!("Retrieved {} enhanced quotes for {} symbols", quotes.len(), symbols.len());
        Ok(quotes)
    }
}

#[async_trait::async_trait]
impl StockDataProvider for SchwabClient {

    /// Get quotes for multiple symbols
    async fn get_quotes(&self, symbols: &[String]) -> Result<Vec<SchwabQuote>> {
        if symbols.is_empty() {
            return Ok(Vec::new());
        }

        let symbols_str = symbols.join(",");
        let url = format!("https://api.schwabapi.com/marketdata/v1/quotes?symbols={}", symbols_str);
        
        let data = self.make_request(&url).await?;
        let mut quotes = Vec::new();

        if let Some(quotes_obj) = data.as_object() {
            for (symbol, quote_data) in quotes_obj {
                if let Some(quote_obj) = quote_data.as_object() {
                    // Parse the quote data
                    let quote = SchwabQuote {
                        symbol: symbol.clone(),
                        last_price: quote_obj.get("lastPrice")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0),
                        open_price: quote_obj.get("openPrice")
                            .and_then(|v| v.as_f64()),
                        high_price: quote_obj.get("highPrice")
                            .and_then(|v| v.as_f64()),
                        low_price: quote_obj.get("lowPrice")
                            .and_then(|v| v.as_f64()),
                        close_price: quote_obj.get("closePrice")
                            .and_then(|v| v.as_f64()),
                        volume: quote_obj.get("totalVolume")
                            .and_then(|v| v.as_i64()),
                        pe_ratio: quote_obj.get("peRatio")
                            .and_then(|v| v.as_f64()),
                        market_cap: quote_obj.get("marketCap")
                            .and_then(|v| v.as_f64()),
                        dividend_yield: quote_obj.get("divYield")
                            .and_then(|v| v.as_f64()),
                    };
                    quotes.push(quote);
                }
            }
        }

        debug!("Retrieved {} quotes for {} symbols", quotes.len(), symbols.len());
        Ok(quotes)
    }

    /// Get price history for a symbol
    async fn get_price_history(
        &self,
        symbol: &str,
        from_date: NaiveDate,
        to_date: NaiveDate,
    ) -> Result<Vec<SchwabPriceBar>> {
        // Convert dates to timestamps (milliseconds since epoch)
        let from_timestamp = from_date
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis();
        let to_timestamp = to_date
            .and_hms_opt(23, 59, 59)
            .unwrap()
            .and_utc()
            .timestamp_millis();

        let url = format!(
            "https://api.schwabapi.com/marketdata/v1/pricehistory?symbol={}&periodType=year&frequencyType=daily&frequency=1&startDate={}&endDate={}",
            symbol, from_timestamp, to_timestamp
        );

        let data = self.make_request(&url).await?;
        let mut price_bars = Vec::new();

        if let Some(candles) = data.get("candles").and_then(|v| v.as_array()) {
            for candle in candles {
                if let Some(candle_obj) = candle.as_object() {
                    let price_bar = SchwabPriceBar {
                        datetime: candle_obj.get("datetime")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0),
                        open: candle_obj.get("open")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0),
                        high: candle_obj.get("high")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0),
                        low: candle_obj.get("low")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0),
                        close: candle_obj.get("close")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0),
                        volume: candle_obj.get("volume")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0),
                    };
                    price_bars.push(price_bar);
                }
            }
        }

        debug!("Retrieved {} price bars for {} from {} to {}", 
               price_bars.len(), symbol, from_date, to_date);
        Ok(price_bars)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stored_tokens_serialization() {
        let tokens = StoredTokens {
            access_token: "test_access".to_string(),
            refresh_token: "test_refresh".to_string(),
            expires_at: Utc::now(),
        };

        let json = serde_json::to_string(&tokens).unwrap();
        let deserialized: StoredTokens = serde_json::from_str(&json).unwrap();

        assert_eq!(tokens.access_token, deserialized.access_token);
        assert_eq!(tokens.refresh_token, deserialized.refresh_token);
    }
}