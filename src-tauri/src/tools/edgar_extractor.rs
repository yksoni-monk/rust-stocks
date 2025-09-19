use anyhow::{Result, anyhow};
use serde_json::Value;
use sqlx::SqlitePool;
use chrono::{NaiveDate, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct EdgarFinancialData {
    pub cik: i32,
    pub entity_name: String,
    pub cash_flow_data: Vec<CashFlowStatement>,
    pub balance_sheet_enhancements: Vec<BalanceSheetEnhancement>,
    pub income_statement_enhancements: Vec<IncomeStatementEnhancement>,
    pub dividend_data: Vec<DividendRecord>,
}

#[derive(Debug, Clone)]
pub struct CashFlowStatement {
    pub period_type: String,
    pub report_date: NaiveDate,
    pub fiscal_year: i32,
    pub fiscal_period: String,
    pub operating_cash_flow: Option<f64>,
    pub investing_cash_flow: Option<f64>,
    pub financing_cash_flow: Option<f64>,
    pub depreciation_amortization: Option<f64>,
    pub capital_expenditures: Option<f64>,
    pub dividends_paid: Option<f64>,
    pub share_repurchases: Option<f64>,
    pub edgar_accession: String,
    pub edgar_form: String,
}

#[derive(Debug, Clone)]
pub struct BalanceSheetEnhancement {
    pub report_date: NaiveDate,
    pub current_assets: Option<f64>,
    pub current_liabilities: Option<f64>,
    pub inventory: Option<f64>,
    pub accounts_receivable: Option<f64>,
    pub accounts_payable: Option<f64>,
    pub edgar_accession: String,
    pub edgar_form: String,
}

#[derive(Debug, Clone)]
pub struct IncomeStatementEnhancement {
    pub report_date: NaiveDate,
    pub cost_of_revenue: Option<f64>,
    pub research_development: Option<f64>,
    pub selling_general_admin: Option<f64>,
    pub depreciation_expense: Option<f64>,
    pub amortization_expense: Option<f64>,
    pub interest_expense: Option<f64>,
    pub edgar_accession: String,
    pub edgar_form: String,
}

#[derive(Debug, Clone)]
pub struct DividendRecord {
    pub ex_date: NaiveDate,
    pub dividend_per_share: f64,
    pub dividend_type: String,
    pub fiscal_year: i32,
    pub edgar_accession: String,
}

pub struct EdgarDataExtractor {
    edgar_data_path: String,
    pool: SqlitePool,
}

impl EdgarDataExtractor {
    pub fn new(edgar_data_path: &str, pool: SqlitePool) -> Self {
        Self {
            edgar_data_path: edgar_data_path.to_string(),
            pool,
        }
    }

    pub async fn extract_company_data(&self, cik: i32) -> Result<EdgarFinancialData> {
        let file_path = format!("{}/companyfacts/CIK{:010}.json", self.edgar_data_path, cik);

        println!("🔍 Extracting EDGAR data for CIK: {}", cik);

        let file_content = tokio::fs::read_to_string(&file_path).await
            .map_err(|e| anyhow!("Failed to read EDGAR file {}: {}", file_path, e))?;

        let json: Value = serde_json::from_str(&file_content)
            .map_err(|e| anyhow!("Failed to parse EDGAR JSON for CIK {}: {}", cik, e))?;

        let entity_name = self.extract_entity_name(&json)?;

        Ok(EdgarFinancialData {
            cik,
            entity_name,
            cash_flow_data: self.extract_cash_flow_data(&json)?,
            balance_sheet_enhancements: self.extract_balance_sheet_enhancements(&json)?,
            income_statement_enhancements: self.extract_income_statement_enhancements(&json)?,
            dividend_data: self.extract_dividend_data(&json)?,
        })
    }

    fn extract_entity_name(&self, json: &Value) -> Result<String> {
        json["entityName"].as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("Missing entityName in EDGAR data"))
    }

    fn extract_cash_flow_data(&self, json: &Value) -> Result<Vec<CashFlowStatement>> {
        let mut cash_flows = Vec::new();
        let facts = &json["facts"]["us-gaap"];

        if !facts.is_object() {
            return Ok(cash_flows);
        }

        // Extract operating cash flow
        if let Some(operating_cf) = facts.get("NetCashProvidedByUsedInOperatingActivities") {
            if let Some(usd_data) = operating_cf["units"]["USD"].as_array() {
                for entry in usd_data {
                    if let Ok(cash_flow) = self.parse_cash_flow_entry(entry, "operating") {
                        cash_flows.push(cash_flow);
                    }
                }
            }
        }

        // Cross-reference with other cash flow items
        self.enhance_cash_flow_data(&mut cash_flows, facts)?;

        // Filter to TTM and recent annual data only
        cash_flows.retain(|cf| {
            matches!(cf.period_type.as_str(), "TTM" | "Annual") && cf.fiscal_year >= 2020
        });

        Ok(cash_flows)
    }

    fn parse_cash_flow_entry(&self, entry: &Value, cf_type: &str) -> Result<CashFlowStatement> {
        let end_date = entry["end"].as_str()
            .ok_or_else(|| anyhow!("Missing end date"))?;
        let report_date = NaiveDate::parse_from_str(end_date, "%Y-%m-%d")?;

        let fiscal_year = entry["fy"].as_i64()
            .ok_or_else(|| anyhow!("Missing fiscal year"))? as i32;

        let period_type = self.determine_period_type(entry)?;
        let value = entry["val"].as_f64();

        Ok(CashFlowStatement {
            period_type,
            report_date,
            fiscal_year,
            fiscal_period: entry["fp"].as_str().unwrap_or("").to_string(),
            operating_cash_flow: if cf_type == "operating" { value } else { None },
            investing_cash_flow: None,
            financing_cash_flow: None,
            depreciation_amortization: None,
            capital_expenditures: None,
            dividends_paid: None,
            share_repurchases: None,
            edgar_accession: entry["accn"].as_str().unwrap_or("").to_string(),
            edgar_form: entry["form"].as_str().unwrap_or("").to_string(),
        })
    }

    fn determine_period_type(&self, entry: &Value) -> Result<String> {
        if let Some(form) = entry["form"].as_str() {
            match form {
                "10-K" => Ok("Annual".to_string()),
                "10-Q" => Ok("Quarterly".to_string()),
                _ => {
                    // Check date ranges to determine if TTM
                    if let (Some(start), Some(end)) = (entry["start"].as_str(), entry["end"].as_str()) {
                        let start_date = NaiveDate::parse_from_str(start, "%Y-%m-%d")?;
                        let end_date = NaiveDate::parse_from_str(end, "%Y-%m-%d")?;
                        let duration = end_date.signed_duration_since(start_date);

                        if duration.num_days() >= 350 && duration.num_days() <= 380 {
                            Ok("TTM".to_string())
                        } else if duration.num_days() >= 85 && duration.num_days() <= 95 {
                            Ok("Quarterly".to_string())
                        } else {
                            Ok("Annual".to_string())
                        }
                    } else {
                        Ok("Unknown".to_string())
                    }
                }
            }
        } else {
            Ok("Unknown".to_string())
        }
    }

    fn enhance_cash_flow_data(&self, cash_flows: &mut Vec<CashFlowStatement>, facts: &Value) -> Result<()> {
        // Create a lookup map for existing cash flows
        let mut cf_map: HashMap<String, usize> = HashMap::new();
        for (i, cf) in cash_flows.iter().enumerate() {
            let key = format!("{}-{}", cf.report_date, cf.period_type);
            cf_map.insert(key, i);
        }

        // Add investing cash flow
        if let Some(investing_cf) = facts.get("NetCashProvidedByUsedInInvestingActivities") {
            if let Some(usd_data) = investing_cf["units"]["USD"].as_array() {
                for entry in usd_data {
                    if let Ok(report_date) = entry["end"].as_str()
                        .ok_or_else(|| anyhow!("Missing end date"))
                        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e| anyhow!("{}", e))) {

                        if let Ok(period_type) = self.determine_period_type(entry) {
                            let key = format!("{}-{}", report_date, period_type);
                            if let Some(&index) = cf_map.get(&key) {
                                cash_flows[index].investing_cash_flow = entry["val"].as_f64();
                            }
                        }
                    }
                }
            }
        }

        // Add financing cash flow
        if let Some(financing_cf) = facts.get("NetCashProvidedByUsedInFinancingActivities") {
            if let Some(usd_data) = financing_cf["units"]["USD"].as_array() {
                for entry in usd_data {
                    if let Ok(report_date) = entry["end"].as_str()
                        .ok_or_else(|| anyhow!("Missing end date"))
                        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e| anyhow!("{}", e))) {

                        if let Ok(period_type) = self.determine_period_type(entry) {
                            let key = format!("{}-{}", report_date, period_type);
                            if let Some(&index) = cf_map.get(&key) {
                                cash_flows[index].financing_cash_flow = entry["val"].as_f64();
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn extract_balance_sheet_enhancements(&self, json: &Value) -> Result<Vec<BalanceSheetEnhancement>> {
        let mut enhancements = Vec::new();
        let facts = &json["facts"]["us-gaap"];

        if !facts.is_object() {
            return Ok(enhancements);
        }

        // Create a map to collect data by date
        let mut data_map: HashMap<String, BalanceSheetEnhancement> = HashMap::new();

        // Extract current assets
        if let Some(current_assets) = facts.get("AssetsCurrent") {
            if let Some(usd_data) = current_assets["units"]["USD"].as_array() {
                for entry in usd_data {
                    if let Ok(date_key) = self.get_date_key(entry) {
                        let enhancement = data_map.entry(date_key.clone()).or_insert_with(|| {
                            self.create_empty_balance_sheet_enhancement(&date_key, entry)
                        });
                        enhancement.current_assets = entry["val"].as_f64();
                    }
                }
            }
        }

        // Extract current liabilities
        if let Some(current_liabilities) = facts.get("LiabilitiesCurrent") {
            if let Some(usd_data) = current_liabilities["units"]["USD"].as_array() {
                for entry in usd_data {
                    if let Ok(date_key) = self.get_date_key(entry) {
                        let enhancement = data_map.entry(date_key.clone()).or_insert_with(|| {
                            self.create_empty_balance_sheet_enhancement(&date_key, entry)
                        });
                        enhancement.current_liabilities = entry["val"].as_f64();
                    }
                }
            }
        }

        enhancements.extend(data_map.into_values());
        Ok(enhancements)
    }

    fn extract_income_statement_enhancements(&self, _json: &Value) -> Result<Vec<IncomeStatementEnhancement>> {
        // Simplified for now - can be expanded later
        Ok(Vec::new())
    }

    fn extract_dividend_data(&self, _json: &Value) -> Result<Vec<DividendRecord>> {
        // Simplified for now - can be expanded later
        Ok(Vec::new())
    }

    fn get_date_key(&self, entry: &Value) -> Result<String> {
        let end_date = entry["end"].as_str()
            .ok_or_else(|| anyhow!("Missing end date"))?;
        let period_type = self.determine_period_type(entry)?;
        Ok(format!("{}-{}", end_date, period_type))
    }

    fn create_empty_balance_sheet_enhancement(&self, date_key: &str, entry: &Value) -> BalanceSheetEnhancement {
        let parts: Vec<&str> = date_key.split('-').collect();
        let report_date = NaiveDate::parse_from_str(parts[0], "%Y-%m-%d").unwrap_or_default();

        BalanceSheetEnhancement {
            report_date,
            current_assets: None,
            current_liabilities: None,
            inventory: None,
            accounts_receivable: None,
            accounts_payable: None,
            edgar_accession: entry["accn"].as_str().unwrap_or("").to_string(),
            edgar_form: entry["form"].as_str().unwrap_or("").to_string(),
        }
    }

    pub async fn store_financial_data(&self, stock_id: i32, data: &EdgarFinancialData) -> Result<i32> {
        let mut records_inserted = 0;

        println!("💾 Storing EDGAR data for stock {} ({} records)", stock_id,
                data.cash_flow_data.len() + data.balance_sheet_enhancements.len());

        // Store cash flow data
        for cf in &data.cash_flow_data {
            let result = sqlx::query(
                r#"
                INSERT OR REPLACE INTO cash_flow_statements
                (stock_id, period_type, report_date, fiscal_year, fiscal_period,
                 operating_cash_flow, investing_cash_flow, financing_cash_flow,
                 depreciation_amortization, capital_expenditures, dividends_paid,
                 share_repurchases, edgar_accession, edgar_form, data_source)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'edgar')
                "#
            )
            .bind(stock_id)
            .bind(&cf.period_type)
            .bind(cf.report_date)
            .bind(cf.fiscal_year)
            .bind(&cf.fiscal_period)
            .bind(cf.operating_cash_flow)
            .bind(cf.investing_cash_flow)
            .bind(cf.financing_cash_flow)
            .bind(cf.depreciation_amortization)
            .bind(cf.capital_expenditures)
            .bind(cf.dividends_paid)
            .bind(cf.share_repurchases)
            .bind(&cf.edgar_accession)
            .bind(&cf.edgar_form)
            .execute(&self.pool)
            .await;

            if result.is_ok() {
                records_inserted += 1;
            }
        }

        // Store balance sheet enhancements
        for bs in &data.balance_sheet_enhancements {
            let result = sqlx::query(
                r#"
                UPDATE balance_sheets
                SET current_assets = ?, current_liabilities = ?, inventory = ?,
                    accounts_receivable = ?, accounts_payable = ?,
                    working_capital = CASE
                        WHEN ? IS NOT NULL AND ? IS NOT NULL
                        THEN ? - ?
                        ELSE working_capital
                    END,
                    edgar_accession = ?, edgar_form = ?
                WHERE stock_id = ? AND report_date = ?
                "#
            )
            .bind(bs.current_assets)
            .bind(bs.current_liabilities)
            .bind(bs.inventory)
            .bind(bs.accounts_receivable)
            .bind(bs.accounts_payable)
            .bind(bs.current_assets)
            .bind(bs.current_liabilities)
            .bind(bs.current_assets)
            .bind(bs.current_liabilities)
            .bind(&bs.edgar_accession)
            .bind(&bs.edgar_form)
            .bind(stock_id)
            .bind(bs.report_date)
            .execute(&self.pool)
            .await;

            if result.is_ok() {
                records_inserted += 1;
            }
        }

        Ok(records_inserted)
    }
}