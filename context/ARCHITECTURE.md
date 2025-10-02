# Stock Analysis System - Architecture Document

## Executive Summary
A high-performance desktop application for stock analysis using Tauri (Rust backend + React frontend) that imports and analyzes comprehensive stock data from SimFin CSV files. Features offline-first architecture with 5,000+ stocks, comprehensive fundamental data, daily price history, and expandable panels UI for efficient analysis.

## Current System Architecture

### Technology Stack
- **Frontend**: SolidJS with TypeScript, signal-based reactivity, expandable panels UI
- **Backend**: Rust with Tauri framework 
- **Database**: SQLite for local persistence
- **Data Source**: EDGAR API integration (current) + SimFin CSV import system (legacy)
- **Future API Integration**: Charles Schwab API (for real-time quotes and options)
- **Desktop Framework**: Tauri for cross-platform desktop application
- **UI Framework**: Web-based interface rendered in Tauri webview

## SEC Company Facts-Based Freshness Checker Architecture

### Overview
The current freshness checker uses time-based staleness (days since last update), which doesn't accurately reflect whether we have the latest SEC filings. The new architecture leverages the SEC Company Facts API (which we already use) to check actual filing dates and determine if our data is truly up-to-date.

### Current Problem
- Freshness based on days since last update
- Doesn't verify if we have the latest SEC filings
- Can miss new filings that were just released
- False staleness when data is actually current

### Proposed Solution
Use SEC Company Facts API to:
1. Check latest filing dates per CIK from existing API calls
2. Compare with our stored data using filing metadata
3. Mark as fresh only if we have the latest filings
4. Trigger refresh only when new filings are available

### Company Facts API Analysis

#### JSON Response Structure
The Company Facts API (`https://data.sec.gov/api/xbrl/companyfacts/CIK##########.json`) provides comprehensive filing metadata:

```json
{
  "cik": 1413447,
  "entityName": "Paycom Software, Inc.",
  "entityType": "Operating company",
  "sic": "7372",
  "sicDescription": "Services-Prepackaged Software",
  "tickers": ["PAYC"],
  "exchanges": ["NASDAQ"],
  "facts": {
    "us-gaap": {
      "Assets": {
        "label": "Assets",
        "description": "Sum of the carrying amounts...",
        "units": {
          "USD": [
            {
              "end": "2023-12-31",        // Report date
              "val": 1234567890,          // Financial value
              "accn": "0001413447-24-000012", // Accession number
              "fy": 2023,                 // Fiscal year
              "fp": "FY",                 // Fiscal period (FY, Q1, Q2, Q3, Q4)
              "form": "10-K",             // Form type
              "filed": "2024-02-15",      // Filing date (when SEC received it)
              "frame": "CY2023"           // Calendar period frame
            }
          ]
        }
      }
    }
  }
}
```

#### Key Filing Metadata Fields
- `filed`: The actual filing date (when SEC received the filing)
- `end`: The period end date (report date)
- `accn`: Accession number (unique filing identifier)
- `form`: Form type (10-K, 10-Q, 8-K, etc.)
- `fy`: Fiscal year
- `fp`: Fiscal period (FY, Q1, Q2, Q3, Q4)
- `frame`: Calendar period frame (CY2023, CY2023Q1, etc.)

### Implementation Architecture

#### Phase 1: Enhanced SecEdgarClient with Filing Metadata
```rust
// src-tauri/src/tools/sec_edgar_client.rs
impl SecEdgarClient {
    /// Check if financial data needs update based on latest SEC filings
    pub async fn check_if_update_needed(&self, cik: &str, stock_id: i64) -> Result<bool> {
        // Fetch company facts (we already do this)
        let facts = self.fetch_company_facts(cik).await?;
        
        // Get latest filing date from SEC
        let latest_sec_filing = facts.get_latest_filing_date();
        
        // Get our latest filing date from database
        let our_latest = self.get_our_latest_filing_date(stock_id).await?;
        
        // Return true if SEC has newer data
        Ok(latest_sec_filing > our_latest)
    }
    
    /// Extract only new data points since last update
    pub async fn extract_new_data_only(&self, cik: &str, stock_id: i64, since_date: &str) -> Result<()> {
        let facts = self.fetch_company_facts(cik).await?;
        let new_data = facts.filter_data_after(since_date);
        self.store_financial_data_with_metadata(stock_id, new_data).await?;
        Ok(())
    }
    
    /// Get latest filing date from our database
    async fn get_our_latest_filing_date(&self, stock_id: i64) -> Result<Option<String>> {
        let query = r#"
            SELECT MAX(filed_date) as latest_filed_date
            FROM income_statements 
            WHERE stock_id = ? AND filed_date IS NOT NULL
        "#;
        
        let result: Option<String> = sqlx::query_scalar(query)
            .bind(stock_id)
            .fetch_optional(&self.pool)
            .await?;
            
        Ok(result)
    }
}
```

#### Phase 2: Enhanced Database Schema with Filing Metadata
```sql
-- Add filing metadata columns to existing tables
ALTER TABLE income_statements ADD COLUMN accession_number TEXT;
ALTER TABLE income_statements ADD COLUMN form_type TEXT;
ALTER TABLE income_statements ADD COLUMN filed_date DATE;
ALTER TABLE income_statements ADD COLUMN fiscal_period TEXT; -- FY, Q1, Q2, Q3, Q4

ALTER TABLE balance_sheets ADD COLUMN accession_number TEXT;
ALTER TABLE balance_sheets ADD COLUMN form_type TEXT;
ALTER TABLE balance_sheets ADD COLUMN filed_date DATE;
ALTER TABLE balance_sheets ADD COLUMN fiscal_period TEXT;

ALTER TABLE cash_flow_statements ADD COLUMN accession_number TEXT;
ALTER TABLE cash_flow_statements ADD COLUMN form_type TEXT;
ALTER TABLE cash_flow_statements ADD COLUMN filed_date DATE;
ALTER TABLE cash_flow_statements ADD COLUMN fiscal_period TEXT;

-- Add indexes for efficient filing date lookups
CREATE INDEX IF NOT EXISTS idx_income_statements_filed_date ON income_statements(filed_date);
CREATE INDEX IF NOT EXISTS idx_balance_sheets_filed_date ON balance_sheets(filed_date);
CREATE INDEX IF NOT EXISTS idx_cash_flow_statements_filed_date ON cash_flow_statements(filed_date);

-- Add table to track latest SEC filings per stock
CREATE TABLE IF NOT EXISTS sec_filing_tracking (
    stock_id INTEGER PRIMARY KEY,
    cik TEXT NOT NULL,
    latest_filing_date TEXT,
    latest_10k_date TEXT,
    latest_10q_date TEXT,
    last_checked TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (stock_id) REFERENCES stocks (id)
);

CREATE INDEX IF NOT EXISTS idx_sec_filing_tracking_cik ON sec_filing_tracking(cik);
CREATE INDEX IF NOT EXISTS idx_sec_filing_tracking_last_checked ON sec_filing_tracking(last_checked);
```

#### Phase 3: Enhanced Freshness Checker
```rust
// src-tauri/src/tools/data_freshness_checker.rs
impl DataStatusReader {
    /// Check financial statements freshness based on SEC Company Facts API
    async fn check_financial_statements_by_company_facts(&self) -> Result<DataFreshnessStatus> {
        let edgar_client = SecEdgarClient::new(self.pool.clone());
        
        // Get S&P 500 stocks with CIKs
        let stocks_with_ciks = self.get_sp500_stocks_with_ciks().await?;
        
        let mut total_stocks = 0;
        let mut up_to_date_stocks = 0;
        let mut missing_latest_filings = 0;
        let mut latest_filing_date: Option<String> = None;
        
        for (stock_id, symbol, cik) in stocks_with_ciks {
            total_stocks += 1;
            
            // Check if this stock needs update using Company Facts API
            match edgar_client.check_if_update_needed(&cik, stock_id).await {
                Ok(needs_update) => {
                    if needs_update {
                        missing_latest_filings += 1;
                    } else {
                        up_to_date_stocks += 1;
                    }
                }
                Err(e) => {
                    println!("⚠️ Failed to check {} ({}): {}", symbol, cik, e);
                    missing_latest_filings += 1;
                }
            }
            
            // Get latest filing date for this stock
            if let Ok(Some(latest_date)) = edgar_client.get_our_latest_filing_date(stock_id).await {
                if latest_filing_date.is_none() || latest_date > latest_filing_date.as_ref().unwrap() {
                    latest_filing_date = Some(latest_date);
                }
            }
        }
        
        // Determine freshness status
        let status = if missing_latest_filings == 0 {
            FreshnessStatus::Current
        } else if missing_latest_filings < total_stocks / 2 {
            FreshnessStatus::Stale
        } else {
            FreshnessStatus::Stale
        };
        
        let message = format!(
            "Latest SEC filings: {}/{} stocks up-to-date, {} missing latest filings",
            up_to_date_stocks, total_stocks, missing_latest_filings
        );
        
        Ok(DataFreshnessStatus {
            data_source: "financial_statements".to_string(),
            status,
            latest_data_date: latest_filing_date,
            last_refresh: None,
            staleness_days: None, // Not based on days anymore
            records_count: total_stocks as i64,
            message,
            refresh_priority: if missing_latest_filings > 0 { 
                RefreshPriority::Medium 
            } else { 
                RefreshPriority::Low 
            },
            data_summary: DataSummary {
                date_range: latest_filing_date,
                stock_count: Some(total_stocks as i64),
                data_types: vec!["SEC Filings".to_string()],
                key_metrics: vec![
                    format!("{} stocks up-to-date", up_to_date_stocks),
                    format!("{} missing latest filings", missing_latest_filings),
                ],
                completeness_score: Some((up_to_date_stocks as f32 / total_stocks as f32) * 100.0),
            },
        })
    }
}
```

#### Phase 4: Smart Refresh Logic
```rust
// src-tauri/src/tools/data_refresh_orchestrator.rs
impl DataRefreshManager {
    /// Check if financial data needs refresh based on SEC Company Facts API
    async fn needs_financial_refresh(&self) -> Result<bool> {
        let edgar_client = SecEdgarClient::new(self.pool.clone());
        
        // Get stocks that haven't been checked recently (e.g., in last 24 hours)
        let stocks_to_check = self.get_stocks_needing_filing_check().await?;
        
        for (stock_id, symbol, cik) in stocks_to_check {
            // Check if this stock needs update using Company Facts API
            if edgar_client.check_if_update_needed(&cik, stock_id).await? {
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    /// Refresh financial data for stocks with new SEC filings
    async fn refresh_financials_smart(&self, session_id: &str) -> Result<i64> {
        let edgar_client = SecEdgarClient::new(self.pool.clone());
        
        // Get stocks that need refresh based on SEC Company Facts API
        let stocks_needing_refresh = self.get_stocks_needing_financial_refresh().await?;
        
        let mut total_records = 0;
        
        for (stock_id, symbol, cik) in stocks_needing_refresh {
            println!("📋 Processing {} ({})", symbol, cik);
            
            // Get our latest filing date to extract only new data
            if let Ok(Some(latest_date)) = edgar_client.get_our_latest_filing_date(stock_id).await {
                // Extract only new data points
                edgar_client.extract_new_data_only(&cik, stock_id, &latest_date).await?;
            } else {
                // No existing data, extract all
                edgar_client.extract_balance_sheet_data(&cik, stock_id, &symbol).await?;
            }
            
            total_records += 1;
            
            // Rate limiting
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        Ok(total_records)
    }
}
```

