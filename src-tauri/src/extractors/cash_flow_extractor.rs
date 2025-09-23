use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite, Row};
use std::collections::HashMap;
use tokio::time::{sleep, Duration};
use std::sync::Arc;
use tokio::sync::Semaphore;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashFlowData {
    pub stock_id: i32,
    pub period_type: String,
    pub report_date: chrono::NaiveDate,
    pub fiscal_year: i32,
    pub fiscal_period: Option<String>,
    
    // Core Cash Flow Metrics (Critical for Piotroski)
    pub operating_cash_flow: Option<f64>,
    pub investing_cash_flow: Option<f64>,
    pub financing_cash_flow: Option<f64>,
    pub net_cash_flow: Option<f64>,
    
    // Additional Metrics
    pub depreciation_expense: Option<f64>,
    pub amortization_expense: Option<f64>,
    pub share_repurchases: Option<f64>,
    pub dividends_paid: Option<f64>,
    
    // Metadata
    pub currency: String,
    pub data_source: String,
}

#[derive(Debug, Deserialize)]
struct EdgarCompanyFacts {
    #[allow(dead_code)]
    cik: i64,  // CIK is actually an integer in the JSON
    #[serde(rename = "entityName")]
    #[allow(dead_code)]
    entity_name: String,
    facts: EdgarFacts,
}

#[derive(Debug, Deserialize)]
struct EdgarFacts {
    #[serde(rename = "us-gaap")]
    us_gaap: HashMap<String, EdgarFact>,
}

#[derive(Debug, Deserialize)]
struct EdgarFact {
    #[allow(dead_code)]
    label: Option<String>,
    #[allow(dead_code)]
    description: Option<String>,
    units: HashMap<String, Vec<EdgarFactValue>>,
}

#[derive(Debug, Deserialize)]
struct EdgarFactValue {
    #[serde(rename = "start")]
    #[allow(dead_code)]
    start_date: Option<String>,
    #[serde(rename = "end")]
    #[allow(dead_code)]
    end_date: Option<String>,
    val: Option<f64>,
    #[allow(dead_code)]
    accn: Option<String>,
    fy: Option<i32>,
    fp: Option<String>,
    #[allow(dead_code)]
    form: Option<String>,
    #[allow(dead_code)]
    filed: Option<String>,
    #[allow(dead_code)]
    frame: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CashFlowExtractor {
    #[allow(dead_code)]
    client: Client,
    db_pool: Pool<Sqlite>,
    rate_limiter: Arc<Semaphore>,
    #[allow(dead_code)]
    cik_cache: Arc<tokio::sync::Mutex<HashMap<String, String>>>,
}

impl CashFlowExtractor {
    pub fn new(db_pool: Pool<Sqlite>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (compatible; RustStocks/1.0)")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            db_pool,
            rate_limiter: Arc::new(Semaphore::new(10)), // Limit to 10 concurrent requests
            cik_cache: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
    }

    /// Extract cash flow data for all S&P 500 stocks
    pub async fn extract_all_sp500_cash_flow(&self) -> Result<ExtractionReport, String> {
        println!("🔄 Starting cash flow extraction for S&P 500 stocks...");
        
        // Get S&P 500 symbols (limit to 10 for testing)
        let symbols = self.get_sp500_symbols().await?;
        println!("📊 Found {} S&P 500 symbols to process", symbols.len());

        let mut successful_extractions = 0;
        let mut failed_extractions = 0;
        let mut total_records = 0;

        // Process symbols sequentially to avoid concurrency issues
        for symbol in &symbols {
            match self.extract_cash_flow_data(symbol).await {
                Ok(data) => {
                    if !data.is_empty() {
                        successful_extractions += 1;
                        total_records += data.len();
                        println!("✅ {}: {} cash flow records extracted", 
                               data[0].stock_id, data.len());
                    }
                }
                Err(e) => {
                    failed_extractions += 1;
                    println!("❌ Cash flow extraction failed for {}: {}", symbol, e);
                }
            }

            // Rate limiting between requests
            sleep(Duration::from_millis(100)).await;
        }

        println!("🎉 Cash flow extraction completed!");
        println!("   ✅ Successful: {}", successful_extractions);
        println!("   ❌ Failed: {}", failed_extractions);
        println!("   📊 Total records: {}", total_records);

        Ok(ExtractionReport {
            total_symbols: symbols.len(),
            successful_extractions,
            failed_extractions,
            total_records,
        })
    }

