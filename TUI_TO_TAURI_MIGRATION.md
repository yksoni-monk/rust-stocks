# Rust-Stocks TUI to Tauri + React Migration Plan

## 🎯 **Migration Overview**

**Objective**: Migrate from ratatui TUI to modern Tauri + React desktop application while preserving all existing business logic.

**Current Status**: 
- ✅ Basic Tauri + React project structure exists
- ✅ Frontend directory with Vite + React setup
- ✅ src-tauri directory with basic Tauri configuration
- ✅ Original src code moved to backend/ folder
- 🔄 Need to integrate existing business logic into Tauri backend

---

## 📁 **Current Project Structure**

```
rust-stocks/
├── backend/                     # Original src code (preserved)
│   ├── analysis/               # Analysis engine
│   ├── api/                    # Schwab API integration
│   ├── bin/                    # Binary entry points
│   ├── models/                 # Data models
│   ├── ui/                     # TUI code (to be removed)
│   ├── data_collector.rs       # Data collection logic
│   ├── database_sqlx.rs        # Database layer
│   ├── concurrent_fetcher.rs   # Concurrent fetching
│   ├── utils.rs               # Utilities
│   ├── lib.rs                 # Library entry
│   └── main.rs                # Original TUI main
├── src-tauri/                  # Tauri Rust backend
│   ├── src/
│   │   ├── main.rs            # Tauri main entry (basic)
│   │   └── lib.rs             # Tauri library
│   ├── Cargo.toml             # Basic Tauri deps
│   └── tauri.conf.json        # Tauri config
├── frontend/                   # React frontend
│   ├── src/
│   │   ├── App.jsx            # Basic React app
│   │   ├── main.jsx           # React entry
│   │   └── index.css          # Tailwind setup
│   ├── package.json           # Frontend deps
│   ├── vite.config.js         # Vite config
│   └── tailwind.config.js     # Tailwind config
└── root files (Cargo.toml, stocks.db, etc.)
```

---

## 🚀 **Migration Plan**

### **Phase 1: Integrate Business Logic into Tauri Backend**

#### **1.1 Update src-tauri/Cargo.toml with all dependencies**
```toml
[package]
name = "rust-stocks-tauri"
version = "0.1.0"
edition = "2021"

[dependencies]
tauri = { version = "2.0", features = ["protocol-asset"] }
tauri-plugin-log = "2.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "sqlite", "chrono", "migrate"] }
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1.0"
dotenvy = "0.15"
reqwest = { version = "0.12", features = ["json"] }
tracing = "0.1"
tracing-subscriber = "0.3"
```

#### **1.2 Copy business logic modules to src-tauri/src/**
- Copy `backend/models/` to `src-tauri/src/models/`
- Copy `backend/analysis/` to `src-tauri/src/analysis/`
- Copy `backend/api/` to `src-tauri/src/api/`
- Copy `backend/database_sqlx.rs` to `src-tauri/src/database_sqlx.rs`
- Copy `backend/data_collector.rs` to `src-tauri/src/data_collector.rs`
- Copy `backend/concurrent_fetcher.rs` to `src-tauri/src/concurrent_fetcher.rs`
- Copy `backend/utils.rs` to `src-tauri/src/utils.rs`

#### **1.3 Create Tauri command handlers**
Create `src-tauri/src/commands/` with:
- `mod.rs` - Module exports
- `stocks.rs` - Stock-related commands
- `data.rs` - Data collection commands  
- `analysis.rs` - Analysis commands

#### **1.4 Tauri Commands to Implement**
```rust
// Stock commands
get_all_stocks() -> Vec<Stock>
search_stocks(query: String) -> Vec<Stock>
get_stock_details(stock_id: i64) -> StockDetails

// Data collection commands
fetch_stock_data(request: FetchRequest) -> String
get_database_stats() -> DatabaseStats
cancel_fetch_operation() -> String

// Analysis commands
get_price_history(stock_id: i64, start_date: String, end_date: String) -> Vec<DailyPrice>
analyze_pe_trends(stock_id: i64) -> PeAnalysis
export_data(stock_id: i64, format: String, start_date: String, end_date: String) -> String
```

### **Phase 2: Enhance React Frontend**

#### **2.1 Update frontend dependencies**
Add to `frontend/package.json`:
```json
{
  "dependencies": {
    "@tauri-apps/api": "^2.0.0",
    "antd": "^5.12.0",
    "chart.js": "^4.4.0",
    "react-chartjs-2": "^5.2.0",
    "react-router-dom": "^6.20.0",
    "zustand": "^4.4.0",
    "date-fns": "^2.30.0"
  }
}
```