#### Phase 5: Enhanced Data Storage with Metadata
```rust
// src-tauri/src/tools/sec_edgar_client.rs
impl SecEdgarClient {
    /// Store financial data with filing metadata
    async fn store_financial_data_with_metadata(
        &self,
        stock_id: i64,
        data: CompanyFactsData
    ) -> Result<()> {
        for fact in data.facts {
            // Extract filing metadata
            let accession_number = fact.accn;
            let form_type = fact.form;
            let filed_date = fact.filed;
            let fiscal_period = fact.fp;
            let report_date = fact.end;
            
            // Store income statement data
            if let Some(income_data) = fact.income_data {
                self.store_income_statement_with_metadata(
                    stock_id,
                    income_data,
                    accession_number,
                    form_type,
                    filed_date,
                    fiscal_period,
                    report_date
                ).await?;
            }
            
            // Store balance sheet data
            if let Some(balance_data) = fact.balance_data {
                self.store_balance_sheet_with_metadata(
                    stock_id,
                    balance_data,
                    accession_number,
                    form_type,
                    filed_date,
                    fiscal_period,
                    report_date
                ).await?;
            }
            
            // Store cash flow data
            if let Some(cashflow_data) = fact.cashflow_data {
                self.store_cash_flow_with_metadata(
                    stock_id,
                    cashflow_data,
                    accession_number,
                    form_type,
                    filed_date,
                    fiscal_period,
                    report_date
                ).await?;
            }
        }
        
        Ok(())
    }
}
```

### Implementation Benefits

#### 1. Accurate Freshness Detection
- Uses actual SEC filing dates from Company Facts API
- Detects new filings immediately
- Eliminates false staleness based on days

#### 2. Efficient Refresh Strategy
- Only refreshes when new filings are available
- Incremental updates (only new data points)
- Reduces unnecessary API calls and processing

#### 3. Better User Experience
- Clear status messages based on actual filing dates
- Faster refresh cycles with incremental updates
- More reliable data freshness indicators

#### 4. Leverages Existing Infrastructure
- Uses our existing `SecEdgarClient` and Company Facts API
- No new API endpoints to learn or implement
- Builds on proven, working code

#### 5. Comprehensive Metadata Tracking
- Tracks accession numbers, form types, filing dates
- Enables data validation and completeness checks
- Supports audit trails and data lineage

### Implementation Timeline
1. **Week 1**: Add filing metadata columns to database tables
2. **Week 2**: Enhance `SecEdgarClient` with filing metadata extraction
3. **Week 3**: Implement smart freshness checker using Company Facts API
4. **Week 4**: Add incremental update logic and testing

### System Components

```
┌──────────────────────────────────────────────────────────────┐
│                 Stock Analysis Desktop App                   │
├──────────────────────────────────────────────────────────────┤
│  SolidJS Frontend (TSX) ←→ Tauri IPC ←→ Rust Backend        │
│         ↓                              ↓                     │
│  [Expandable Panels UI]       [Tauri Commands]               │
│  [Stock Row Management]       [Database Manager]             │
│  [Data Visualization]         [EDGAR API Client]             │
│  [User-Driven Analysis]       [Analysis Engine]              │
│                              [Future: Schwab API]            │
└──────────────────────────────────────────────────────────────┘
```

## Current Data Architecture - Pure SQL View Architecture

The system uses SEC EDGAR API for comprehensive financial data with a pure SQL view architecture that eliminates calculated tables and stale data:

### Pure SQL View Architecture

#### 1. Design Philosophy
- **Single Source of Truth**: EDGAR financial data + daily_prices only
- **On-Demand Calculations**: All metrics calculated in SQL views when queried
- **No Stale Data**: Eliminates calculated tables that can become outdated
- **Simplified Architecture**: Pure SQL views for all screening algorithms

#### 2. Data Sources
- **SEC EDGAR Company Facts API**: Real-time financial statements (Income, Balance Sheet, Cash Flow)
- **Schwab API**: Daily price data and market metrics
- **S&P 500 Coverage**: 503 stocks with comprehensive financial data
- **Data Types**: Annual financial statements for screening algorithms
- **Update Frequency**: Quarterly for financials, daily for prices

#### 3. Data Processing Architecture
**Phase 1**: EDGAR Financial Data Integration
- Real-time API calls to SEC EDGAR Company Facts API via `SecEdgarClient`
- Extract Income Statements, Balance Sheets, Cash Flow Statements
- Store Annual data for screening algorithms
- Integrated into `refresh_data --financials` command

**Phase 2**: Pure SQL View Calculations
- **Piotroski F-Score**: 9 criteria calculated directly from EDGAR data
- **O'Shaughnessy Value**: 6 metrics calculated from EDGAR + daily_prices
- **No Calculated Tables**: All metrics computed on-demand in SQL views
- **Real-time Accuracy**: Always uses latest available data

**Phase 3**: Data Quality Assessment
- Calculate data completeness scores in SQL views
- Validate financial statement consistency
- Generate data freshness reports

**Phase 4**: Performance Optimization
- Create database indexes for fast screening queries
- Optimize SQL views for real-time analysis
- Eliminate maintenance overhead of calculated tables

#### 4. Current Data Coverage
**Financial Data:**
- Income Statements: Revenue, Net Income, Operating Income, Gross Profit, Shares Outstanding
- Balance Sheets: Total Assets, Total Equity, Current Assets, Current Liabilities, Total Debt, Cash
- Cash Flow Statements: Operating Cash Flow, Dividends Paid, Share Repurchases, Depreciation
- Multi-year coverage: 5+ years of historical data for trend analysis

**Screening Algorithms:**
- **Piotroski F-Score**: 9-criteria financial strength scoring (pure SQL view)
- **O'Shaughnessy Value**: 6-metric value composite screening (pure SQL view)
- **Data completeness scoring**: Calculated in SQL views for quality assessment
- **Real-time screening**: Dynamic criteria with on-demand calculations

#### 5. O'Shaughnessy Value Metrics (Pure SQL View)
**6 Core Metrics Calculated On-Demand:**
1. **P/E Ratio**: `price / (net_income / shares_diluted)` from `daily_prices` + `income_statements`
2. **P/B Ratio**: `price / (total_equity / shares_outstanding)` from `daily_prices` + `balance_sheets`
3. **P/S Ratio**: `market_cap / revenue` from `daily_prices` + `income_statements`
4. **EV/S Ratio**: `enterprise_value / revenue` from `daily_prices` + `balance_sheets` + `income_statements`
5. **EV/EBITDA Ratio**: `enterprise_value / ebitda` from `daily_prices` + `balance_sheets` + `income_statements` + `cash_flow_statements`
6. **Shareholder Yield**: `(dividends_paid + share_repurchases) / market_cap` from `daily_prices` + `cash_flow_statements`

**Data Requirements:**
- All metrics calculated from raw EDGAR data + daily_prices
- No dependency on calculated tables or stale data
- Real-time accuracy with latest available data

### Current Database Schema (Pure SQL View Architecture)

```sql
-- Stocks table with S&P 500 integration
CREATE TABLE stocks (
    id INTEGER PRIMARY KEY,
    symbol TEXT UNIQUE NOT NULL,
    company_name TEXT NOT NULL,
    sector TEXT,
    industry TEXT,
    market_cap REAL,
    status TEXT DEFAULT 'active',
    first_trading_date DATE,
    last_updated DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    currency TEXT DEFAULT 'USD',
    shares_outstanding INTEGER,
    is_sp500 INTEGER DEFAULT 0  -- S&P 500 flag
);

-- Daily price data from Schwab API
CREATE TABLE daily_prices (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER NOT NULL,
    date DATE NOT NULL,
    price REAL NOT NULL,
    volume INTEGER,
    market_cap REAL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, date)
);

-- Income statements from EDGAR API
CREATE TABLE income_statements (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER NOT NULL,
    period_type TEXT NOT NULL,    -- 'Annual', 'Quarterly'
    report_date DATE NOT NULL,
    fiscal_year INTEGER,
    
    -- Core income metrics
    revenue REAL,
    gross_profit REAL,
    operating_income REAL,
    net_income REAL,
    shares_basic REAL,
    shares_diluted REAL,
    
    -- EDGAR metadata
    data_source TEXT DEFAULT 'sec_edgar_json',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, period_type, report_date)
);

-- Balance sheets from EDGAR API
CREATE TABLE balance_sheets (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER NOT NULL,
    period_type TEXT NOT NULL,    -- 'Annual', 'Quarterly'
    report_date DATE NOT NULL,
    fiscal_year INTEGER,
    
    -- Enterprise value components
    cash_and_equivalents REAL,
    short_term_debt REAL,
    long_term_debt REAL,
    total_debt REAL,
    
    -- Additional metrics
    total_assets REAL,
    total_liabilities REAL,
    total_equity REAL,
    current_assets REAL,
    current_liabilities REAL,
    shares_outstanding REAL,
    
    -- EDGAR metadata
    data_source TEXT DEFAULT 'sec_edgar_json',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, period_type, report_date)
);

-- Cash flow statements from EDGAR API
CREATE TABLE cash_flow_statements (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER NOT NULL,
    period_type TEXT NOT NULL,    -- 'Annual', 'Quarterly'
    report_date DATE NOT NULL,
    fiscal_year INTEGER,
    
    -- Cash flow metrics
    operating_cash_flow REAL,
    investing_cash_flow REAL,
    financing_cash_flow REAL,
    dividends_paid REAL,
    share_repurchases REAL,
    depreciation_expense REAL,
    amortization_expense REAL,
    
    -- EDGAR metadata
    data_source TEXT DEFAULT 'sec_edgar_json',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, period_type, report_date)
);

-- Performance indexes for fast analysis
CREATE INDEX idx_stocks_sp500 ON stocks(is_sp500);
CREATE INDEX idx_daily_prices_stock_date ON daily_prices(stock_id, date);
CREATE INDEX idx_income_statements_stock_period ON income_statements(stock_id, period_type, report_date);
CREATE INDEX idx_balance_sheets_stock_period ON balance_sheets(stock_id, period_type, report_date);
CREATE INDEX idx_cash_flow_statements_stock_period ON cash_flow_statements(stock_id, period_type, report_date);
```

### Pure SQL View Implementation

#### O'Shaughnessy Value Composite View
```sql
-- Pure SQL view calculating all 6 metrics on-demand
CREATE VIEW oshaughnessy_value_composite AS
WITH latest_data AS (
  SELECT 
    s.id as stock_id,
    s.symbol,
    s.sector,
    s.industry,
    
    -- Latest price data
    dp.price as current_price,
    dp.market_cap,
    
    -- Latest income statement data
    i.net_income,
    i.revenue,
    i.shares_diluted,
    
    -- Latest balance sheet data
    b.total_equity,
    b.shares_outstanding,
    b.total_debt,
    b.cash_and_equivalents,
    
    -- Latest cash flow data
    cf.dividends_paid,
    cf.share_repurchases,
    cf.depreciation_expense,
    cf.amortization_expense,
    
    -- Calculate enterprise value
    (dp.market_cap + COALESCE(b.total_debt, 0) - COALESCE(b.cash_and_equivalents, 0)) as enterprise_value,
    
    -- Calculate EBITDA
    (COALESCE(i.operating_income, 0) + COALESCE(cf.depreciation_expense, 0) + COALESCE(cf.amortization_expense, 0)) as ebitda,
    
    ROW_NUMBER() OVER (PARTITION BY s.id ORDER BY dp.date DESC) as price_rn,
    ROW_NUMBER() OVER (PARTITION BY s.id ORDER BY i.report_date DESC) as income_rn,
    ROW_NUMBER() OVER (PARTITION BY s.id ORDER BY b.report_date DESC) as balance_rn,
    ROW_NUMBER() OVER (PARTITION BY s.id ORDER BY cf.report_date DESC) as cashflow_rn
    
  FROM stocks s
  LEFT JOIN daily_prices dp ON s.id = dp.stock_id
  LEFT JOIN income_statements i ON s.id = i.stock_id AND i.period_type = 'Annual'
  LEFT JOIN balance_sheets b ON s.id = b.stock_id AND b.period_type = 'Annual'
  LEFT JOIN cash_flow_statements cf ON s.id = cf.stock_id AND cf.period_type = 'Annual'
  WHERE s.is_sp500 = 1
)
SELECT 
  stock_id,
  symbol,
  sector,
  industry,
  current_price,
  market_cap,
  enterprise_value,
  
  -- Calculate all 6 O'Shaughnessy metrics
  CASE WHEN net_income > 0 AND shares_diluted > 0 
       THEN current_price / (net_income / shares_diluted) 
       ELSE NULL END as pe_ratio,
  
  CASE WHEN total_equity > 0 AND shares_outstanding > 0 
       THEN current_price / (total_equity / shares_outstanding) 
       ELSE NULL END as pb_ratio,
  
  CASE WHEN revenue > 0 
       THEN market_cap / revenue 
       ELSE NULL END as ps_ratio,
  
  CASE WHEN revenue > 0 
       THEN enterprise_value / revenue 
       ELSE NULL END as evs_ratio,
  
  CASE WHEN ebitda > 0 
       THEN enterprise_value / ebitda 
       ELSE NULL END as ev_ebitda_ratio,
  
  CASE WHEN market_cap > 0 
       THEN (COALESCE(dividends_paid, 0) + COALESCE(share_repurchases, 0)) / market_cap 
       ELSE NULL END as shareholder_yield,
  
  -- Data completeness score
  ((CASE WHEN net_income > 0 AND shares_diluted > 0 THEN 1 ELSE 0 END) +
   (CASE WHEN total_equity > 0 AND shares_outstanding > 0 THEN 1 ELSE 0 END) +
   (CASE WHEN revenue > 0 THEN 1 ELSE 0 END) +
   (CASE WHEN revenue > 0 THEN 1 ELSE 0 END) +
   (CASE WHEN ebitda > 0 THEN 1 ELSE 0 END) +
   (CASE WHEN market_cap > 0 THEN 1 ELSE 0 END)) * 16.67 as data_completeness_score

FROM latest_data
WHERE price_rn = 1 AND income_rn = 1 AND balance_rn = 1 AND cashflow_rn = 1
  AND market_cap > 200000000;  -- $200M minimum
```