    /// Extract cash flow data for a single symbol from EDGAR
    pub async fn extract_cash_flow_data(&self, symbol: &str) -> Result<Vec<CashFlowData>, String> {
        let _permit = self.rate_limiter.acquire().await
            .map_err(|e| format!("Rate limiter error: {}", e))?;

        // Get stock_id from symbol
        let stock_id = self.get_stock_id_from_symbol(symbol).await?;
        
        // Get CIK from symbol
        let cik = self.get_cik_from_symbol(symbol).await?;
        
        // Load EDGAR companyfacts JSON file
        let edgar_data = self.load_edgar_data(&cik).await?;
        
        // Extract cash flow data from JSON
        let cash_flow_records = self.parse_cash_flow_from_json(&edgar_data, stock_id)?;
        
        println!("✅ Extracted {} cash flow records for {}", cash_flow_records.len(), symbol);
        
        Ok(cash_flow_records)
    }

    async fn load_edgar_data(&self, cik: &str) -> Result<EdgarCompanyFacts, String> {
        // Format CIK as zero-padded 10-digit string
        let padded_cik = format!("{:0>10}", cik);
        let file_path = format!("../edgar_data/companyfacts/CIK{}.json", padded_cik);
        
        if !Path::new(&file_path).exists() {
            return Err(format!("EDGAR data file not found: {}", file_path));
        }
        
        let content = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read EDGAR file {}: {}", file_path, e))?;
        
        let edgar_data: EdgarCompanyFacts = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse EDGAR JSON for CIK {}: {}", cik, e))?;
        
        Ok(edgar_data)
    }

    fn parse_cash_flow_from_json(&self, edgar_data: &EdgarCompanyFacts, stock_id: i32) -> Result<Vec<CashFlowData>, String> {
        let mut cash_flow_records = Vec::new();
        
        // Key cash flow fields to extract
        let cash_flow_fields = [
            "NetCashProvidedByUsedInOperatingActivities",
            "NetCashProvidedByUsedInInvestingActivities", 
            "NetCashProvidedByUsedInFinancingActivities",
            "CashAndCashEquivalentsPeriodIncreaseDecrease",
            "DepreciationDepletionAndAmortization",
            "PaymentsOfDividends",
            "PaymentsToAcquireBusinessesNetOfCashAcquired",
            "PaymentsToAcquirePropertyPlantAndEquipment",
        ];
        
        // Group data by fiscal period
        let mut period_data: HashMap<String, HashMap<String, f64>> = HashMap::new();
        
        for field_name in &cash_flow_fields {
            if let Some(fact) = edgar_data.facts.us_gaap.get(*field_name) {
                if let Some(usd_values) = fact.units.get("USD") {
                    for value in usd_values {
                        if let (Some(fy), Some(fp), Some(val)) = (value.fy, &value.fp, value.val) {
                            let period_key = format!("{}-{}", fy, fp);
                            period_data.entry(period_key)
                                .or_insert_with(HashMap::new)
                                .insert(field_name.to_string(), val);
                        }
                    }
                }
            }
        }
        
        // Convert grouped data to CashFlowData records
        for (period_key, data) in period_data {
            let parts: Vec<&str> = period_key.split('-').collect();
            if parts.len() != 2 {
                continue;
            }
            
            let fiscal_year = parts[0].parse::<i32>().unwrap_or(0);
            let fiscal_period = parts[1].to_string();
            
            // Determine period type
            let period_type = match fiscal_period.as_str() {
                "FY" => "Annual".to_string(),
                "Q1" | "Q2" | "Q3" | "Q4" => "Quarterly".to_string(),
                _ => "Other".to_string(),
            };
            
            // Parse report date (use end of period)
            let report_date = self.parse_report_date(&fiscal_period, fiscal_year);
            
            let record = CashFlowData {
                stock_id,
                period_type,
                report_date,
                fiscal_year,
                fiscal_period: Some(fiscal_period),
                operating_cash_flow: data.get("NetCashProvidedByUsedInOperatingActivities").copied(),
                investing_cash_flow: data.get("NetCashProvidedByUsedInInvestingActivities").copied(),
                financing_cash_flow: data.get("NetCashProvidedByUsedInFinancingActivities").copied(),
                net_cash_flow: data.get("CashAndCashEquivalentsPeriodIncreaseDecrease").copied(),
                depreciation_expense: data.get("DepreciationDepletionAndAmortization").copied(),
                amortization_expense: None, // Not separately available in basic fields
                dividends_paid: data.get("PaymentsOfDividends").map(|v| -*v), // Payments are negative
                share_repurchases: None, // Not directly available in basic fields
                currency: "USD".to_string(),
                data_source: "EDGAR".to_string(),
            };
            
            cash_flow_records.push(record);
        }
        
        // Sort by fiscal year and period
        cash_flow_records.sort_by(|a, b| {
            a.fiscal_year.cmp(&b.fiscal_year)
                .then_with(|| a.fiscal_period.cmp(&b.fiscal_period))
        });
        
        Ok(cash_flow_records)
    }