#### **2.2 Create main application structure**
- `src/App.jsx` - Main app with routing
- `src/components/Layout/` - Navigation and layout
- `src/pages/Dashboard/` - Dashboard overview
- `src/pages/DataCollection/` - Data fetching UI
- `src/pages/Analysis/` - Stock analysis with charts
- `src/services/` - Tauri API communication
- `src/stores/` - Zustand state management

#### **2.3 Key React Components**
1. **Dashboard**: Database stats, recent activity
2. **Data Collection**: Stock selection, date range, progress tracking
3. **Analysis**: Stock search, date picker, interactive charts
4. **Stock Selector**: Multi-select with search
5. **Progress Tracker**: Real-time fetch progress
6. **Chart Components**: Candlestick charts with P/E overlay

### **Phase 3: Data Collection UI**

#### **3.1 Stock Selection Component**
- Multi-select dropdown with search
- Filter by sector/industry
- Select all S&P 500 stocks option

#### **3.2 Date Range Picker**
- Calendar component for start/end dates
- Quick presets (1 month, 3 months, 1 year, YTD)
- Validation for date ranges

#### **3.3 Progress Tracking**
- Real-time progress bar
- Current stock being processed
- Success/error counts
- Cancel operation button
- Detailed logs with timestamps

### **Phase 4: Stock Analysis UI**

#### **4.1 Stock Search & Selection**
- Search stocks by symbol or company name
- Recent stocks list
- Favorites/watchlist functionality

#### **4.2 Interactive Charts**
- Candlestick price charts using Chart.js
- P/E ratio overlay (secondary Y-axis)
- Zoom and pan functionality
- Tooltip with detailed data
- Toggle between chart types (line, candlestick)

#### **4.3 Data Export**
- Export to CSV, JSON formats
- Date range selection for export
- Download progress indicator

### **Phase 5: Dashboard & Statistics**

#### **5.1 Database Overview**
- Total stocks count
- Total price records
- Data coverage percentage
- Last update timestamps

#### **5.2 Quick Actions**
- Jump to data collection
- Jump to analysis
- Recent search history

---

## 🛠️ **Implementation Steps**

### **Step 1: Tauri Backend Integration**
1. Update `src-tauri/Cargo.toml` with dependencies
2. Copy business logic modules from `backend/` to `src-tauri/src/`
3. Create command handler modules
4. Update `src-tauri/src/main.rs` to register commands
5. Test basic Tauri commands work

### **Step 2: React Frontend Setup**
1. Install additional frontend dependencies
2. Set up routing with React Router
3. Create basic page structure
4. Set up Zustand stores for state management
5. Create Tauri API service layer

### **Step 3: Data Collection UI**
1. Build stock selection component
2. Build date range picker
3. Integrate with Tauri data fetching commands
4. Add real-time progress tracking
5. Handle errors and cancellation

### **Step 4: Analysis UI**
1. Build stock search component
2. Integrate Chart.js for data visualization
3. Create candlestick price charts
4. Add P/E ratio overlay functionality
5. Implement data export features

### **Step 5: Dashboard & Polish**
1. Build dashboard with database stats
2. Add navigation between pages
3. Implement error handling and loading states
4. Add responsive design for different window sizes
5. Test full application flow

### **Step 6: Testing & Cleanup**
1. Test all Tauri commands
2. Test React components and user flows
3. Handle edge cases and errors
4. Remove unused TUI code from backend/
5. Update documentation

---

## 📝 **Key Migration Principles**

1. **Preserve All Business Logic**: Zero changes to core functionality
2. **Progressive Enhancement**: Build new UI while keeping backend intact
3. **Type Safety**: Use TypeScript for frontend type safety
4. **Modern UX**: Professional desktop application experience
5. **Real-time Updates**: Live progress and status updates
6. **Data Visualization**: Rich charts and graphs for analysis

---

## 🎯 **Success Criteria**

- ✅ All existing TUI functionality available in React UI
- ✅ Professional desktop application look and feel
- ✅ Interactive charts for stock price analysis
- ✅ Real-time progress tracking for data collection
- ✅ Responsive design for different window sizes
- ✅ No regression in existing business logic or data handling
- ✅ Improved user experience over terminal-based interface

---

## 📅 **Estimated Timeline**

- **Phase 1 (Backend)**: 2-3 days
- **Phase 2 (Frontend Setup)**: 1-2 days  
- **Phase 3 (Data Collection)**: 2-3 days
- **Phase 4 (Analysis UI)**: 3-4 days
- **Phase 5 (Dashboard)**: 1-2 days
- **Phase 6 (Testing)**: 1-2 days

**Total: 10-16 days**

---

**Ready to begin implementation!**