#### O'Shaughnessy Ranking View
```sql
-- Ranking view with composite scoring
CREATE VIEW oshaughnessy_ranking AS
WITH ranked AS (
  SELECT *,
    RANK() OVER (ORDER BY pe_ratio ASC) as pe_rank,
    RANK() OVER (ORDER BY pb_ratio ASC) as pb_rank,
    RANK() OVER (ORDER BY ps_ratio ASC) as ps_rank,
    RANK() OVER (ORDER BY evs_ratio ASC) as evs_rank,
    RANK() OVER (ORDER BY ev_ebitda_ratio ASC) as ebitda_rank,
    RANK() OVER (ORDER BY shareholder_yield DESC) as yield_rank,
    COUNT(*) OVER () as total_stocks
  FROM oshaughnessy_value_composite
  WHERE pe_ratio IS NOT NULL AND pb_ratio IS NOT NULL 
    AND ps_ratio IS NOT NULL AND evs_ratio IS NOT NULL
    AND ev_ebitda_ratio IS NOT NULL AND shareholder_yield IS NOT NULL
)
SELECT *,
  -- Composite score (average of all ranks)
  CAST((pe_rank + pb_rank + ps_rank + evs_rank + ebitda_rank + yield_rank) / 6.0 AS REAL) as composite_score,
  
  -- Percentile ranking
  CAST(ROUND(((pe_rank + pb_rank + ps_rank + evs_rank + ebitda_rank + yield_rank) / 6.0 / total_stocks) * 100, 1) AS REAL) as composite_percentile,
  
  -- Overall ranking
  RANK() OVER (ORDER BY (pe_rank + pb_rank + ps_rank + evs_rank + ebitda_rank + yield_rank) / 6.0 ASC) as overall_rank,
  
  -- Pass screening if in top 10 stocks
  CASE WHEN 
    RANK() OVER (ORDER BY (pe_rank + pb_rank + ps_rank + evs_rank + ebitda_rank + yield_rank) / 6.0 ASC) <= 10
    THEN 1 ELSE 0 END as passes_screening
FROM ranked
ORDER BY composite_score ASC;
```

### Future Schema Extensions (Schwab API)

```sql
-- Future: Real-time quotes table for live data
CREATE TABLE real_time_quotes (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER,
    timestamp TIMESTAMP,
    bid_price REAL,
    ask_price REAL,
    last_price REAL,
    volume INTEGER,
    change_amount REAL,
    change_percent REAL,
    FOREIGN KEY (stock_id) REFERENCES stocks (id)
);

-- Future: Option chains data  
CREATE TABLE option_chains (
    id INTEGER PRIMARY KEY,
    stock_id INTEGER,
    expiration_date DATE,
    strike_price REAL,
    option_type TEXT, -- 'CALL' or 'PUT'
    bid REAL,
    ask REAL,
    last_price REAL,
    volume INTEGER,
    open_interest INTEGER,
    FOREIGN KEY (stock_id) REFERENCES stocks (id)
);
```

## Frontend Architecture (SolidJS)

**Status**: ✅ **SolidJS Implementation** (September 2025) - Modern reactive frontend

### Technology Stack
- **UI Framework**: SolidJS 1.9.9 with TypeScript
- **Build Tool**: Vite 7.1.5 with vite-plugin-solid
- **Styling**: Tailwind CSS 3.4.0
- **Language**: TypeScript with JSX preserve mode
- **State Management**: Signal-based reactive stores
- **Desktop Integration**: Tauri API 2.0.0

### Architecture Pattern
- **Store-based State Management**: Global reactive stores using SolidJS signals
- **Component Architecture**: Functional components with fine-grained reactivity
- **API Integration**: Typed service layer for Tauri backend communication
- **Performance**: Direct DOM updates without virtual DOM overhead

### Key Components
- `App.tsx` - Main application with screening interface
- `HeroSection.tsx` - Screening method selection and criteria
- `ResultsPanel.tsx` - Stock screening results display
- `StockBrowser.tsx` - Individual stock analysis
- `SimpleDataManagement.tsx` - Data refresh and system status

### State Stores
- `recommendationsStore.ts` - Screening algorithms, criteria, results
- `dataRefreshStore.ts` - Data refresh status and operations
- `uiStore.ts` - Panel visibility, modals, notifications

### Key Features
- **Piotroski F-Score Screening**: 9-criteria financial strength analysis
- **O'Shaughnessy Value Screening**: 6-metric value composite analysis
- **Real-time Results**: Dynamic screening with configurable criteria
- **Data Quality Indicators**: Completeness scores and freshness status
- **Responsive Design**: Modern UI with smooth animations

## Event-Driven Data Refresh Architecture

**Status**: ✅ **Implemented** (September 2025) - Replaced complex polling with real-time event notifications

### System Overview
The data refresh system uses Tauri's event system for immediate backend-to-frontend communication, eliminating polling complexity and providing instant UI updates when refresh operations complete.

### Architecture Components

#### Backend Event Emission (`src-tauri/src/commands/data_refresh.rs:64`)
```rust
#[tauri::command]
pub async fn start_data_refresh(app_handle: tauri::AppHandle, request: RefreshRequestDto) -> Result<String, String> {
    // Start refresh operation
    let session_id = orchestrator.execute_refresh(refresh_request).await?;

    // Spawn background monitoring task
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            // Poll for completion and emit event
            if status == "completed" || status == "failed" {
                let _ = app_handle.emit("refresh-completed", serde_json::json!({
                    "mode": mode,
                    "session_id": session_id,
                    "status": status
                }));
                break;
            }
        }
    });
}
```

#### Frontend Event Listener (`src/src/stores/dataRefreshStore.ts:240`)
```typescript
// Listen for refresh completion events from backend
listen('refresh-completed', async (event: any) => {
    const { mode, status } = event.payload;
    console.log(`🎉 Refresh completed for ${mode}: ${status}`);

    // Remove from refreshing set
    setRefreshingCards(prev => {
        const newSet = new Set(prev);
        newSet.delete(mode);
        return newSet;
    });

    // Update freshness status immediately
    await checkDataFreshness();
    setIsRefreshing(false);
    stopProgressPolling();
});
```

### Event Flow
1. **User Action**: User clicks refresh button for any card (market, financials, ratios)
2. **Frontend Call**: `startRefresh(mode)` calls backend via Tauri IPC
3. **Backend Processing**: Orchestrator executes refresh operation asynchronously
4. **Monitoring Task**: Background task polls database every 2 seconds for completion
5. **Event Emission**: When complete, backend emits `refresh-completed` event
6. **Frontend Reception**: Event listener immediately updates UI state
7. **State Update**: Card shows updated status without manual refresh

### Multi-Card Support
The system handles all three refresh modes uniformly:
- **Market Data**: Daily prices, market cap updates
- **Financials**: Financial statements, ratios calculation
- **Ratios**: P/E, P/S, EV/S ratio updates

Each mode emits the same event structure ensuring consistent UI behavior across all cards.

### Benefits
- **Immediate Updates**: No waiting for polling intervals
- **Resource Efficient**: No continuous polling overhead
- **Consistent UX**: All three cards update instantly upon completion
- **Reliable State**: Event-driven prevents UI/backend state mismatches
- **Scalable**: Handles multiple concurrent refresh operations

## Frontend Architecture (Legacy - React)

### Current State: Backend Code Mixed with UI Components

**Issues Identified:**

1. **No API Service Layer**: All 29 `invoke()` calls are directly embedded in React components
2. **Inconsistent Error Handling**: Each component handles errors differently with custom logic
3. **Duplicate API Calls**: Same operations repeated across multiple components
4. **Tight Coupling**: UI components directly depend on backend API structure
5. **No Caching**: No data caching or request deduplication
6. **Hard to Test**: Cannot mock backend calls for unit testing
7. **Maintenance Nightmare**: Backend changes require touching multiple UI files

### Backend Actions Inventory

**Stock Operations (4):**
- `get_stocks_paginated` - Fetch paginated stock list
- `get_stocks_with_data_status` - Get all stocks with data status  
- `search_stocks` - Search stocks by query
- `get_sp500_symbols` - Get S&P 500 symbols list

**Analysis Operations (5):**
- `get_stock_date_range` - Get date range for stock data
- `get_price_history` - Get price history data
- `get_valuation_ratios` - Get P/S, EV/S ratios
- `get_ps_evs_history` - Get P/S and EV/S history
- `export_data` - Export stock data

**Recommendations Operations (2):**
- `get_undervalued_stocks_by_ps` - Get undervalued stocks by P/S ratio
- `get_value_recommendations_with_stats` - Get value recommendations with statistics

**System Operations (2):**
- `get_initialization_status` - Get system initialization status
- `get_database_stats` - Get database statistics

**Total: 13 unique backend operations across 4 components**

### Solution Design: Clean Architecture

#### Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    UI Components                            │
│  (App.jsx, AnalysisPanel.jsx, RecommendationsPanel.jsx)   │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      │ Uses
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                Data Service Layer                          │
│  (stockDataService, analysisDataService, etc.)            │
│  • Business logic & data transformation                    │
│  • Complex operations combining multiple API calls        │
│  • Data aggregation and caching                            │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      │ Uses
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                  API Service Layer                         │
│  (stockAPI, analysisAPI, recommendationsAPI, etc.)        │
│  • Direct invoke() calls to Tauri backend                 │
│  • Consistent error handling                               │
│  • Response normalization                                  │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      │ Uses
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                Tauri Backend                              │
│  (Rust commands and database operations)                   │
└─────────────────────────────────────────────────────────────┘
```

#### Service Layer Structure

**1. `api.js` - Raw API Layer**
- Contains all direct `invoke()` calls to Tauri backend
- Organized by functional areas (stock, analysis, recommendations, etc.)
- Provides consistent error handling wrapper
- **Purpose**: Abstract Tauri-specific communication

**2. `dataService.js` - Business Logic Layer**  
- Contains complex data operations and business logic
- Handles data transformation and aggregation
- Provides higher-level operations that combine multiple API calls
- **Purpose**: Handle business rules and data processing

#### Design Principles

1. **Single Responsibility**: Each service handles one domain
2. **Dependency Inversion**: UI depends on abstractions, not concrete implementations
3. **Consistent Error Handling**: All services return normalized error responses
4. **Reusability**: Services can be used across multiple components
5. **Testability**: Services can be easily mocked for unit testing

### React Component Structure (Current Implementation)

```
frontend/src/
├── App.jsx                    # Main application - expandable panels system
├── components/
│   ├── StockRow.jsx          # Individual stock with expand controls
│   ├── ExpandablePanel.jsx   # Generic expandable container with animations
│   ├── AnalysisPanel.jsx     # User-driven metric analysis interface  
│   ├── DataFetchingPanel.jsx # Unified data fetching interface
│   └── RecommendationsPanel.jsx # Value recommendations interface
├── services/                  # NEW: Service layer architecture
│   ├── api.js               # Raw API layer with invoke() calls
│   ├── dataService.js       # Business logic layer
│   └── README.md            # Service layer documentation
└── utils/
    ├── formatters.js        # Data formatting utilities
    ├── calculations.js      # Financial calculations
    └── api.js              # Legacy API helper functions
