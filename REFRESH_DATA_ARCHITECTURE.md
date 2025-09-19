# 🔄 Refresh Data Architecture - Complete Design

## 🎯 Overview

This document defines the complete data architecture for the unified `refresh_data` binary, supporting all 4 screening algorithms: GARP, Graham Value, O'Shaughnessy Value Composite, and Piotroski F-Score.

## 📊 Complete Data Requirements Analysis

### **1. GARP Screening** (Growth At Reasonable Price)
**Data Sources**: Market + Financials + Ratios
- Current stock price, P/E ratio (TTM), EPS (TTM + Annual)
- EPS growth rate, Revenue (TTM + Annual), Revenue growth rate
- Net income (TTM), Net profit margin, Total debt, Total equity
- Debt-to-equity ratio

### **2. Graham Value Screening** (Benjamin Graham)
**Data Sources**: Market + Financials + Ratios
- Current stock price, P/E ratio, Revenue (TTM), Net income (TTM)
- Operating income, Total assets, Total equity, Total debt
- Cash and equivalents, Shares outstanding
- Historical revenue/income (1-year ago for comparison)

### **3. O'Shaughnessy Value Composite** ⭐ NEW
**Data Sources**: Market + Financials + Ratios + Enhanced Calculations
**6 Core Metrics**:
- **Price to Book ratio** = Price / (Total Equity / Shares)
- **Price to Sales ratio** = Price / (Revenue / Shares)
- **Price to Cash Flow ratio** = Price / (Operating Cash Flow / Shares)
- **Price to Earnings ratio** = Price / EPS
- **EV to EBITDA ratio** = Enterprise Value / EBITDA
- **Shareholder Yield** = Dividend Yield + Share Buyback Yield

**Additional Requirements**:
- 6-month price momentum (for final filtering)
- Market cap > $200M (for filtering)
- Operating Cash Flow, EBITDA, Enterprise Value
- Dividend payments, Share repurchases, Book value

### **4. Piotroski F-Score** ⭐ NEW
**Data Sources**: Market + Complete EDGAR Financials + Multi-year Ratios
**9 Criteria Categories**:

**Profitability (4 criteria)**:
- ROA = Net Income / Total Assets
- Positive Net Income, Positive Operating Cash Flow
- Operating CF > Net Income

**Leverage/Liquidity (3 criteria)**:
- Lower Long-term Debt (current vs prior year)
- Higher Current Ratio (current vs prior year)
- No New Share Issuance (shares outstanding comparison)

**Operating Efficiency (2 criteria)**:
- Higher Gross Margin (current vs prior year)
- Higher Asset Turnover (current vs prior year)

**Multi-year Comparison Requirements**:
- Current assets, Current liabilities, Long-term debt (current + prior year)
- Gross profit, Asset turnover, Multi-year financial data

## 🏗️ Clean 3-Option Architecture

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, clap::ValueEnum)]
pub enum RefreshMode {
    /// Market data from Schwab: prices, shares outstanding, market cap (~15min)
    Market,
    /// All EDGAR financial data: income, balance sheets, cash flows (~90min)
    Financials,
    /// All calculated ratios and metrics: P/E, P/S, Piotroski, O'Shaughnessy (~10min, requires Market + Financials)
    Ratios,
}
```

## 📋 Database Schema Requirements

### **Table 1: daily_prices** (Market Data)
```sql
-- From Schwab API
current_price, market_cap, shares_outstanding, volume
dividend_yield, beta, eps_basic
data_source, date, created_at
```

### **Table 2: income_statements** (EDGAR Financial Data)
```sql
-- From EDGAR SEC filings
revenue, net_income, operating_income, gross_profit
cost_of_revenue, research_development, selling_general_admin
depreciation_expense, amortization_expense, interest_expense
shares_basic, shares_diluted, fiscal_year, fiscal_period
period_type ('TTM', 'Annual', 'Quarterly') -- for multi-year comparison
edgar_accession, edgar_form, edgar_filed_date
```

### **Table 3: balance_sheets** (EDGAR Financial Data)
```sql
-- From EDGAR SEC filings
total_assets, total_equity, total_debt, cash_and_equivalents
current_assets, current_liabilities -- for Piotroski current ratio
inventory, accounts_receivable, accounts_payable
working_capital, retained_earnings
fiscal_year, period_type -- for multi-year comparison
edgar_accession, edgar_form
```

### **Table 4: cash_flow_statements** ⭐ NEW (EDGAR Financial Data)
```sql
-- From EDGAR SEC filings
operating_cash_flow, investing_cash_flow, financing_cash_flow
depreciation_amortization, capital_expenditures
dividends_paid, share_repurchases -- for O'Shaughnessy shareholder yield
fiscal_year, period_type
edgar_accession, edgar_form
```

### **Table 5: daily_valuation_ratios** (Calculated Ratios)
```sql
-- All calculated metrics (computed from Market + Financials)
-- Basic ratios
pe_ratio_ttm, ps_ratio_ttm, pb_ratio_ttm, pcf_ratio_ttm
ev_ebitda_ratio, shareholder_yield

