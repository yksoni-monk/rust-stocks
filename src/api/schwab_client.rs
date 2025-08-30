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
use tracing::{info, warn, error, debug};
use url::Url;

use crate::models::{Config, SchwabQuote, SchwabPriceBar};
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
    creation_timestamp: i64,
    token: TokenData,
}

#[derive(Debug, Deserialize)]
struct TokenData {
    access_token: String,
    refresh_token: String,
    expires_at: i64,
    expires_in: i64,
    token_type: String,
    scope: String,
    id_token: String,
}

/// Stored token information (internal format)
#[derive(Debug, Clone, Deserialize, Serialize)]
struct StoredTokens {
    access_token: String,
    refresh_token: String,
    expires_at: DateTime<Utc>,
}

/// Schwab API client
pub struct SchwabClient {
    client: Client,
    api_key: String,
    app_secret: String,
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
        if !std::path::Path::new(&self.token_path).exists() {
            return Err(anyhow!("Token file does not exist: {}", self.token_path));
        }

        let content = fs::read_to_string(&self.token_path)?;
        
        // Try to parse the Python-generated token file format first
        let tokens = if let Ok(token_file) = serde_json::from_str::<TokenFile>(&content) {
            StoredTokens {
                access_token: token_file.token.access_token,
                refresh_token: token_file.token.refresh_token,
                expires_at: DateTime::from_timestamp(token_file.token.expires_at, 0)
                    .unwrap_or_else(|| Utc::now()),
            }
        } else {
            // Fallback to direct StoredTokens format
            serde_json::from_str::<StoredTokens>(&content)?
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
        // Try to load tokens if we don't have any yet
        {
            let tokens_guard = self.current_tokens.lock().await;
            if tokens_guard.is_none() {
                drop(tokens_guard);
                let _ = self.load_tokens().await; // Ignore errors for now
            }
        }

        let tokens_guard = self.current_tokens.lock().await;
        if let Some(tokens) = &*tokens_guard {
            if tokens.expires_at > Utc::now() + chrono::Duration::minutes(5) {
                // Token is still valid for at least 5 more minutes
                return Ok(tokens.access_token.clone());
            }

            // Try to refresh the token
            let refresh_token = tokens.refresh_token.clone();
            drop(tokens_guard); // Release the lock before async call
            
            match self.refresh_access_token(&refresh_token).await {
                Ok(new_tokens) => {
                    *self.current_tokens.lock().await = Some(new_tokens.clone());
                    self.save_tokens(&new_tokens)?;
                    return Ok(new_tokens.access_token);
                }
                Err(e) => {
                    warn!("Failed to refresh token: {}", e);
                }
            }
        }

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

    /// Get fundamental data for a symbol
    pub async fn get_fundamentals(&self, symbol: &str) -> Result<HashMap<String, Value>> {
        let url = format!("https://api.schwabapi.com/marketdata/v1/instruments?symbol={}&projection=fundamental", symbol);
        let data = self.make_request(&url).await?;
        
        // Parse the response and extract fundamental data
        let mut fundamentals = HashMap::new();
        
        if let Some(instruments) = data.as_object() {
            if let Some((_, instrument_data)) = instruments.iter().next() {
                if let Some(fundamental) = instrument_data.get("fundamental") {
                    fundamentals.insert("fundamental".to_string(), fundamental.clone());
                }
            }
        }
        
        Ok(fundamentals)
    }

    /// Get instrument data by symbol
    pub async fn get_instrument(&self, symbol: &str) -> Result<Value> {
        let url = format!("https://api.schwabapi.com/marketdata/v1/instruments?symbol={}&projection=symbol-search", symbol);
        self.make_request(&url).await
    }

    /// Get current market hours
    pub async fn get_market_hours(&self, market: &str) -> Result<Value> {
        let url = format!("https://api.schwabapi.com/marketdata/v1/markets/{}", market);
        self.make_request(&url).await
    }
    
    /// Get market hours for a specific date
    pub async fn get_market_hours_for_date(&self, market: &str, date: &str) -> Result<Value> {
        let url = format!("https://api.schwabapi.com/marketdata/v1/markets?markets={}&date={}", market, date);
        self.make_request(&url).await
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