```

### Current Features (Phase 3 Complete)
1. **Expandable Panel Interface**: Single-page stock list with contextual expansion
2. **User-Driven Analysis**: Dynamic metric selection (P/E, EPS, Price, Volume, etc.)  
3. **S&P 500 Filtering**: Toggle between all stocks and S&P 500 subset
4. **Paginated Stock Loading**: 50 stocks per page with load more functionality
5. **Real-Time Search**: Search stocks by symbol or company name
6. **Visual Data Indicators**: 📊 for stocks with data, 📋 for no data
7. **Multiple Panel Support**: Multiple stocks can have expanded panels simultaneously
8. **Smooth Animations**: Professional expand/collapse transitions

### Frontend Refactoring Strategy

#### Phase 1: Service Layer Creation ✅
- [x] Create `api.js` with all backend operations
- [x] Create `dataService.js` with business logic
- [x] Document architecture and design decisions

#### Phase 2: Component Refactoring 🔄
- [ ] Refactor `App.jsx` to use `stockDataService`
- [ ] Refactor `AnalysisPanel.jsx` to use `analysisDataService`
- [ ] Refactor `RecommendationsPanel.jsx` to use `recommendationsDataService`
- [ ] Refactor `DataFetchingPanel.jsx` to use `systemDataService`

#### Phase 3: Cleanup 🔄
- [ ] Remove all direct `invoke()` calls from components
- [ ] Remove unused imports (`@tauri-apps/api/core`)
- [ ] Add consistent error handling across all components
- [ ] Add loading states management

#### Phase 4: Optimization 🔄
- [ ] Add data caching where appropriate
- [ ] Implement request deduplication
- [ ] Add retry logic for failed requests
- [ ] Add request cancellation for component unmounting

#### Usage Examples

**Before (Mixed UI and Backend):**
```javascript
// In React component - BAD
const loadData = async () => {
  try {
    setLoading(true);
    const history = await invoke('get_price_history', {
      symbol: stock.symbol,
      startDate,
      endDate
    });
    setPriceHistory(history);
    setLoading(false);
  } catch (err) {
    setError(`Failed to fetch data: ${err}`);
    setLoading(false);
  }
};
```

**After (Clean Separation):**
```javascript
// In React component - GOOD
import { analysisDataService } from '../services/dataService.js';

const loadData = async () => {
  setLoading(true);
  const result = await analysisDataService.loadStockAnalysis(
    stock.symbol, 
    startDate, 
    endDate
  );
  
  if (result.error) {
    setError(result.error);
  } else {
    setPriceHistory(result.priceHistory);
    setValuationRatios(result.valuationRatios);
  }
  setLoading(false);
};
```

#### Expected Benefits

1. **Separation of Concerns**: UI components only handle UI logic
2. **Reusability**: API services can be used across multiple components
3. **Testability**: Services can be easily mocked for testing
4. **Consistency**: Centralized error handling and data transformation
5. **Maintainability**: Backend changes only require service layer updates
6. **Performance**: Data caching and request deduplication
7. **Developer Experience**: Clear separation makes code easier to understand and modify

## Current Backend Architecture (Tauri + SimFin)

### Tauri Backend Structure

```rust
src-tauri/src/
├── main.rs                   # Tauri application entry point
├── lib.rs                    # Library exports
├── commands/
│   ├── mod.rs               # Commands module exports
│   ├── stocks.rs            # Stock information commands
│   ├── analysis.rs          # Data analysis commands
│   ├── fetching.rs          # Data fetching commands (legacy)
│   └── earnings.rs          # Earnings data commands
├── database/
│   ├── mod.rs              # Database management (SQLx-based)
│   ├── helpers.rs          # Database helper functions
│   ├── processing.rs       # Data processing operations
│   └── earnings.rs         # Earnings data operations
├── tools/
│   ├── mod.rs              # Tool modules
│   └── simfin_importer.rs  # SimFin CSV import system
├── bin/
│   └── import_simfin.rs    # SimFin import CLI tool
├── models/
│   └── mod.rs              # Data models and structures
├── analysis/
│   └── mod.rs              # Analysis engine
├── api/ (Future use)
│   ├── mod.rs              # API clients
│   ├── schwab_client.rs    # Schwab API client (preserved)
│   └── alpha_vantage_client.rs # Alpha Vantage client (legacy)
├── data_collector.rs        # Data collection logic
├── concurrent_fetcher.rs    # Concurrent processing utilities
└── utils.rs                 # Utility functions
```

### Current Tauri Commands (SimFin-Based)

```rust
// Stock information commands
#[tauri::command]
async fn get_stocks_paginated(limit: u32, offset: u32) -> Result<Vec<StockInfo>, String>

#[tauri::command]
async fn search_stocks(query: String) -> Result<Vec<StockInfo>, String>

#[tauri::command]
async fn get_stocks_with_data_status() -> Result<Vec<StockWithData>, String>

#[tauri::command]
async fn get_sp500_symbols() -> Result<Vec<String>, String>

// Analysis commands  
#[tauri::command]
async fn get_price_history(symbol: String, start_date: String, end_date: String) -> Result<Vec<PriceData>, String>

#[tauri::command]
async fn get_price_and_pe_data(symbol: String, start_date: String, end_date: String) -> Result<PriceAndPeData, String>

#[tauri::command]
async fn export_stock_data(symbol: String, format: String, start_date: String, end_date: String) -> Result<String, String>

// Database and statistics commands
#[tauri::command]
async fn get_database_stats() -> Result<DatabaseStats, String>

#[tauri::command]
async fn get_stock_summary(symbol: String) -> Result<StockSummary, String>

// Legacy/Future commands (preserved)
#[tauri::command]
async fn get_initialization_status() -> Result<InitProgress, String>

// SimFin Import (CLI tool - not Tauri command)
// cargo run --bin import-simfin -- --prices [CSV] --income [CSV]
```

### SimFin Import System

```rust
// Located in src-tauri/src/bin/import_simfin.rs and src-tauri/src/tools/simfin_importer.rs

pub async fn import_stocks_from_daily_prices(pool: &SqlitePool, csv_path: &str) -> Result<usize>
pub async fn import_daily_prices(pool: &SqlitePool, csv_path: &str, batch_size: usize) -> Result<usize>
pub async fn import_quarterly_financials(pool: &SqlitePool, csv_path: &str) -> Result<usize>
pub async fn calculate_and_store_eps(pool: &SqlitePool) -> Result<usize>
pub async fn calculate_and_store_pe_ratios(pool: &SqlitePool) -> Result<usize>
pub async fn add_performance_indexes(pool: &SqlitePool) -> Result<()>

// Usage:
// cargo run --bin import-simfin -- --prices ~/simfin_data/us-shareprices-daily.csv --income ~/simfin_data/us-income-quarterly.csv
```

## Current Implementation Status & Future Roadmap

### ✅ Completed Phases

**Phase 1: EDGAR API Integration (COMPLETE)**
- ✅ SEC EDGAR Company Facts API integration via `SecEdgarClient`
- ✅ Comprehensive financial data extraction (Income, Balance Sheet, Cash Flow)
- ✅ 994 income statements, 496 balance sheets, 496 cash flow statements
- ✅ S&P 500 coverage with latest financial data (2024-12-31)

**Phase 2: Pure SQL View Architecture (COMPLETE)**
- ✅ Piotroski F-Score: 9-criteria financial strength scoring (pure SQL view)
- ✅ O'Shaughnessy Value: 6-metric value composite screening (pure SQL view)
- ✅ Real-time calculations from raw EDGAR data + daily_prices
- ✅ No dependency on calculated tables or stale data

**Phase 3: Modern Desktop Frontend (COMPLETE)**  
- ✅ SolidJS frontend with signal-based reactivity
- ✅ Piotroski F-Score and O'Shaughnessy Value screening interfaces
- ✅ Real-time results with dynamic criteria
- ✅ Data quality indicators and freshness status

**Phase 4: Data Refresh System (COMPLETE)**
- ✅ Schwab API integration for daily price data
- ✅ Event-driven refresh system with real-time UI updates
- ✅ Data freshness monitoring and status reporting

### 🔄 Active Development

**Current Priority: Pure SQL View Implementation**
- 🔄 Create pure O'Shaughnessy SQL view without `daily_valuation_ratios` dependency
- 🔄 Integrate `SecEdgarClient` into `refresh_data --financials` command
- 🔄 Remove ratio calculation code and `daily_valuation_ratios` table
- 🔄 Update data freshness checker for pure SQL view architecture

### 🚀 Future Enhancements

**Phase 5: Advanced Analysis Tools**
1. **Technical Indicators**: Moving averages, RSI, MACD, Bollinger Bands
2. **Comparative Analysis**: Multi-stock comparison in expandable panels
3. **Sector Analysis**: Industry-wide trend analysis
4. **Portfolio Tracking**: Track and analyze custom stock portfolios

**Phase 6: Real-Time Features (Schwab API)**
1. **Real-Time Quotes**: Live price updates during market hours
2. **Options Data**: Options chain visualization and analysis
3. **Market News**: Real-time news feed integration
4. **Alert System**: Price and fundamental metric alerts

**Phase 7: Advanced Features**
1. **Custom Screening**: Build complex stock screens
2. **PDF Reports**: Export comprehensive analysis reports
3. **Data Sync**: Cloud backup and multi-device sync
4. **Advanced Charts**: Candlestick charts with overlays

## Pure SQL View Architecture Benefits

### Key Advantages
1. **Single Source of Truth**: EDGAR financial data + daily_prices only
2. **Real-time Accuracy**: All calculations use latest available data
3. **No Stale Data**: Eliminates calculated tables that can become outdated
4. **Simplified Maintenance**: No need to refresh calculated ratios
5. **Better Performance**: On-demand calculations optimized by SQL engine
6. **Easier Debugging**: All logic visible in SQL views
7. **Consistent Results**: Same calculation logic for all queries

### Implementation Strategy
1. **Phase 1**: Create pure O'Shaughnessy SQL view
2. **Phase 2**: Integrate EDGAR into `refresh_data --financials`
3. **Phase 3**: Remove ratio calculation code and `daily_valuation_ratios` table
4. **Phase 4**: Update data freshness checker for pure SQL view architecture
5. **Phase 5**: Test and validate with existing data

## Data Import Usage

### SimFin Data Import Commands

```bash
# From project root directory (recommended)
cargo run --bin import-simfin -- \
  --prices ~/simfin_data/us-shareprices-daily.csv \
  --income ~/simfin_data/us-income-quarterly.csv \
  --db stocks.db

# Alternative: From src-tauri directory
cd src-tauri
cargo run --bin import_simfin -- \
  --prices ~/simfin_data/us-shareprices-daily.csv \
  --income ~/simfin_data/us-income-quarterly.csv \
  --db ../stocks.db