    fn parse_report_date(&self, fiscal_period: &str, fiscal_year: i32) -> chrono::NaiveDate {
        match fiscal_period {
            "FY" => chrono::NaiveDate::from_ymd_opt(fiscal_year, 12, 31).unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(fiscal_year, 1, 1).unwrap()),
            "Q1" => chrono::NaiveDate::from_ymd_opt(fiscal_year, 3, 31).unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(fiscal_year, 1, 1).unwrap()),
            "Q2" => chrono::NaiveDate::from_ymd_opt(fiscal_year, 6, 30).unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(fiscal_year, 1, 1).unwrap()),
            "Q3" => chrono::NaiveDate::from_ymd_opt(fiscal_year, 9, 30).unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(fiscal_year, 1, 1).unwrap()),
            "Q4" => chrono::NaiveDate::from_ymd_opt(fiscal_year, 12, 31).unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(fiscal_year, 1, 1).unwrap()),
            _ => chrono::NaiveDate::from_ymd_opt(fiscal_year, 12, 31).unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(fiscal_year, 1, 1).unwrap()),
        }
    }

    async fn get_cik_from_symbol(&self, symbol: &str) -> Result<String, String> {
        let row = sqlx::query(
            "SELECT cik FROM cik_mappings_sp500 WHERE symbol = ?"
        )
        .bind(symbol)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

        match row {
            Some(row) => {
                let cik: String = row.try_get("cik")
                    .map_err(|e| format!("Failed to get CIK: {}", e))?;
                Ok(cik)
            }
            None => Err(format!("CIK not found for symbol: {}", symbol))
        }
    }

    async fn get_stock_id_from_symbol(&self, symbol: &str) -> Result<i32, String> {
        let row = sqlx::query(
            "SELECT id FROM stocks WHERE symbol = ?"
        )
        .bind(symbol)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

        match row {
            Some(row) => {
                let id: i32 = row.try_get("id").map_err(|e| format!("Database error: {}", e))?;
                Ok(id)
            }
            None => Err(format!("Stock not found: {}", symbol))
        }
    }

    #[allow(dead_code)]
    async fn save_cash_flow_data(&self, data: &[CashFlowData]) -> Result<(), String> {
        for record in data {
            sqlx::query(
                r#"
                INSERT OR REPLACE INTO cash_flow_statements 
                (stock_id, period_type, report_date, fiscal_year, fiscal_period,
                 operating_cash_flow, depreciation_amortization, depreciation_expense, amortization_expense,
                 investing_cash_flow, capital_expenditures, financing_cash_flow, dividends_paid, 
                 share_repurchases, net_cash_flow, edgar_accession, edgar_form, edgar_filed_date, data_source)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#
            )
            .bind(record.stock_id)
            .bind(&record.period_type)
            .bind(record.report_date)
            .bind(record.fiscal_year)
            .bind(&record.fiscal_period)
            .bind(record.operating_cash_flow)
            .bind(None::<f64>) // depreciation_amortization
            .bind(record.depreciation_expense)
            .bind(record.amortization_expense)
            .bind(record.investing_cash_flow)
            .bind(None::<f64>) // capital_expenditures
            .bind(record.financing_cash_flow)
            .bind(record.dividends_paid)
            .bind(record.share_repurchases)
            .bind(record.net_cash_flow)
            .bind(None::<String>) // edgar_accession
            .bind(None::<String>) // edgar_form
            .bind(None::<String>) // edgar_filed_date
            .bind(&record.data_source)
            .execute(&self.db_pool)
            .await
            .map_err(|e| format!("Failed to save cash flow data: {}", e))?;
        }
        Ok(())
    }

    async fn get_sp500_symbols(&self) -> Result<Vec<String>, String> {
        let rows = sqlx::query(
            "SELECT symbol FROM cik_mappings_sp500 WHERE symbol IS NOT NULL ORDER BY symbol LIMIT 10"
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

        let mut symbols = Vec::new();
        for row in rows {
            let symbol: String = row.try_get("symbol").map_err(|e| format!("Database error: {}", e))?;
            symbols.push(symbol);
        }
        Ok(symbols)
    }
}

#[derive(Debug, Clone)]
pub struct ExtractionReport {
    pub total_symbols: usize,
    pub successful_extractions: usize,
    pub failed_extractions: usize,
    pub total_records: usize,
}