-- O'Shaughnessy Value Composite components
value_composite_score, value_composite_rank
momentum_6m, market_cap_filter_passed

-- Piotroski F-Score components
piotroski_score, roa, current_ratio, debt_to_equity
gross_margin, asset_turnover, profitability_score
leverage_score, efficiency_score

-- Supporting calculations
enterprise_value, ebitda, ttm_eps, book_value_per_share
price_momentum_6m, shareholder_yield_total
```

## 🔄 Data Flow Dependencies

```
Market (Independent)     ──┐
                           ├──▶ Ratios (Calculated)
Financials (Independent) ──┘

Market:     Schwab API → daily_prices
Financials: EDGAR → income_statements + balance_sheets + cash_flow_statements
Ratios:     Market + Financials → daily_valuation_ratios (all calculated metrics)
```

## 🚀 Implementation Architecture

### Internal Function Structure
```rust
impl DataRefreshOrchestrator {
    // NO external cargo calls - all internal functions

    async fn refresh_market_internal(&self) -> Result<i64> {
        // Schwab price import logic moved here
        // Updates: daily_prices table
    }

    async fn refresh_financials_internal(&self) -> Result<i64> {
        // EDGAR extraction logic moved here
        // Updates: income_statements + balance_sheets + cash_flow_statements
    }

    async fn refresh_ratios_internal(&self) -> Result<i64> {
        // 1. Check prerequisites: market + financials current
        // 2. Calculate all ratios for all 4 algorithms:
        //    - Basic ratios (P/E TTM, P/S TTM, P/B TTM, PCF TTM)
        //    - O'Shaughnessy Value Composite (6 metrics + ranking)
        //    - Piotroski F-Score (9 criteria + scoring)
        //    - Enterprise value, EBITDA, multi-year comparisons
        // 3. Store in: daily_valuation_ratios
    }
}
```

### Dependency Logic
```rust
async fn execute_refresh(&self, mode: RefreshMode) -> Result<RefreshResult> {
    match mode {
        RefreshMode::Market => {
            // Independent - always runs
            self.refresh_market_internal().await
        }
        RefreshMode::Financials => {
            // Independent - always runs
            self.refresh_financials_internal().await
        }
        RefreshMode::Ratios => {
            // Dependent - check both market + financials are current first
            self.check_prerequisites(&["market", "financials"]).await?;
            self.refresh_ratios_internal().await
        }
    }
}
```

## 🎯 Algorithm Coverage

This architecture provides complete data support for:

✅ **GARP Screening** - All P/E, growth, profitability, and debt metrics
✅ **Graham Value Screening** - All value, asset, and historical comparison metrics
✅ **O'Shaughnessy Value Composite** - All 6 value metrics + momentum filtering + composite scoring
✅ **Piotroski F-Score** - All 9 criteria across profitability, leverage, and efficiency + multi-year comparisons

## 🔧 Migration Requirements

### New Columns Needed
```sql
-- Add to daily_valuation_ratios
ALTER TABLE daily_valuation_ratios ADD COLUMN pe_ratio_ttm REAL;
ALTER TABLE daily_valuation_ratios ADD COLUMN pb_ratio_ttm REAL;
ALTER TABLE daily_valuation_ratios ADD COLUMN pcf_ratio_ttm REAL;
ALTER TABLE daily_valuation_ratios ADD COLUMN ev_ebitda_ratio REAL;
ALTER TABLE daily_valuation_ratios ADD COLUMN shareholder_yield REAL;
ALTER TABLE daily_valuation_ratios ADD COLUMN value_composite_score REAL;
ALTER TABLE daily_valuation_ratios ADD COLUMN piotroski_score INTEGER;

-- Add indexes for performance
CREATE INDEX idx_daily_ratios_pe_ttm ON daily_valuation_ratios(pe_ratio_ttm);
CREATE INDEX idx_daily_ratios_value_composite ON daily_valuation_ratios(value_composite_score);
CREATE INDEX idx_daily_ratios_piotroski ON daily_valuation_ratios(piotroski_score);
```

### New Table Creation
```sql
-- cash_flow_statements table (if not exists)
-- See existing migration: 20250918_create_cash_flow_statements.sql
```

## 💡 Key Design Principles

1. **Single Binary**: One `refresh_data` with 3 clear options
2. **Internal Functions**: No external cargo command calls
3. **Clean Dependencies**: Market + Financials → Ratios
4. **Complete Coverage**: Support all 4 screening algorithms
5. **Logical Names**: market/financials/ratios (not prices/enhanced/complete)
6. **No Warnings**: Clean, production-ready code
7. **Progress Indicators**: Show real-time progress for long operations
8. **Comprehensive Ratios**: Single ratios refresh covers all algorithms

This architecture provides a solid foundation for implementing all current and future screening algorithms with clean, maintainable code.