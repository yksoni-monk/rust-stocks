# Data Analysis View - Complete Redesign

## 🎯 **New Design Goals**

**Simple, focused, and functional:**
1. **Stock Search** - Easy stock selection via search
2. **Date Range Selection** - User-friendly date picker
3. **Candlestick Chart** - Visual price data with P/E overlay
4. **Single Stock Focus** - One stock at a time, done well

## 🏗️ **Architecture Overview**

### **Component Breakdown**

```
DataAnalysisView
├── SearchComponent          # Stock search with autocomplete
├── DateRangeComponent      # Start/end date selection  
├── ChartComponent          # Candlestick + P/E chart
└── StatusComponent         # Loading/error states
```

## 📋 **Detailed Component Design**

### **1. SearchComponent**
```rust
pub struct SearchComponent {
    search_input: String,
    search_results: Vec<Stock>,
    selected_stock: Option<Stock>,
    is_searching: bool,
}
```

**Functionality:**
- Real-time search as user types (minimum 2 characters)
- Search by symbol or company name using fuzzy matching
- Show top 10 matches in dropdown
- Arrow keys for navigation, Enter to select

**UI Layout:**
```
┌─ Stock Search ────────────────────┐
│ Search: [AAPL________________]    │
│                                   │
│ Results:                          │
│ > AAPL - Apple Inc.              │
│   AAPLX - Apple Extended Fund     │
│   AAL - American Airlines        │
└───────────────────────────────────┘
```

### **2. DateRangeComponent**
```rust
pub struct DateRangeComponent {
    start_date: NaiveDate,
    end_date: NaiveDate,
    editing_field: DateField,
    date_input: String,
}

enum DateField {
    StartDate,
    EndDate,
}
```

**Functionality:**
- Default: Last 3 months
- Tab between start/end date fields
- Date format: YYYY-MM-DD
- Validation: start < end, not future dates
- Quick presets: 1M, 3M, 6M, 1Y

**UI Layout:**
```
┌─ Date Range ──────────────────────┐
│ From: [2024-06-01] To: [2024-09-01] │
│ Presets: [1M] [3M] [6M] [1Y]       │
└────────────────────────────────────┘
```

### **3. ChartComponent**
```rust
pub struct ChartComponent {
    chart_data: Vec<ChartDataPoint>,
    chart_type: ChartType,
    is_loading: bool,
}

pub struct ChartDataPoint {
    date: NaiveDate,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: Option<i64>,
    pe_ratio: Option<f64>,
}

enum ChartType {
    CandlestickOnly,
    CandlestickWithPE,
}
```

**Functionality:**
- ASCII-based candlestick chart using ratatui
- Dual Y-axis: Price (left), P/E ratio (right)
- Color coding: Green (up days), Red (down days)
- Scrollable for long date ranges
- Toggle P/E overlay with 'P' key

**UI Layout:**
```
┌─ AAPL: 2024-06-01 to 2024-09-01 ────────────────────────────────┐
│ Price                                                      P/E   │
│ 230 ┤                                                     ┤ 30  │
│     │  ╭─╮                                               │      │
│ 220 ┤  │ │    ╭─╮                                        ┤ 25  │
│     │  ╰─╯    │ │                                        │      │
│ 210 ┤         ╰─╯                                        ┤ 20  │
│     └─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┬────┘      │
│          Jun   Jul   Aug   Sep                                  │
│                                                                 │
│ 🟢 Last: $225.50  📈 +2.3% (1D)  📊 Vol: 45.2M  💰 P/E: 28.5   │
└─────────────────────────────────────────────────────────────────┘
```

## 🔄 **User Interaction Flow**

### **State Machine**
```
Initial → Searching → StockSelected → DateSelected → ChartLoaded
    ↑                     ↓              ↓             ↓
    └─── Error ←──────────┴──────────────┴─────────────┘
```

### **Key Bindings**
- **Tab**: Navigate between components
- **Enter**: Confirm selection/load chart
- **Esc**: Clear search/go back
- **P**: Toggle P/E overlay
- **R**: Refresh data
- **Arrow Keys**: Navigate search results/adjust dates

## 🛠️ **Implementation Plan**

### **Phase 1: Core Structure** ✅
1. Create new simplified `DataAnalysisView` struct
2. Remove all existing complex logic
3. Add basic component structure
4. Implement simple rendering

### **Phase 2: Search Component** 
1. Implement real-time stock search
2. Add fuzzy matching using existing `AnalysisEngine::search_stocks`
3. Create dropdown results UI
4. Add keyboard navigation

### **Phase 3: Date Range Component**
1. Implement date input validation
2. Add preset buttons (1M, 3M, etc.)
3. Create date picker UI
4. Add range validation

### **Phase 4: Chart Component**
1. Fetch price data for selected stock/range
2. Create ASCII candlestick rendering
3. Implement dual-axis display (Price + P/E)
4. Add color coding and summary stats

### **Phase 5: Integration & Polish**
1. Connect all components with state management
2. Add loading states and error handling
3. Implement keyboard shortcuts
4. Add data refresh capability

## 💾 **Data Layer Requirements**

### **New Database Methods Needed**
```rust
// Already exists - just use it
AnalysisEngine::search_stocks(query: &str) -> Vec<Stock>

// New method needed
DatabaseManagerSqlx::get_price_history_range(
    stock_id: i64, 
    start_date: NaiveDate, 
    end_date: NaiveDate
) -> Vec<DailyPrice>
```

### **Data Structures**
```rust
// Reuse existing
pub struct Stock { /* existing */ }
pub struct DailyPrice { /* existing */ }

// New for chart
pub struct ChartDataPoint {
    pub date: NaiveDate,
    pub open: f64,
    pub high: f64, 
    pub low: f64,
    pub close: f64,
    pub volume: Option<i64>,
    pub pe_ratio: Option<f64>,
}
```

## ⚡ **Performance Considerations**

1. **Search Debouncing**: Wait 300ms after user stops typing
2. **Result Limiting**: Max 10 search results
3. **Chart Data Limiting**: Max 1 year of data at once
4. **Caching**: Cache last search results and chart data
5. **Async Loading**: Non-blocking data fetching with loading indicators

## 🎨 **UI/UX Principles**

1. **Progressive Disclosure**: Show only relevant options at each step
2. **Clear Visual Hierarchy**: Search → Dates → Chart
3. **Immediate Feedback**: Real-time search, loading indicators
4. **Keyboard-First**: All functionality accessible via keyboard
5. **Error Recovery**: Clear error messages with suggested actions

## 🧪 **Testing Strategy**

1. **Unit Tests**: Each component logic separately
2. **Integration Tests**: Full workflow end-to-end
3. **Manual Tests**: User experience validation
4. **Performance Tests**: Large datasets, rapid typing

---

**This design eliminates the complexity of the current broken implementation and focuses on delivering a single, high-quality feature that users can actually use effectively.**