```

### Expected Performance
- **Data Processing**: 15-30 minutes for full dataset
- **Records Imported**: ~6.2M price records + ~52k financial records
- **Database Size**: 2-3 GB final size
- **EPS & P/E Calculations**: Automated during import

## Database Migration Strategy

### Migration Steps
1. **Create Migration Scripts**: SQL scripts for schema changes
2. **Data Backup**: Export current data to CSV/JSON
3. **Schema Update**: Apply new table structure
4. **Data Import**: Migrate existing data to new format
5. **Index Creation**: Add performance indexes
6. **Verification**: Validate data integrity and completeness

### Migration Script Example
```rust
async fn migrate_to_enhanced_schema(db: &DatabaseManager) -> Result<(), String> {
    // Step 1: Create backup tables
    db.execute("CREATE TABLE stocks_backup AS SELECT * FROM stocks").await?;
    db.execute("CREATE TABLE daily_prices_backup AS SELECT * FROM daily_prices").await?;

    // Step 2: Create new enhanced tables
    db.execute(CREATE_STOCKS_ENHANCED_SQL).await?;
    db.execute(CREATE_DAILY_PRICES_ENHANCED_SQL).await?;

    // Step 3: Migrate existing data
    migrate_stocks_data(db).await?;
    migrate_price_data(db).await?;

    // Step 4: Verify data integrity
    verify_migration(db).await?;

    // Step 5: Drop backup tables (optional)
    // db.execute("DROP TABLE stocks_backup").await?;
    
    Ok(())
}
```

## Current System Performance

### Achieved Metrics
- **Data Coverage**: 5,876+ stocks with comprehensive historical data (2019-2024)
- **Database Performance**: Optimized with performance indexes for fast queries
- **UI Responsiveness**: <100ms response time for expandable panel interactions
- **Data Quality**: Professional-grade SimFin data with calculated fundamentals
- **Application Performance**: Smooth desktop application with paginated loading

### System Architecture Benefits
- **Offline-First**: Full functionality without internet connectivity
- **Comprehensive Data**: Both price and fundamental data in single system
- **Modern UI**: Expandable panels eliminate tab navigation complexity
- **Professional Quality**: SimFin institutional-grade financial data
- **Scalable Design**: Modular architecture supports future API integrations
- **Enterprise Safety**: Production-grade database safeguards and backup system

## Database Migration & Safety System

### Enterprise-Grade Database Protection

#### Database Manager
```rust
// Located in src-tauri/src/database/migrations.rs
pub struct DatabaseManager {
    pool: SqlitePool,
    db_path: String,
}

impl DatabaseManager {
    // Automatic backup before any operations
    pub async fn create_backup(db_path: &str) -> Result<String, Box<dyn std::error::Error>>
    
    // Production database detection with safeguards
    pub async fn verify_data_safety(&self) -> Result<DatabaseStats, Box<dyn std::error::Error>>
    
    // Safe migration runner with multiple verification steps
    pub async fn run_migrations_safely(&self) -> Result<(), Box<dyn std::error::Error>>
}
```

#### Safety Features
1. **Production Detection**: Automatically detects databases >50MB or >1000 stocks
2. **Automatic Backups**: Created before any schema changes with verification
3. **Data Integrity Verification**: Post-migration validation prevents data loss
4. **Rollback Support**: Timestamped backups for easy restoration
5. **Health Monitoring**: Real-time database statistics and alerts

#### Database Admin CLI Tool
```bash
# Check database health and statistics
cargo run --bin db_admin -- status
# Output: Shows stocks count, price records, size, and production warnings

# Create manual backup with verification
cargo run --bin db_admin -- backup
# Output: Timestamped backup in backups/ directory

# Run migrations with safety checks (requires explicit confirmation)
cargo run --bin db_admin -- migrate --confirm
# Output: Multi-layer backup creation, verification, and rollback capabilities

# Verify database integrity  
cargo run --bin db_admin -- verify
# Output: Comprehensive health check and data validation
```

#### Migration Safety Process
1. **Pre-Migration Backup**: Automatic backup with size verification
2. **Production Detection**: Large database warning with confirmation requirement
3. **Migration Execution**: SQLx migrations with error handling
4. **Post-Migration Verification**: Data integrity checks prevent silent data loss
5. **Cleanup**: Optional backup management (keeps last 5 backups)

#### Backup System
```bash
# Automatic backup script
./backup_database.sh

# Creates: backups/stocks_backup_YYYYMMDD_HHMMSS.db
# Includes: Size verification, integrity checks, automatic cleanup
```

### Migration Architecture

#### SQLx Migration System
- **Migration Files**: Located in `src-tauri/migrations/`
- **Automatic Tracking**: SQLx manages applied migrations in `_sqlx_migrations` table
- **Additive Only**: Migrations designed to add features, not destroy data
- **Production Safe**: Explicit confirmation required for large databases

#### Protected Initialization
```rust
// Located in src-tauri/src/database/protected_init.rs
pub async fn initialize_database_safely(db_path: &str) -> Result<SqlitePool, Box<dyn std::error::Error>>
pub async fn run_manual_migration(db_path: &str, confirm: bool) -> Result<(), Box<dyn std::error::Error>>
```

**Safety Levels:**
- **Small Databases** (<50MB, <100 stocks): Automatic migrations allowed
- **Medium Databases** (50MB-1GB, 100-1000 stocks): Backup + confirmation  
- **Production Databases** (>1GB, >1000 stocks): Manual confirmation required + multiple backups

#### Current Database Protection Status
```
Database: stocks.db (2,110.83 MB)
📊 Stocks: 5,892
📈 Price records: 6,198,657  
🏢 Financial records: 50,673
🚨 PRODUCTION DATABASE - Extra safeguards active
```

## Multi-Period Valuation Ratios System (P/S & EV/S)

### Overview
Extension to the existing P/E ratio system to include Price-to-Sales (P/S) and Enterprise Value-to-Sales (EV/S) ratios across multiple time periods (TTM, Annual, Quarterly) for comprehensive valuation analysis.

### Business Rationale
- **P/E Limitations**: P/E ratios become invalid when earnings are negative, limiting value investing analysis
- **Revenue-Based Ratios**: P/S and EV/S work with revenue (always positive), providing valuation metrics for unprofitable companies
- **Multi-Period Analysis**: Different time horizons serve different investment strategies (TTM for screening, Annual for trends, Quarterly for momentum)

### Technical Formulas

**Price-to-Sales (P/S) Ratio:**
```
P/S = Market Cap / Revenue
P/S = (Stock Price × Shares Outstanding) / Revenue
```

**Enterprise Value-to-Sales (EV/S) Ratio:**
```
EV/S = Enterprise Value / Revenue
Where: Enterprise Value = Market Cap + Total Debt - Cash & Cash Equivalents
EV/S = (Market Cap + Total Debt - Cash) / Revenue
```

### Data Sources & Strategy

#### Available SimFin Data Files
- `us-income-ttm.csv` - **PRIMARY**: Trailing Twelve Months revenue data for standard ratios
- `us-income-annual.csv` - Annual revenue data for trend analysis
- `us-income-quarterly.csv` - Quarterly revenue for momentum analysis
- `us-balance-ttm.csv` - **PRIMARY**: TTM balance sheet data (Cash, Debt)
- `us-balance-annual.csv` - Annual balance sheet data
- `us-balance-quarterly.csv` - Quarterly balance sheet data

#### Import Priority Strategy
1. **TTM Data (Phase 1)** - Standard industry ratios for screening and comparison
2. **Annual Data (Phase 2)** - Long-term trend analysis for fundamental research  
3. **Quarterly Data (Phase 3)** - Short-term momentum for trading strategies

### Enhanced Database Schema

#### New Financial Data Tables
```sql
-- Multi-period income statements  
CREATE TABLE income_statements (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    period_type TEXT NOT NULL, -- 'TTM', 'Annual', 'Quarterly'
    report_date DATE NOT NULL,
    fiscal_year INTEGER,
    fiscal_period TEXT, -- NULL for TTM/Annual, 'Q1'-'Q4' for quarterly
    
    -- Core income metrics
    revenue REAL,
    gross_profit REAL,
    operating_income REAL,
    net_income REAL,
    shares_basic REAL,
    shares_diluted REAL,
    
    -- Import metadata
    currency TEXT DEFAULT 'USD',
    simfin_id INTEGER,
    publish_date DATE,
    data_source TEXT DEFAULT 'simfin',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (stock_id) REFERENCES stocks (id),
    UNIQUE(stock_id, period_type, report_date)
);

-- Multi-period balance sheets
CREATE TABLE balance_sheets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    period_type TEXT NOT NULL, -- 'TTM', 'Annual', 'Quarterly'
    report_date DATE NOT NULL,
    fiscal_year INTEGER,
    fiscal_period TEXT,
    
    -- Enterprise value components
    cash_and_equivalents REAL,
    short_term_debt REAL,
    long_term_debt REAL,
    total_debt REAL, -- Calculated: short_term + long_term
    
    -- Additional metrics
    total_assets REAL,
    total_liabilities REAL,
    total_equity REAL,
    shares_outstanding REAL,
    
    -- Import metadata
    currency TEXT DEFAULT 'USD',
    simfin_id INTEGER,
    data_source TEXT DEFAULT 'simfin',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (stock_id) REFERENCES stocks (id),
    UNIQUE(stock_id, period_type, report_date)
);

-- Enhanced daily ratios table
CREATE TABLE daily_valuation_ratios (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    date DATE NOT NULL,
    price REAL,
    
    -- Market metrics
    market_cap REAL, -- Stock Price × Shares Outstanding
    enterprise_value REAL, -- Market Cap + Total Debt - Cash
    
    -- Existing ratios (preserved)
    pe_ratio REAL,
    
    -- New multi-period ratios
    ps_ratio_ttm REAL,    -- PRIMARY: Standard P/S using TTM revenue
    ps_ratio_annual REAL, -- Annual P/S for trend analysis
    ps_ratio_quarterly REAL, -- Latest quarter P/S for momentum
    
    evs_ratio_ttm REAL,    -- PRIMARY: Standard EV/S using TTM revenue
    evs_ratio_annual REAL, -- Annual EV/S for trend analysis
    evs_ratio_quarterly REAL, -- Latest quarter EV/S for momentum
    
    -- Supporting data
    revenue_ttm REAL,      -- TTM revenue for calculations
    revenue_annual REAL,   -- Annual revenue
    revenue_quarterly REAL, -- Latest quarterly revenue
    
    -- Data quality tracking
    data_completeness_score INTEGER, -- 0-100 based on available ratios
    last_financial_update DATE,      -- Most recent financial data used
    
    FOREIGN KEY (stock_id) REFERENCES stocks (id),
    UNIQUE(stock_id, date)
);

-- Performance indexes for multi-period analysis
CREATE INDEX idx_income_statements_period_lookup ON income_statements(stock_id, period_type, report_date);
CREATE INDEX idx_balance_sheets_period_lookup ON balance_sheets(stock_id, period_type, report_date);
CREATE INDEX idx_daily_ratios_ps_ttm ON daily_valuation_ratios(ps_ratio_ttm);
CREATE INDEX idx_daily_ratios_evs_ttm ON daily_valuation_ratios(evs_ratio_ttm);
CREATE INDEX idx_daily_ratios_multi_period ON daily_valuation_ratios(stock_id, date, ps_ratio_ttm, evs_ratio_ttm);
```

### Implementation Architecture

#### Phase 1: TTM Data Import (Priority)
```rust
// New importer modules in src-tauri/src/tools/
pub mod ttm_importer {
    pub async fn import_ttm_income_statements(pool: &SqlitePool, csv_path: &str) -> Result<usize>;
    pub async fn import_ttm_balance_sheets(pool: &SqlitePool, csv_path: &str) -> Result<usize>;
}

// Enhanced CLI tool
cargo run --bin import_simfin -- \
    --prices ~/simfin_data/us-shareprices-daily.csv \
    --income-quarterly ~/simfin_data/us-income-quarterly.csv \
    --income-ttm ~/simfin_data/us-income-ttm.csv \        # NEW
    --balance-ttm ~/simfin_data/us-balance-ttm.csv \      # NEW
    --db stocks.db
```

#### Phase 2: Multi-Period Ratio Calculations
```rust
// Enhanced ratio calculation engine
pub struct RatioCalculator {
    pool: SqlitePool,
}

