# Graham-Inspired Value Screener Architecture

## Executive Summary

This document outlines the complete architecture for implementing Benjamin Graham's value investing principles as a stock screener within our existing Rust-Tauri application. The implementation leverages our current data foundation (SimFin TTM data, S&P 500 symbols, existing P/S ratios) and extends it with Graham-specific calculations and screening logic.

## Data Availability Analysis

### ✅ Available Data (SimFin TTM Files)
Our `simfin_data/` folder contains comprehensive financial data:

1. **Income Statements** (`us-income-ttm.csv`)
   - Revenue (for P/S calculations)
   - Net Income (profitability filter)
   - Shares Outstanding (for EPS calculations)
   - Operating Income, Interest Expense (quality metrics)

2. **Balance Sheets** (`us-balance-ttm.csv`)
   - Total Assets, Total Equity (for P/B calculations)
   - Long Term Debt (debt-to-equity ratios)
   - Cash & Equivalents (financial strength)

3. **Share Prices** (`us-shareprices-latest.csv`)
   - Current prices (for P/E, P/B calculations)
   - Shares Outstanding (cross-validation)

4. **Existing Infrastructure**
   - S&P 500 symbol filtering (503 symbols)
   - P/S ratio calculation system
   - Multi-period database schema

### 🔄 Calculated/Derived Metrics Needed
1. **P/E Ratio** = Current Price ÷ (Net Income ÷ Shares Outstanding)
2. **P/B Ratio** = Current Price ÷ (Total Equity ÷ Shares Outstanding)
3. **Debt-to-Equity** = Long Term Debt ÷ Total Equity
4. **Profit Margin** = Net Income ÷ Revenue
5. **Revenue Growth** = (Current Revenue - Prior Revenue) / Prior Revenue

## Graham Screening Criteria Implementation

### Core Filters (Based on Research)

1. **Positive Earnings Filter**
   - Net Income > 0 (from income statements)
   - Eliminates unprofitable companies

