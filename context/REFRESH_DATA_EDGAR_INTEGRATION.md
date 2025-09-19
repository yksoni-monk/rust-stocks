# Refresh Data Tool - EDGAR Integration Plan

## 🚨 **CRITICAL UPDATE: EDGAR DATA EXTRACTION INTEGRATION**

With the discovery that all required financial data exists in your EDGAR company facts JSON files, the refresh_data tool must be enhanced to include EDGAR data extraction as a core component.

## 🔄 **Enhanced Refresh Data Architecture**

### **New Refresh Modes (Updated)**

| **Mode** | **Data Sources** | **Duration** | **Coverage** |
|----------|------------------|-------------|--------------|
| `prices` | Schwab prices + P/E ratios | ~40min | Basic screening |
| `ratios` | Prices + P/S ratios + **EDGAR cash flow** | ~60min | Enhanced screening |
| `everything` | All data + **Complete EDGAR extraction** | ~90min | Full algorithm accuracy |

## 🗄️ **Database Schema Extensions Required**

### **1. Cash Flow Statements Table**
```sql
CREATE TABLE cash_flow_statements (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    period_type TEXT NOT NULL, -- 'TTM', 'Annual', 'Quarterly'
    report_date DATE NOT NULL,
    fiscal_year INTEGER,
    fiscal_period TEXT,

    -- Core Cash Flow Data (from EDGAR)
    operating_cash_flow REAL, -- NetCashProvidedByUsedInOperatingActivities
    investing_cash_flow REAL, -- NetCashProvidedByUsedInInvestingActivities
    financing_cash_flow REAL, -- NetCashProvidedByUsedInFinancingActivities
    net_cash_flow REAL,       -- Total net change in cash

    -- EBITDA Components
    depreciation_amortization REAL, -- DepreciationDepletionAndAmortization
    depreciation_expense REAL,      -- DepreciationAndAmortization
    amortization_expense REAL,      -- AmortizationOfIntangibleAssets

    -- Additional Details
    capital_expenditures REAL,      -- PaymentsToAcquirePropertyPlantAndEquipment
    dividends_paid REAL,            -- PaymentsOfDividends
    share_repurchases REAL,         -- PaymentsForRepurchaseOfCommonStock

    -- EDGAR Metadata
    edgar_accession TEXT,
    edgar_form TEXT, -- '10-K', '10-Q'
    edgar_filed_date DATE,
    data_source TEXT DEFAULT 'edgar',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, period_type, report_date)
);
```

### **2. Enhanced Balance Sheets (Add Missing Fields)**
```sql
-- Add fields extracted from EDGAR AssetsCurrent, LiabilitiesCurrent, etc.
ALTER TABLE balance_sheets ADD COLUMN current_assets REAL;
ALTER TABLE balance_sheets ADD COLUMN current_liabilities REAL;
ALTER TABLE balance_sheets ADD COLUMN inventory REAL;
ALTER TABLE balance_sheets ADD COLUMN accounts_receivable REAL;
ALTER TABLE balance_sheets ADD COLUMN accounts_payable REAL;
ALTER TABLE balance_sheets ADD COLUMN working_capital REAL;

-- Add EDGAR metadata
ALTER TABLE balance_sheets ADD COLUMN edgar_accession TEXT;
ALTER TABLE balance_sheets ADD COLUMN edgar_form TEXT;
ALTER TABLE balance_sheets ADD COLUMN edgar_filed_date DATE;
```

### **3. Enhanced Income Statements (Add Missing Fields)**
```sql
-- Add fields for complete EBITDA calculation
ALTER TABLE income_statements ADD COLUMN cost_of_revenue REAL;
ALTER TABLE income_statements ADD COLUMN research_development REAL;
ALTER TABLE income_statements ADD COLUMN selling_general_admin REAL;
ALTER TABLE income_statements ADD COLUMN depreciation_expense REAL;
ALTER TABLE income_statements ADD COLUMN amortization_expense REAL;
ALTER TABLE income_statements ADD COLUMN interest_expense REAL;

-- Add EDGAR metadata
ALTER TABLE income_statements ADD COLUMN edgar_accession TEXT;
ALTER TABLE income_statements ADD COLUMN edgar_form TEXT;
ALTER TABLE income_statements ADD COLUMN edgar_filed_date DATE;
```

### **4. Dividend History Table**
```sql
CREATE TABLE dividend_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    ex_date DATE NOT NULL,
    payment_date DATE,
    record_date DATE,
    dividend_per_share REAL,
    dividend_type TEXT DEFAULT 'regular', -- 'regular', 'special', 'stock'

    -- Calculated fields
    annualized_dividend REAL,
    yield_at_ex_date REAL,

    -- EDGAR Metadata
    edgar_accession TEXT,
    fiscal_year INTEGER,
    fiscal_period TEXT,
    data_source TEXT DEFAULT 'edgar',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, ex_date)
);
```