impl RatioCalculator {
    // Primary ratio calculations using TTM data
    pub async fn calculate_ps_ratios_ttm(&self) -> Result<usize>;
    pub async fn calculate_evs_ratios_ttm(&self) -> Result<usize>;
    
    // Multi-period calculations
    pub async fn calculate_all_period_ratios(&self, period_type: PeriodType) -> Result<usize>;
    
    // Data quality assessment
    pub async fn assess_data_completeness(&self) -> Result<DataQualityReport>;
}

pub enum PeriodType {
    TTM,        // Primary for standard ratios
    Annual,     // Long-term trend analysis  
    Quarterly,  // Short-term momentum
}
```

#### Phase 3: Enhanced Analysis Features
```rust
// New Tauri commands for multi-period analysis
#[tauri::command]
async fn get_valuation_ratios_multi_period(
    symbol: String, 
    period_types: Vec<PeriodType>
) -> Result<MultiPeriodRatios, String>;

#[tauri::command]
async fn screen_stocks_by_ratios(
    criteria: RatioScreeningCriteria
) -> Result<Vec<StockScreeningResult>, String>;

#[tauri::command]
async fn get_ratio_trend_analysis(
    symbol: String,
    start_date: String,
    end_date: String
) -> Result<RatioTrendData, String>;
```

### Investment Strategy Applications

#### Use Case Mapping
| Ratio Type | Investment Strategy | Data Source | Update Frequency |
|------------|-------------------|-------------|------------------|
| **TTM P/S & EV/S** | Standard valuation screening | TTM files | Quarterly |
| **Annual P/S & EV/S** | Long-term trend analysis, fundamental research | Annual files | Yearly |  
| **Quarterly P/S & EV/S** | Momentum trading, earnings-driven strategies | Quarterly files | Quarterly |

#### Stock Screening Enhancement
- **Value Investing**: Use TTM P/S < 2.0 when P/E is negative (unprofitable companies)
- **Growth Screening**: Compare quarterly vs annual P/S for acceleration
- **Sector Comparison**: EV/S ratios for cross-sector valuation comparisons
- **Quality Metrics**: Data completeness scores for reliable analysis

### Migration Strategy

#### Database Migration Plan
```sql
-- Migration 20250909000005_add_multi_period_ratios.sql
CREATE TABLE income_statements (...);
CREATE TABLE balance_sheets (...);  
CREATE TABLE daily_valuation_ratios (...);

-- Migrate existing P/E ratios to new table
INSERT INTO daily_valuation_ratios (stock_id, date, price, pe_ratio)
SELECT stock_id, date, close_price, pe_ratio 
FROM daily_prices 
WHERE pe_ratio IS NOT NULL;

-- Create performance indexes
CREATE INDEX idx_income_statements_period_lookup ...;
```

#### Data Import Workflow
1. **Import TTM Financial Data**: Revenue and balance sheet data
2. **Calculate TTM Ratios**: P/S and EV/S using most recent price data
3. **Validate Data Quality**: Ensure completeness and accuracy
4. **Import Annual Data**: Historical trend data for comparative analysis
5. **Import Quarterly Data**: Latest momentum indicators
6. **Performance Optimization**: Index creation and query optimization

### Expected Outcomes

#### Data Coverage Enhancement
- **Ratio Coverage**: Expand from P/E-only to P/E + P/S + EV/S across 3 time periods
- **Stock Analysis**: Enable valuation analysis for unprofitable growth companies
- **Investment Flexibility**: Support value, growth, and momentum investment strategies

#### Performance Metrics
- **Import Time**: ~45-60 minutes for full TTM + Annual + Quarterly dataset
- **Database Size**: Additional ~1-2GB for comprehensive multi-period data
- **Query Performance**: <50ms for multi-period ratio lookups with proper indexing
- **Data Quality**: >95% coverage for S&P 500 stocks with TTM ratios

#### Frontend Integration
- **Enhanced Recommendations Panel**: Include P/S and EV/S in stock screening
- **Multi-Period Analysis**: Toggle between TTM/Annual/Quarterly views
- **Ratio Comparison Charts**: Visual comparison of valuation ratios over time
- **Smart Filtering**: Auto-switch to P/S when P/E is invalid (negative earnings)

## Enhanced P/S Screening Algorithm Architecture

### Overview
Sophisticated algorithm to screen S&P 500 stocks for undervalued opportunities based on P/S ratio fluctuations AND revenue growth requirements. Combines statistical undervaluation detection with fundamental quality filters.

### Algorithm Evolution

#### Phase 1: Basic P/S Screening (Current)
- **Logic**: P/S < (Historical Mean - 0.5 × Std Dev) AND P/S < Historical Median
- **Data Requirements**: Minimum 20 historical data points
- **Limitations**: No revenue growth consideration, simple statistical threshold

#### Phase 2: Enhanced P/S Screening (Proposed)
- **Logic**: P/S < (Historical Median - 1.0 × Std Dev) AND Revenue Growth > 0%
- **Data Requirements**: Minimum 50 historical data points
- **Enhancements**: Revenue growth validation, quality scoring, enhanced Z-score

### Enhanced Algorithm Design

#### 1. Data Sources
- **S&P 500 Symbols**: From `sp500_symbols` table (503 stocks)
- **Historical P/S Data**: From `daily_valuation_ratios` table (4-5 years of data)
- **Revenue Growth Data**: From `income_statements` table (TTM and Annual periods)
- **Current P/S Data**: Latest available P/S ratios from TTM/annual data

#### 2. Statistical Analysis
**Enhanced Historical Statistics**:
- **Mean P/S**: Average P/S ratio over historical period
- **Median P/S**: Median P/S ratio over historical period (more robust than mean)
- **Standard Deviation**: P/S volatility measure
- **Min/Max P/S**: Historical range boundaries
- **Data Points**: Minimum 50 historical records required (vs 20 in basic)

**Revenue Growth Analysis**:
- **TTM Growth Rate**: (Current TTM Revenue - Previous TTM Revenue) / Previous TTM Revenue × 100
- **Annual Growth Rate**: (Current Annual Revenue - Previous Annual Revenue) / Previous Annual Revenue × 100
- **Growth Requirement**: Either TTM or Annual growth rate > 0%

#### 3. Enhanced Undervalued Detection Logic
**Triple Criteria Approach**:
```sql
-- Stock is undervalued if ALL THREE conditions are met:
1. Current P/S < (Historical Median - 1.0 × Std Dev)  -- Statistical undervaluation
2. Revenue Growth > 0% (TTM OR Annual)               -- Growth requirement
3. Quality Score >= 50                               -- Data quality filter
```

**Enhanced Quality Filters**:
- Minimum 50 historical data points (reliability)
- P/S ratio > 0.01 (avoid penny stocks)
- Market Cap > $500M (configurable minimum)
- Revenue growth validation (TTM or Annual > 0%)
- S&P 500 stocks only

#### 4. Enhanced Z-Score Calculation
```sql
-- Enhanced Z-score based on median (more robust than mean)
Z-Score = (Current P/S - Historical Median) / Historical Std Dev
```

### Backend Implementation

#### Enhanced Command: `get_enhanced_undervalued_stocks_by_ps`
**Parameters**:
- `stock_tickers: Vec<String>` - S&P 500 symbols to analyze
- `limit: Option<i32>` - Maximum results (default: 50)
- `minMarketCap: Option<f64>` - Minimum market cap (default: $500M)
- `minGrowthRate: Option<f64>` - Minimum growth rate filter (default: 0.0%)

**Return Type**: `Vec<EnhancedUndervaluedStock>`
```rust
pub struct EnhancedUndervaluedStock {
    pub stock_id: i32,
    pub symbol: String,
    pub current_ps: f64,
    
    // Historical P/S statistics
    pub historical_mean: f64,
    pub historical_median: f64,
    pub historical_stddev: f64,
    pub historical_min: f64,
    pub historical_max: f64,
    pub data_points: i32,
    
    // Revenue growth metrics
    pub current_ttm_revenue: Option<f64>,
    pub ttm_growth_rate: Option<f64>,
    pub current_annual_revenue: Option<f64>,
    pub annual_growth_rate: Option<f64>,
    
    // Enhanced metrics
    pub z_score: f64,
    pub quality_score: i32,
    pub is_undervalued: bool,
    