2. **Moderate P/E Ratio**
   - P/E < 15 (Graham's traditional threshold)
   - Modern adaptation: P/E < 20-25 for current market conditions

3. **Low Price-to-Book Value**
   - P/B < 1.5 (Graham's preference)
   - Combined rule: P/E × P/B < 22.5 (Graham's flexible approach)

4. **Dividend Yield** (Optional)
   - Dividend yield ≥ 2/3 of AAA bond yield
   - Currently ~2.2% based on bond yields

5. **Financial Stability**
   - Debt-to-Equity < 1.0 (conservative approach)
   - Profit Margin > 5% (quality filter)

6. **Revenue Growth**
   - Revenue growth > 0% over 1-3 years
   - Ensures business stability

### Modern Adaptations for S&P 500

1. **Sector-Adjusted P/S Ratios**
   - Technology: P/S < 5.0
   - Utilities: P/S < 2.0
   - Financials: P/S < 2.5
   - General: P/S < 1.5

2. **Quality Score Composite**
   - Combines profit margin, debt ratios, revenue stability
   - Weighted scoring: 0-100 scale

## Technical Architecture

### Database Schema Extensions

```sql
-- New table for Graham screening results
CREATE TABLE graham_screening_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id INTEGER NOT NULL,
    symbol TEXT NOT NULL,
    screening_date DATE NOT NULL,
    
    -- Core Graham Metrics
    pe_ratio REAL,
    pb_ratio REAL,
    pe_pb_product REAL,
    dividend_yield REAL,
    debt_to_equity REAL,
    profit_margin REAL,
    revenue_growth_1y REAL,
    
    -- Screening Results
    passes_earnings_filter BOOLEAN,
    passes_pe_filter BOOLEAN,
    passes_pb_filter BOOLEAN,
    passes_pe_pb_combined BOOLEAN,
    passes_dividend_filter BOOLEAN,
    passes_debt_filter BOOLEAN,
    passes_quality_filter BOOLEAN,
    passes_growth_filter BOOLEAN,
    passes_all_filters BOOLEAN,
    
    -- Scoring
    graham_score REAL,
    value_rank INTEGER,
    quality_score REAL,
    
    -- Metadata
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (stock_id) REFERENCES stocks(id),
    UNIQUE(stock_id, screening_date)
);

-- Index for performance
CREATE INDEX idx_graham_screening_symbol_date ON graham_screening_results(symbol, screening_date);
CREATE INDEX idx_graham_screening_passes_all ON graham_screening_results(passes_all_filters, graham_score);
```

### Rust Backend Implementation

#### 1. Data Models (`src/models/graham_value.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrahamScreeningCriteria {
    pub max_pe_ratio: f64,
    pub max_pb_ratio: f64,
    pub max_pe_pb_product: f64,
    pub min_dividend_yield: f64,
    pub max_debt_to_equity: f64,
    pub min_profit_margin: f64,
    pub min_revenue_growth: f64,
    pub require_positive_earnings: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrahamScreeningResult {
    pub id: Option<i64>,
    pub stock_id: i64,
    pub symbol: String,
    pub company_name: Option<String>,
    
    // Financial Metrics
    pub current_price: Option<f64>,
    pub pe_ratio: Option<f64>,
    pub pb_ratio: Option<f64>,
    pub pe_pb_product: Option<f64>,
    pub dividend_yield: Option<f64>,
    pub debt_to_equity: Option<f64>,
    pub profit_margin: Option<f64>,
    pub revenue_growth_1y: Option<f64>,
    
    // Filter Results
    pub passes_earnings_filter: bool,
    pub passes_pe_filter: bool,
    pub passes_pb_filter: bool,
    pub passes_pe_pb_combined: bool,
    pub passes_dividend_filter: bool,
    pub passes_debt_filter: bool,
    pub passes_quality_filter: bool,
    pub passes_growth_filter: bool,
    pub passes_all_filters: bool,
    
    // Scores
    pub graham_score: Option<f64>,
    pub value_rank: Option<i32>,
    pub quality_score: Option<f64>,
    
    // Metadata
    pub screening_date: String,
    pub reasoning: String,
}
```

#### 2. Graham Screening Engine (`src/analysis/graham_screener.rs`)

```rust
pub struct GrahamScreener {
    db_pool: Pool<Sqlite>,
}

impl GrahamScreener {
    pub async fn run_screening(&self, criteria: &GrahamScreeningCriteria) -> Result<Vec<GrahamScreeningResult>> {
        // 1. Load S&P 500 stocks with financial data
        let stocks = self.load_stocks_with_financials().await?;
        
        // 2. Calculate Graham metrics for each stock
        let mut results = Vec::new();
        for stock in stocks {
            let result = self.calculate_graham_metrics(&stock, criteria).await?;
            results.push(result);
        }
        
        // 3. Apply screening filters
        self.apply_graham_filters(&mut results, criteria).await?;
        
        // 4. Calculate scores and rankings
        self.calculate_scores_and_rankings(&mut results).await?;
        
        // 5. Save results to database
        self.save_screening_results(&results).await?;
        
        // 6. Return filtered and ranked results
        Ok(results.into_iter()
            .filter(|r| r.passes_all_filters)
            .collect())
    }
    
    async fn calculate_graham_metrics(&self, stock: &StockWithFinancials, criteria: &GrahamScreeningCriteria) -> Result<GrahamScreeningResult> {
        // Calculate P/E ratio
        let pe_ratio = if let (Some(price), Some(net_income), Some(shares)) = 
            (stock.current_price, stock.net_income, stock.shares_outstanding) {
            if net_income > 0.0 && shares > 0.0 {
                Some(price / (net_income / shares))
            } else { None }
        } else { None };
        
        // Calculate P/B ratio
        let pb_ratio = if let (Some(price), Some(equity), Some(shares)) = 
            (stock.current_price, stock.total_equity, stock.shares_outstanding) {
            if equity > 0.0 && shares > 0.0 {
                Some(price / (equity / shares))
            } else { None }
        } else { None };
        
        // Additional calculations...
        
        Ok(GrahamScreeningResult {
            // Populate all fields
        })
    }
}
```

#### 3. Tauri Commands (`src/commands/graham_screening.rs`)

```rust
#[tauri::command]
pub async fn run_graham_screening(
    criteria: GrahamScreeningCriteria,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<GrahamScreeningResult>, String> {
    let screener = GrahamScreener::new(state.db_pool.clone());
    screener.run_screening(&criteria)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_graham_criteria_defaults() -> Result<GrahamScreeningCriteria, String> {
    Ok(GrahamScreeningCriteria {
        max_pe_ratio: 15.0,
        max_pb_ratio: 1.5,
        max_pe_pb_product: 22.5,
        min_dividend_yield: 2.0,
        max_debt_to_equity: 1.0,
        min_profit_margin: 5.0,
        min_revenue_growth: 0.0,
        require_positive_earnings: true,
    })
}

#[tauri::command]
pub async fn get_graham_screening_history(
    limit: Option<i32>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<GrahamScreeningResult>, String> {
    // Return historical screening results
}
```

### Frontend Integration

#### 1. Store Updates (`src/stores/recommendationsStore.ts`)

```typescript
interface GrahamCriteria {
  maxPeRatio: number;
  maxPbRatio: number;
  maxPePbProduct: number;
  minDividendYield: number;
  maxDebtToEquity: number;
  minProfitMargin: number;
  minRevenueGrowth: number;
  requirePositiveEarnings: boolean;
}

interface GrahamResult {
  symbol: string;
  companyName: string;
  currentPrice: number;
  peRatio: number;
  pbRatio: number;
  dividendYield: number;
  debtToEquity: number;
  profitMargin: number;
  revenueGrowth1y: number;
  passesAllFilters: boolean;
  grahamScore: number;
  valueRank: number;
  reasoning: string;
}

// Add to existing store
const [grahamCriteria, setGrahamCriteria] = createSignal<GrahamCriteria>({
  maxPeRatio: 15.0,
  maxPbRatio: 1.5,
  maxPePbProduct: 22.5,
  minDividendYield: 2.0,
  maxDebtToEquity: 1.0,
  minProfitMargin: 5.0,
  minRevenueGrowth: 0.0,
  requirePositiveEarnings: true,
});

const [grahamResults, setGrahamResults] = createSignal<GrahamResult[]>([]);

const runGrahamScreening = async () => {
  setLoading(true);
  try {
    const results = await invoke('run_graham_screening', {
      criteria: grahamCriteria(),
    });
    setGrahamResults(results);
    setScreeningType('graham_value');
  } catch (error) {
    setError(`Graham screening failed: ${error}`);
  } finally {
    setLoading(false);
  }
};
```

#### 2. UI Components

Update `HeroSection.tsx` to include Graham screening:

```typescript
<button
  onClick={() => handleAlternativeScreening('graham_value')}
  class="text-blue-600 hover:text-blue-800 text-sm font-medium px-4 py-2 rounded-lg hover:bg-blue-50 transition-colors"
>
  📈 Graham Value
</button>
```

Update `ResultsPanel.tsx` to display Graham-specific metrics and filters.

## Implementation Roadmap

### Phase 1: Database & Core Infrastructure (Week 1)
1. **Database Migration**
   - Create `graham_screening_results` table
   - Add indexes for performance
   - Test with existing data

2. **Data Models**
   - Implement Rust structs for Graham screening
   - Create serialization/deserialization
   - Add validation logic

3. **Basic Calculation Engine**
   - P/E, P/B ratio calculations
   - Financial health metrics
   - Basic filtering logic

### Phase 2: Screening Engine (Week 2)
1. **Core Screening Logic**
   - Implement all Graham filters
   - Scoring and ranking algorithms
   - S&P 500 integration

2. **Tauri Commands**
   - API endpoints for screening
   - Configuration management
   - Error handling

3. **Data Integration**
   - Connect to existing SimFin data
   - Handle missing data gracefully
   - Performance optimization

### Phase 3: Frontend Integration (Week 3)
1. **Store Updates**
   - Graham criteria management
   - Results handling
   - State synchronization

2. **UI Components**
   - Graham screening option in HeroSection
   - Dedicated results display
   - Criteria configuration panel

3. **User Experience**
   - Loading states
   - Error handling
   - Results visualization

### Phase 4: Testing & Optimization (Week 4)
1. **Unit Tests**
   - Calculation accuracy
   - Filter logic
   - Edge case handling

2. **Integration Tests**
   - End-to-end screening flow
   - Database operations
   - Frontend-backend communication

3. **Performance Optimization**
   - Query optimization
   - Caching strategies
   - UI responsiveness

## Testing Strategy

### Data Validation Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_pe_ratio_calculation() {
        // Test P/E ratio calculation with known values
        let stock = create_test_stock(100.0, 1000000.0, 10000000.0);
        let pe = calculate_pe_ratio(&stock).unwrap();
        assert_eq!(pe, 10.0);
    }
    
    #[tokio::test]
    async fn test_graham_filtering() {
        // Test complete Graham screening pipeline
        let criteria = GrahamScreeningCriteria::default();
        let stocks = create_test_stock_portfolio();
        let results = run_graham_screening(&stocks, &criteria).await.unwrap();
        
        // Verify filtering logic
        for result in results {
            if result.passes_all_filters {
                assert!(result.pe_ratio.unwrap_or(999.0) <= criteria.max_pe_ratio);
                assert!(result.pb_ratio.unwrap_or(999.0) <= criteria.max_pb_ratio);
            }
        }
    }
}
```

### Frontend Tests
```typescript
// Test store functionality
describe('Graham Screening Store', () => {
  it('should update criteria correctly', () => {
    const store = createRecommendationsStore();
    store.updateGrahamCriteria({ maxPeRatio: 12.0 });
    expect(store.grahamCriteria().maxPeRatio).toBe(12.0);
  });
  
  it('should handle screening results', async () => {
    const store = createRecommendationsStore();
    await store.runGrahamScreening();
    expect(store.grahamResults().length).toBeGreaterThan(0);
  });
});
```

## Success Metrics

### Functional Requirements
- ✅ Screen 500+ S&P 500 stocks in < 5 seconds
- ✅ Calculate accurate P/E, P/B, debt ratios
- ✅ Apply all 8 Graham filters correctly
- ✅ Return 10-50 qualifying stocks typically
- ✅ Integrate seamlessly with existing UI

### Performance Requirements
- ✅ Database queries < 1 second
- ✅ UI responsiveness maintained
- ✅ Memory usage < 100MB additional
- ✅ Handle concurrent users

### Quality Requirements
- ✅ 95%+ test coverage
- ✅ Error handling for missing data
- ✅ Input validation on all criteria
- ✅ Graceful degradation

## Future Enhancements

### Advanced Features
1. **Historical Backtesting**
   - Compare Graham picks vs S&P 500 performance
   - Risk-adjusted returns analysis
   - Sector performance breakdown

2. **Enhanced Scoring**
   - Machine learning quality scores
   - Sector-specific adjustments
   - Economic cycle considerations

3. **Portfolio Construction**
   - Diversification algorithms
   - Risk management rules
   - Rebalancing recommendations

### Integration Opportunities
1. **Real-time Data**
   - Live price updates
   - News sentiment integration
   - Earnings announcement tracking

2. **Advanced Analytics**
   - Monte Carlo simulations
   - Stress testing scenarios
   - Correlation analysis

This architecture provides a solid foundation for implementing Benjamin Graham's timeless value investing principles while leveraging our existing technology stack and data infrastructure.