## 🔧 **EDGAR Data Extraction Module**

### **Rust Implementation Structure**

#### **1. EDGAR JSON Parser (`src/tools/edgar_extractor.rs`)**
```rust
use serde_json::{Value, Map};
use anyhow::{Result, anyhow};
use std::collections::HashMap;

pub struct EdgarDataExtractor {
    company_facts_path: String,
}

#[derive(Debug, Clone)]
pub struct EdgarFinancialData {
    pub cik: i32,
    pub entity_name: String,
    pub cash_flow_data: Vec<CashFlowStatement>,
    pub balance_sheet_data: Vec<BalanceSheetData>,
    pub income_statement_data: Vec<IncomeStatementData>,
    pub dividend_data: Vec<DividendRecord>,
}

#[derive(Debug, Clone)]
pub struct CashFlowStatement {
    pub period_type: String,
    pub report_date: chrono::NaiveDate,
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

impl EdgarDataExtractor {
    pub fn new(edgar_data_path: &str) -> Self {
        Self {
            company_facts_path: format!("{}/companyfacts", edgar_data_path),
        }
    }

    pub async fn extract_company_data(&self, cik: i32) -> Result<EdgarFinancialData> {
        let file_path = format!("{}/CIK{:010}.json", self.company_facts_path, cik);
        let file_content = tokio::fs::read_to_string(&file_path).await?;
        let json: Value = serde_json::from_str(&file_content)?;

        Ok(EdgarFinancialData {
            cik,
            entity_name: self.extract_entity_name(&json)?,
            cash_flow_data: self.extract_cash_flow_data(&json)?,
            balance_sheet_data: self.extract_balance_sheet_data(&json)?,
            income_statement_data: self.extract_income_statement_data(&json)?,
            dividend_data: self.extract_dividend_data(&json)?,
        })
    }

    fn extract_cash_flow_data(&self, json: &Value) -> Result<Vec<CashFlowStatement>> {
        let mut cash_flows = Vec::new();
        let facts = json["facts"]["us-gaap"].as_object().unwrap();

        // Extract operating cash flow
        if let Some(operating_cf) = facts.get("NetCashProvidedByUsedInOperatingActivities") {
            if let Some(usd_data) = operating_cf["units"]["USD"].as_array() {
                for entry in usd_data {
                    cash_flows.push(CashFlowStatement {
                        period_type: self.determine_period_type(entry)?,
                        report_date: self.parse_date(entry["end"].as_str().unwrap())?,
                        fiscal_year: entry["fy"].as_i64().unwrap() as i32,
                        fiscal_period: entry["fp"].as_str().unwrap_or("").to_string(),
                        operating_cash_flow: entry["val"].as_f64(),
                        investing_cash_flow: None, // Will be filled by cross-referencing
                        financing_cash_flow: None, // Will be filled by cross-referencing
                        depreciation_amortization: None, // Will be filled by cross-referencing
                        capital_expenditures: None,
                        dividends_paid: None,
                        share_repurchases: None,
                        edgar_accession: entry["accn"].as_str().unwrap_or("").to_string(),
                        edgar_form: entry["form"].as_str().unwrap_or("").to_string(),
                    });
                }
            }
        }

        // Cross-reference and fill other cash flow items
        self.enhance_cash_flow_data(&mut cash_flows, facts)?;

        Ok(cash_flows)
    }

    fn extract_balance_sheet_data(&self, json: &Value) -> Result<Vec<BalanceSheetData>> {
        let mut balance_sheets = Vec::new();
        let facts = json["facts"]["us-gaap"].as_object().unwrap();

        // Extract current assets, current liabilities, etc.
        // Similar pattern to cash flow extraction

        Ok(balance_sheets)
    }

    fn determine_period_type(&self, entry: &Value) -> Result<String> {
        // Logic to determine if this is TTM, Annual, or Quarterly
        if let Some(form) = entry["form"].as_str() {
            match form {
                "10-K" => Ok("Annual".to_string()),
                "10-Q" => Ok("Quarterly".to_string()),
                _ => {
                    // Check date ranges to determine if TTM
                    if let (Some(start), Some(end)) = (entry["start"].as_str(), entry["end"].as_str()) {
                        let start_date = chrono::NaiveDate::parse_from_str(start, "%Y-%m-%d")?;
                        let end_date = chrono::NaiveDate::parse_from_str(end, "%Y-%m-%d")?;
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
}
```