    // Market metrics
    pub market_cap: f64,
    pub price: f64,
    pub data_completeness_score: i32,
}
```

#### Legacy Command: `get_undervalued_stocks_by_ps` (Basic Algorithm)
**Parameters**:
- `stock_tickers: Vec<String>` - S&P 500 symbols to analyze
- `limit: Option<i32>` - Maximum results (default: 50)
- `minMarketCap: Option<f64>` - Minimum market cap (default: $500M)

**Return Type**: `Vec<SmartUndervaluedStock>`
```rust
pub struct SmartUndervaluedStock {
    pub stock_id: i32,
    pub symbol: String,
    pub current_ps: f64,
    pub historical_mean: f64,
    pub historical_median: f64,
    pub historical_min: f64,
    pub historical_max: f64,
    pub historical_variance: f64,  // Actually std_dev
    pub z_score: f64,
    pub is_undervalued: bool,
    pub market_cap: f64,
    pub price: f64,
    pub data_completeness_score: i32,
}
```

#### SQL Query Architecture
**Multi-CTE Approach**:
1. `sp500_stocks` - Filter to S&P 500 symbols only
2. `historical_ps_data` - Get all historical P/S data with row numbers
3. `current_data` - Latest P/S data (rn = 1)
4. `historical_stats` - Calculate mean, min, max, std_dev from historical data
5. `median_calc` - Calculate median using window functions
6. `median_data` - Extract median values
7. `market_stats` - Overall market statistics for context

### Frontend Integration Architecture

#### 1. Enhanced API Service Layer (`src/services/api.js`)
**Enhanced Function**: `getEnhancedUndervaluedStocksByPs(stockTickers, limit, minMarketCap, minGrowthRate)`
- Calls Tauri command `get_enhanced_undervalued_stocks_by_ps`
- Handles parameter mapping (camelCase ↔ snake_case)
- Error handling and response formatting

**Legacy Function**: `getUndervaluedStocksByPs(stockTickers, limit, minMarketCap)`
- Calls Tauri command `get_undervalued_stocks_by_ps` (basic algorithm)
- Maintains backward compatibility

#### 2. Enhanced Data Service Layer (`src/services/dataService.js`)
**Enhanced Function**: `loadEnhancedUndervaluedStocksByPs(stockTickers, limit, minMarketCap, minGrowthRate)`
- Business logic wrapper around enhanced API call
- Default parameter handling (minGrowthRate = 0.0%)
- Error handling and data transformation
- Returns structured result with success/error states

**Legacy Function**: `loadUndervaluedStocksByPs(stockTickers, limit, minMarketCap)`
- Maintains backward compatibility for basic algorithm

#### 3. Enhanced UI Component (`src/components/RecommendationsPanel.jsx`)
**Enhanced Integration Points**:
- **S&P 500 Symbol Loading**: Uses `stockDataService.loadSp500Symbols()`
- **Algorithm Selection**: Enhanced dropdown with "P/S Ratio (Enhanced)" option
- **Growth Rate Configuration**: New filter for minimum growth rate
- **Quality Score Display**: Shows data quality metrics
- **Results Display**: Transforms `EnhancedUndervaluedStock` to UI format

**Enhanced UI Flow**:
1. Load S&P 500 symbols on component mount
2. User selects "P/S Ratio (Enhanced)" screening type
3. User configures market cap filter, limit, and minimum growth rate
4. Call `recommendationsDataService.loadEnhancedUndervaluedStocksByPs(sp500Symbols, limit, minMarketCap, minGrowthRate)`
5. Transform results for display with historical statistics and growth metrics
6. Show undervalued stocks with enhanced reasoning including growth rates

**Enhanced UI Features**:
- **Growth Rate Filter**: Dropdown for minimum growth rate (0%, 5%, 10%, 15%, 20%)
- **Quality Score Indicator**: Visual indicator of data quality (0-100)
- **Growth Metrics Display**: Shows both TTM and Annual growth rates
- **Enhanced Reasoning**: More detailed explanation including growth validation

#### 4. Enhanced Data Transformation
**Enhanced Backend → Frontend Mapping**:
```javascript
const transformedRecommendations = result.stocks.map((stock, index) => ({
  rank: index + 1,
  symbol: stock.symbol,
  company_name: stock.symbol,
  current_pe: null,  // Not used in P/S screening
  ps_ratio_ttm: stock.current_ps,
  market_cap: stock.market_cap,
  
  // Enhanced reasoning with growth metrics
  reasoning: `Enhanced algorithm: P/S ${stock.current_ps.toFixed(2)} (Z-score: ${stock.z_score.toFixed(2)}) | TTM Growth: ${stock.ttm_growth_rate?.toFixed(1) || 'N/A'}% | Quality: ${stock.quality_score}/100`,
  
  // Enhanced algorithm specific fields
  historical_mean: stock.historical_mean,
  historical_median: stock.historical_median,
  historical_stddev: stock.historical_stddev,
  historical_min: stock.historical_min,
  historical_max: stock.historical_max,
  data_points: stock.data_points,
  
  // Revenue growth metrics
  current_ttm_revenue: stock.current_ttm_revenue,
  ttm_growth_rate: stock.ttm_growth_rate,
  current_annual_revenue: stock.current_annual_revenue,
  annual_growth_rate: stock.annual_growth_rate,
  
  // Enhanced metrics
  z_score: stock.z_score,
  quality_score: stock.quality_score,
  is_undervalued: stock.is_undervalued
}));
```

**Legacy Backend → Frontend Mapping** (for basic algorithm):
```javascript
const transformedRecommendations = result.stocks.map((stock, index) => ({
  rank: index + 1,
  symbol: stock.symbol,
  company_name: stock.symbol,
  current_pe: null,  // Not used in P/S screening
  ps_ratio_ttm: stock.current_ps,
  market_cap: stock.market_cap,
  reasoning: `Basic algorithm: P/S ${stock.current_ps.toFixed(2)} (Z-score: ${stock.z_score.toFixed(2)})`,
  // Basic algorithm specific fields
  historical_mean: stock.historical_mean,
  historical_median: stock.historical_median,
  historical_min: stock.historical_min,
  historical_max: stock.historical_max,
  historical_variance: stock.historical_variance,
  z_score: stock.z_score,
  is_undervalued: stock.is_undervalued
}));
```

### Enhanced Performance Characteristics

#### Enhanced Algorithm Performance
- **Query Time**: ~2-3 seconds for S&P 500 analysis (vs ~1 second for basic)
- **Data Requirements**: Minimum 50 historical data points per stock (vs 20 for basic)
- **Coverage**: ~80-90% of S&P 500 stocks (vs ~95% for basic)
- **Precision**: Higher precision, lower recall (fewer but higher quality results)
- **On-the-fly Calculation**: No caching, calculates statistics and growth rates in real-time
- **Efficient SQL**: Uses CTEs, window functions, and revenue growth joins for optimal performance

#### Basic Algorithm Performance (Legacy)
- **Query Time**: ~1 second for S&P 500 analysis
- **Data Requirements**: Minimum 20 historical data points per stock
- **Coverage**: ~95% of S&P 500 stocks
- **On-the-fly Calculation**: No caching, calculates statistics in real-time
- **Efficient SQL**: Uses CTEs and window functions for optimal performance

### Enhanced Error Handling
- **Data Validation**: Minimum historical data points requirement (50 for enhanced, 20 for basic)
- **Revenue Growth Validation**: Handles missing TTM/Annual revenue data gracefully
- **Quality Score Validation**: Ensures minimum quality score thresholds
- **Graceful Degradation**: Returns empty results if insufficient data
- **User Feedback**: Clear error messages for data issues and growth rate problems
- **Fallback Logic**: Handles missing historical statistics and revenue growth data
- **Algorithm Selection**: Users can fall back to basic algorithm if enhanced fails

## Production-Grade Testing Architecture

### Simplified Test Architecture (Current Implementation)

**Architecture**: Single consolidated test file with reliable database synchronization

#### Design Philosophy
Simple, reliable testing using production data copies with SQLite WAL mode for true concurrency. No complex incremental sync - just robust file copying when needed.

#### Test Structure

```
src-tauri/tests/
├── backend_tests.rs          # Single consolidated test file (16 tests)
└── helpers/
    ├── database.rs           # SimpleTestDatabase helper
    └── mod.rs               # Module exports
```

#### Database Strategy

**Simple Copy Approach**: Copy `db/stocks.db` to `db/test.db` when needed
- **First Run**: Full copy of production database (~2.7GB in ~500ms)
- **Subsequent Runs**: Reuse existing `test.db` if up-to-date (0ms)
- **Concurrent Access**: SQLite WAL mode enables true concurrent testing
- **Production Safety**: Read-only access to production database

#### Test Database Helper

```rust
// Located in src-tauri/tests/helpers/database.rs

pub struct SimpleTestDatabase {
    pub pool: SqlitePool,
    pub is_copy: bool,
}

impl SimpleTestDatabase {
    pub async fn new() -> Result<Self> {
        // Check if test.db exists and is up-to-date
        if Path::new(test_db_path).exists() {
            let prod_modified = fs::metadata(production_db)?.modified()?;
            let test_modified = fs::metadata(test_db_path)?.modified()?;
            
            if test_modified >= prod_modified {
                // No sync needed - reuse existing test.db
                return Ok(SimpleTestDatabase { pool, is_copy: false });
            }
        }
        
        // Copy production database to test.db
        std::fs::copy(production_db, test_db_path)?;
        Ok(SimpleTestDatabase { pool, is_copy: true })
    }
    
    pub async fn new_no_sync() -> Result<Self> {
        // For concurrent tests - connect to already copied test.db
        let pool = connect_to_test_database(&test_db_path).await?;
        Ok(SimpleTestDatabase { pool, is_copy: false })
    }
}
```

#### SQLite Configuration for Concurrency

```rust
// WAL mode + Connection pooling for true concurrency
SqlitePoolOptions::new()
    .max_connections(10)
    .min_connections(2)
    .acquire_timeout(Duration::from_secs(10))
    .idle_timeout(Some(Duration::from_secs(600)))
    .connect(&database_url).await
```

#### Test Execution Pattern

```rust
#[tokio::test]
async fn test_example() {
    // Setup: Connect to test database (copy of production)
    let test_db = SimpleTestDatabase::new().await.unwrap();
    set_test_database_pool(test_db.pool().clone()).await;
    
    // Test: Run backend command
    let result = get_stocks_paginated(5, 0).await.expect("Test failed");
    assert_eq!(result.len(), 5, "Should return 5 stocks");
    
    // Cleanup: Clear test pool and close connections
    clear_test_database_pool().await;
    test_db.cleanup().await.unwrap();
}
```

#### Test Results

**Current Status**: 16/16 tests passing (100% success rate)
- ✅ All functional tests pass (pagination, search, analysis, recommendations)
- ✅ All performance tests pass (response time validation)
- ✅ All concurrent access tests pass (WAL mode enabled)
- ✅ Tests run in ~2.7 seconds total execution time

#### Test Categories

**Functional Tests (13 tests)**:
1. `test_database_setup` - Database verification
2. `test_get_stocks_paginated` - Pagination functionality
3. `test_search_stocks` - Search functionality
4. `test_get_sp500_symbols` - S&P 500 symbol loading
5. `test_get_price_history` - Historical price data
6. `test_get_stock_date_range` - Date range validation
7. `test_get_valuation_ratios` - P/S and EV/S ratios
8. `test_get_ps_evs_history` - Historical ratio data
9. `test_get_undervalued_stocks_by_ps` - P/S screening
10. `test_get_value_recommendations_with_stats` - Recommendations
11. `test_get_initialization_status` - System status
12. `test_get_database_stats` - Database statistics
13. `test_export_data` - Data export functionality

**Performance Tests (3 tests)**:
1. `test_pagination_performance` - Pagination speed validation
2. `test_search_performance` - Search speed validation
3. `test_concurrent_access_performance` - Concurrent access validation

#### Key Benefits

1. **True Concurrency**: SQLite WAL mode enables simultaneous test execution
2. **Production Data**: Tests use real production data (5,892 stocks, 6.2M prices)
3. **Fast Execution**: Complete test suite runs in ~2.7 seconds
4. **Simple Architecture**: Single test file, minimal complexity
5. **Production Safety**: Zero risk to production database
6. **Reliable Sync**: Robust file copying with timestamp validation
7. **No Hanging**: Eliminated complex incremental sync that caused 60+ second hangs

#### Test Commands

```bash
# Run all backend tests
cargo test --test backend_tests --features test-utils

# Run specific test
cargo test test_database_setup --features test-utils -- --nocapture

# Run with verbose output
cargo test --test backend_tests --features test-utils -- --nocapture
```

#### Migration from Complex Architecture

**Before**: Multiple test files with complex intelligent sync system
- `integration_tests.rs`, `performance_tests.rs`, `safe_backend_tests.rs`
- Complex `ATTACH DATABASE` incremental sync
- Test hanging issues (60+ second delays)
- Multiple helper files with unused code

**After**: Single consolidated test file with simple copy strategy
- `backend_tests.rs` - All 16 tests in one file
- Simple file copy with timestamp validation
- Fast, reliable execution (~2.7 seconds total)
- Minimal helper code (`SimpleTestDatabase`)

#### Files Cleanup Completed

**Deleted Files**:
- `src-tauri/tests/integration_tests.rs`
- `src-tauri/tests/performance_tests.rs` 
- `src-tauri/tests/safe_backend_tests.rs`
- `src-tauri/tests/helpers/sync_report.rs`
- `src-tauri/tests/helpers/test_config.rs`

**Current Files**:
- `src-tauri/tests/backend_tests.rs` - Consolidated test suite
- `src-tauri/tests/helpers/database.rs` - SimpleTestDatabase helper
- `src-tauri/tests/helpers/mod.rs` - Module exports

### Test Implementation Details

#### Frontend API Coverage Analysis

**✅ IMPLEMENTED & USED BY FRONTEND** (13 commands):

**Stock Operations (4 commands)**:
1. `get_stocks_paginated(limit, offset)` - Core pagination for main stock list
2. `get_stocks_with_data_status()` - Get all stocks with data availability flags  
3. `search_stocks(query)` - Real-time stock search functionality
4. `get_sp500_symbols()` - S&P 500 filtering support

**Analysis Operations (5 commands)**:
5. `get_stock_date_range(symbol)` - Date range for stock data
6. `get_price_history(symbol, start_date, end_date)` - Historical price data
7. `get_valuation_ratios(symbol)` - P/S, EV/S ratio display
8. `get_ps_evs_history(symbol, start_date, end_date)` - Historical P/S & EV/S data
9. `export_data(symbol, format)` - Data export functionality

**Recommendations Operations (2 commands)**:
10. `get_undervalued_stocks_by_ps(max_ps_ratio, limit)` - P/S ratio screening
11. `get_value_recommendations_with_stats(limit)` - P/E based recommendations

**System Operations (2 commands)**:
12. `get_initialization_status()` - System status for UI
13. `get_database_stats()` - Database statistics display

#### Test Priority Strategy

**HIGH Priority Tests** (8 commands - 60% of functionality):
- Stock pagination, data status, S&P 500 filtering
- Price history, valuation ratios, P/S EV/S history
- P/S screening, P/E recommendations

**MEDIUM Priority Tests** (3 commands - 25% of functionality):
- Search functionality, date range validation, database statistics

**LOW Priority Tests** (2 commands - 15% of functionality):
- Data export, system status

#### Performance Benchmarks

**Response Time Targets**:
- **Stock Pagination**: <100ms for 50 stocks
- **Stock Search**: <200ms for query results
- **S&P 500 Filter**: <150ms for symbol loading
- **Price History**: <500ms for 1-year data
- **Valuation Ratios**: <300ms for P/S & EV/S calculation
- **Recommendations**: <1s for 20 recommendations with stats
- **Database Stats**: <200ms for statistics calculation

#### Future Work & Enhancements

**Performance Benchmarks**:
- Comprehensive performance validation across all commands
- Memory usage testing for large datasets
- Concurrent access stress testing
- Response time regression detection

**Integration Test Workflows**:
- Complete user journey tests (search → analyze → export)
- S&P 500 filter workflow validation
- Recommendations workflow cross-validation
- Error recovery workflow testing

**Advanced Testing Features**:
- Concurrent access testing with multiple simultaneous requests
- Memory usage validation for large dataset operations
- Database corruption recovery testing
- Edge case data scenarios (zero revenue, negative P/E, etc.)

**Continuous Integration Enhancements**:
- Automated test result reporting with coverage metrics
- Performance regression tracking over time
- Test data refresh automation
- CI/CD pipeline integration

---

## Enhanced P/S Screening Algorithm Architecture

### Overview
The enhanced P/S screening algorithm provides sophisticated undervaluation detection using historical statistical analysis combined with revenue growth requirements. This represents a significant upgrade from simple P/S ratio screening to a multi-dimensional value + growth hybrid approach.

### Algorithm Design

#### Core Screening Criteria
The algorithm screens stocks that meet **ALL THREE** conditions:

1. **Statistical Undervaluation**: Current P/S < (Historical Median - 1.0 × Standard Deviation)
2. **Revenue Growth Requirement**: TTM Revenue Growth > 0% (positive growth)
3. **Data Quality Filter**: Quality Score >= 50 (sufficient data completeness)

#### Enhanced Data Coverage
- **Annual Revenue Data**: ~500+ stocks with 4-5 years of annual revenue data
- **Quarterly Revenue Data**: ~500+ stocks with 16-20 quarters of quarterly revenue data  
- **TTM Revenue Data**: ~500+ stocks with 4-5 years of TTM revenue data
- **Balance Sheet Data**: Cash, debt data for EV/S calculations
- **S&P 500 Coverage**: ~95%+ coverage (vs previous 82.7%)

#### Statistical Analysis
- **Historical Period**: Last 4-5 years of P/S ratio data
- **Minimum Data Points**: >= 10 data points required for statistical validity
- **Statistical Measures**: Mean, Median, Standard Deviation, Min, Max
- **Z-Score Calculation**: (Current P/S - Historical Mean) / Historical Std Dev

#### Revenue Growth Analysis
- **Primary Metric**: TTM Revenue Growth Rate
- **Growth Calculation**: (Current TTM Revenue - Previous TTM Revenue) / Previous TTM Revenue × 100
- **Growth Threshold**: > 0% (positive growth required)
- **Data Validation**: Cross-reference with Annual revenue trends

### Backend Implementation

#### New Command: `get_enhanced_undervalued_stocks_by_ps`
```rust
#[tauri::command]
pub async fn get_enhanced_undervalued_stocks_by_ps(
    pool: &SqlitePool,
    min_market_cap: Option<f64>,
    max_results: Option<i32>,
    min_growth_rate: Option<f64>,
    min_quality_score: Option<i32>,
) -> Result<Vec<EnhancedUndervaluedStock>, String>
```

#### Enhanced Data Structure
```rust
pub struct EnhancedUndervaluedStock {
    pub stock_id: i32,
    pub symbol: String,
    pub current_ps: f64,
    
    // Historical P/S statistics
    pub historical_mean: f64,
    pub historical_median: f64,
    pub historical_stddev: f64,
    pub historical_min: f64,
    pub historical_max: f64,
    pub data_points: i32,
    
    // Revenue growth metrics
    pub current_ttm_revenue: Option<f64>,
    pub ttm_growth_rate: Option<f64>,
    pub current_annual_revenue: Option<f64>,
    pub annual_growth_rate: Option<f64>,
    
    // Enhanced metrics
    pub z_score: f64,
    pub quality_score: i32,
    pub is_undervalued: bool,
    
    // Market metrics
    pub market_cap: f64,
    pub price: f64,
    pub data_completeness_score: i32,
}
```

#### SQL Query Architecture
```sql
-- Enhanced P/S screening with statistical analysis
WITH historical_ps_stats AS (
    SELECT 
        stock_id,
        AVG(ps_ratio_ttm) as mean_ps,
        PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY ps_ratio_ttm) as median_ps,
        STDDEV(ps_ratio_ttm) as stddev_ps,
        MIN(ps_ratio_ttm) as min_ps,
        MAX(ps_ratio_ttm) as max_ps,
        COUNT(*) as data_points
    FROM daily_valuation_ratios 
    WHERE ps_ratio_ttm IS NOT NULL 
        AND ps_ratio_ttm > 0
        AND date >= date('now', '-5 years')
    GROUP BY stock_id
    HAVING COUNT(*) >= 10
),
revenue_growth_analysis AS (
    SELECT 
        s.id as stock_id,
        s.symbol,
        
        -- Current TTM revenue
        ttm_current.revenue as current_ttm_revenue,
        
        -- TTM revenue growth
        CASE 
            WHEN ttm_previous.revenue > 0 THEN 
                (ttm_current.revenue - ttm_previous.revenue) / ttm_previous.revenue * 100
            ELSE NULL 
        END as ttm_growth_rate,
        
        -- Data quality scoring
        CASE 
            WHEN ttm_current.revenue IS NOT NULL AND annual_current.revenue IS NOT NULL THEN 100
            WHEN ttm_current.revenue IS NOT NULL THEN 75
            ELSE 50
        END as quality_score
        
    FROM stocks s
    LEFT JOIN income_statements ttm_current ON s.id = ttm_current.stock_id 
        AND ttm_current.period_type = 'TTM'
        AND ttm_current.report_date = (
            SELECT MAX(report_date) FROM income_statements 
            WHERE stock_id = s.id AND period_type = 'TTM'
        )
    LEFT JOIN income_statements ttm_previous ON s.id = ttm_previous.stock_id 
        AND ttm_previous.period_type = 'TTM'
        AND ttm_previous.report_date = (
            SELECT MAX(report_date) FROM income_statements 
            WHERE stock_id = s.id AND period_type = 'TTM'
            AND report_date < ttm_current.report_date
        )
    LEFT JOIN income_statements annual_current ON s.id = annual_current.stock_id 
        AND annual_current.period_type = 'Annual'
        AND annual_current.report_date = (
            SELECT MAX(report_date) FROM income_statements 
            WHERE stock_id = s.id AND period_type = 'Annual'
        )
)
SELECT 
    s.id as stock_id,
    s.symbol,
    dp.close_price as price,
    dp.market_cap,
    dp.ps_ratio_ttm as current_ps,
    
    -- Historical statistics
    hps.mean_ps as historical_mean,
    hps.median_ps as historical_median,
    hps.stddev_ps as historical_stddev,
    hps.min_ps as historical_min,
    hps.max_ps as historical_max,
    hps.data_points,
    
    -- Revenue growth data
    rga.current_ttm_revenue,
    rga.ttm_growth_rate,
    rga.quality_score,
    
    -- Enhanced calculations
    CASE 
        WHEN hps.stddev_ps > 0 THEN 
            (dp.ps_ratio_ttm - hps.mean_ps) / hps.stddev_ps
        ELSE 0
    END as z_score,
    
    -- Undervaluation determination
    CASE 
        WHEN dp.ps_ratio_ttm < (hps.median_ps - 1.0 * hps.stddev_ps)
            AND rga.ttm_growth_rate > 0
            AND rga.quality_score >= 50
        THEN 1 ELSE 0
    END as is_undervalued
    
FROM stocks s
INNER JOIN sp500_symbols sp ON s.symbol = sp.symbol
INNER JOIN daily_prices dp ON s.id = dp.stock_id 
    AND dp.date = (SELECT MAX(date) FROM daily_prices WHERE stock_id = s.id)
INNER JOIN historical_ps_stats hps ON s.id = hps.stock_id
LEFT JOIN revenue_growth_analysis rga ON s.id = rga.stock_id
WHERE dp.market_cap > COALESCE(?, 500000000)  -- Default $500M minimum
    AND dp.ps_ratio_ttm IS NOT NULL
    AND dp.ps_ratio_ttm > 0
    AND hps.data_points >= 10
ORDER BY 
    CASE 
        WHEN dp.ps_ratio_ttm < (hps.median_ps - 1.0 * hps.stddev_ps)
            AND rga.ttm_growth_rate > 0
            AND rga.quality_score >= 50
        THEN (hps.median_ps - dp.ps_ratio_ttm) / hps.stddev_ps
        ELSE 0
    END DESC,
    rga.quality_score DESC
LIMIT COALESCE(?, 50);  -- Default 50 results
```

### Frontend Integration

#### Enhanced UI Components
- **Pre-filter Selection**: P/E vs P/S screening method selection
- **Advanced Filtering**: Growth rate, quality score, market cap filters
- **Statistical Display**: Historical P/S statistics, Z-scores, growth rates
- **Quality Indicators**: Data completeness scores and confidence levels

#### User Experience Improvements
- **Default P/S Screening**: P/S algorithm set as default (more sophisticated)
- **Collapsible Footer**: Space-efficient display of algorithm details
- **Real-time Filtering**: Dynamic result updates based on filter changes
- **Enhanced Tooltips**: Detailed explanations of statistical measures

### Performance Characteristics

#### Query Optimization
- **Indexed Lookups**: Optimized indexes for multi-period data analysis
- **CTE Performance**: Common Table Expressions for complex statistical calculations
- **Batch Processing**: Efficient handling of large datasets
- **Caching Strategy**: Intelligent caching of statistical calculations

#### Expected Performance
- **Query Time**: < 2 seconds for S&P 500 analysis
- **Memory Usage**: Optimized for large historical datasets
- **Scalability**: Supports expansion to full market coverage
- **Real-time Updates**: Efficient incremental data updates

### Error Handling and Validation

#### Data Quality Assurance
- **Statistical Validation**: Minimum data point requirements
- **Growth Rate Validation**: Revenue data consistency checks
- **Outlier Detection**: Statistical outlier identification and handling
- **Data Completeness**: Quality scoring based on available data

#### Error Recovery
- **Graceful Degradation**: Fallback to simpler algorithms if data insufficient
- **User Feedback**: Clear error messages and data quality indicators
- **Logging**: Comprehensive logging for debugging and monitoring
- **Validation Queries**: Post-import data integrity verification

### Migration and Deployment

#### Database Migration
- **Migration File**: `20250915000006_complete_revenue_import.sql`
- **Additive Changes**: No data destruction, only additions
- **Backup Strategy**: Automatic backups before migration
- **Rollback Support**: Timestamped backups for restoration

#### Import Process
- **Complete Import Tool**: `import_complete_revenue` binary
- **Batch Processing**: Efficient handling of large CSV files
- **Progress Tracking**: Real-time import progress indicators
- **Error Handling**: Robust error handling with detailed logging

#### Data Validation
- **Post-Import Verification**: Automated data integrity checks
- **Coverage Analysis**: S&P 500 coverage verification
- **Quality Metrics**: Data completeness and accuracy validation
- **Performance Testing**: Query performance validation

### Future Enhancements

#### Algorithm Improvements
- **Multi-Period Growth**: Annual + Quarterly + TTM growth analysis
- **Sector Analysis**: Sector-specific P/S ratio normalization
- **Market Cycle Awareness**: Economic cycle-adjusted screening
- **Machine Learning**: ML-enhanced undervaluation detection

#### Data Expansion
- **Full Market Coverage**: Expansion beyond S&P 500
- **International Markets**: Global stock screening capabilities
- **Alternative Data**: ESG, sentiment, and alternative data integration
- **Real-time Updates**: Live data feed integration

#### User Experience
- **Advanced Analytics**: Portfolio analysis and backtesting
- **Custom Screens**: User-defined screening criteria
- **Export Capabilities**: Data export for external analysis
- **Mobile Support**: Mobile-optimized interface
*Last Updated: 2025-09-10*
*Version: 3.4 - Consolidated Testing Architecture Documentation*