#### **2. EDGAR Integration in Refresh Orchestrator**
```rust
// Add to existing DataRefreshOrchestrator
impl DataRefreshOrchestrator {
    pub async fn refresh_edgar_data(&self, stocks: &[i32]) -> Result<RefreshStepResult> {
        let edgar_extractor = EdgarDataExtractor::new("edgar_data");
        let mut records_processed = 0;
        let mut errors = Vec::new();

        for stock_id in stocks {
            // Get CIK for this stock
            if let Some(cik) = self.get_cik_for_stock(*stock_id).await? {
                match edgar_extractor.extract_company_data(cik).await {
                    Ok(financial_data) => {
                        // Insert cash flow data
                        self.insert_cash_flow_data(*stock_id, &financial_data.cash_flow_data).await?;

                        // Update balance sheet data
                        self.update_balance_sheet_data(*stock_id, &financial_data.balance_sheet_data).await?;

                        // Update income statement data
                        self.update_income_statement_data(*stock_id, &financial_data.income_statement_data).await?;

                        // Insert dividend data
                        self.insert_dividend_data(*stock_id, &financial_data.dividend_data).await?;

                        records_processed += 1;
                    }
                    Err(e) => {
                        errors.push(format!("Failed to extract EDGAR data for stock {}: {}", stock_id, e));
                    }
                }
            }
        }

        Ok(RefreshStepResult {
            step_name: "edgar_extraction".to_string(),
            records_processed,
            success: errors.is_empty(),
            error_message: if errors.is_empty() { None } else { Some(errors.join("; ")) },
            duration_seconds: 0, // Will be calculated by caller
        })
    }

    async fn get_cik_for_stock(&self, stock_id: i32) -> Result<Option<i32>> {
        let query = "SELECT cik FROM cik_mappings_sp500 cms JOIN stocks s ON cms.symbol = s.symbol WHERE s.id = ?";
        let row = sqlx::query(query)
            .bind(stock_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| r.get::<i32, _>("cik")))
    }
}
```

#### **3. Enhanced Refresh Data CLI Commands**
```rust
// Update existing refresh modes
impl From<CliRefreshMode> for RefreshMode {
    fn from(cli_mode: CliRefreshMode) -> Self {
        match cli_mode {
            CliRefreshMode::Prices => RefreshMode::Quick,
            CliRefreshMode::Ratios => RefreshMode::StandardWithEdgar, // New mode
            CliRefreshMode::Everything => RefreshMode::FullWithEdgar, // Enhanced mode
        }
    }
}

#[derive(Clone, ValueEnum, Debug)]
enum CliRefreshMode {
    /// Update stock prices + P/E ratios (~40min, unblocks GARP screening)
    Prices,
    /// Prices + P/S ratios + EDGAR cash flow data (~60min, unblocks full screening)
    Ratios,
    /// Complete data refresh including all EDGAR financials (~90min, full accuracy)
    Everything,
}
```

## ⚡ **Implementation Timeline (Revised)**

### **Phase 1: EDGAR Foundation (Week 1)**
1. **Day 1-2**: Database schema migrations for new tables
2. **Day 3-4**: EDGAR JSON parser implementation
3. **Day 5**: Basic extraction testing with sample S&P 500 companies

### **Phase 2: Integration (Week 2)**
1. **Day 1-2**: Integrate EDGAR extraction into refresh_data tool
2. **Day 3-4**: Update refresh modes and CLI
3. **Day 5**: End-to-end testing of complete data pipeline

### **Phase 3: Algorithm Implementation (Week 3)**
1. **Day 1-2**: Complete O'Shaughnessy implementation with true cash flow data
2. **Day 3-4**: Complete Piotroski 9-criteria implementation
3. **Day 5**: Backend testing and validation

### **Phase 4: Frontend & Production (Week 4)**
1. **Day 1-2**: Frontend integration for both complete algorithms
2. **Day 3-4**: Performance optimization and indexing
3. **Day 5**: Production deployment and monitoring

## 📊 **Expected Results (Complete Implementation)**

### **Data Coverage**
- **O'Shaughnessy**: 95%+ S&P 500 with 100% algorithm accuracy
- **Piotroski**: 90%+ S&P 500 with complete 9-criteria scoring
- **Performance**: Sub-5-second screening for 500+ stocks

### **Business Impact**
- **Academic Accuracy**: Matches published research papers exactly
- **Competitive Advantage**: True cash flow analysis vs competitors using proxies
- **User Confidence**: Transparent data quality and completeness indicators

This integration transforms the refresh_data tool from a basic price/ratio updater into a comprehensive financial data extraction system that enables world-class stock screening algorithms.