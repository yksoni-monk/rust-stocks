# Frontend Expandable Panels Implementation Context

**Date**: 2025-01-05  
**Phase**: Phase 3 - Frontend UX Restructuring  
**Current Status**: Ready to implement expandable panels system

## Project Overview
Stock analysis desktop app (Tauri + React) with Schwab API integration. Moving from tab-based navigation to expandable panels for better UX.

## Current Architecture
- **Backend**: Rust/Tauri with SQLite and Schwab API client
- **Frontend**: React with existing components:
  - `App.jsx` - Main app with tab navigation (needs refactor)
  - `EnhancedStockDetails.jsx` - Comprehensive stock analysis (reuse content)  
  - `EnhancedDataFetching.jsx` - Data fetching interface (reuse logic)

## Key Design Decisions Made

### 1. **Expandable Panels Over Modals**
- In-place expansion below stock rows
- Multiple panels can be open simultaneously for comparison
- Contextual information stays connected to relevant stock
- Familiar accordion-style interaction pattern

### 2. **User-Driven Analysis (No Basic vs Enhanced)**
- Single unified data fetching system
- Users choose specific metrics they want to analyze
- Dynamic chart generation based on selections
- No artificial data tiers - comprehensive system only

### 3. **Clean Information Architecture**
- Single page foundation with stock list always visible
- Progressive disclosure - show details on demand  
- Contextual actions (Analyze, Fetch) stay with stock rows
- Smooth animations and familiar UX patterns

## Implementation Plan

### Phase 1: Core Panel System
1. Create `ExpandablePanel.jsx` - Base component with animations
2. Create `StockRow.jsx` - Individual stock with expand controls
3. Refactor `App.jsx` - Remove tabs, single stock list
4. Implement expansion state management

### Phase 2: Content Integration
1. Create `AnalysisPanel.jsx` - User-selectable metrics + dynamic charts
2. Create `DataFetchingPanel.jsx` - Unified data fetching (no tiers)
3. Adapt existing component logic into new panel structure
4. Add smooth animations and loading states

### Phase 3: Polish
1. Multiple simultaneous panel expansion
2. Keyboard navigation (arrows, enter, esc)
3. Auto-scroll to keep expanded content visible
4. Performance optimization

## Technical Requirements

### State Management
```javascript
const [expandedPanels, setExpandedPanels] = useState({}); 
// Format: { stockId: 'analysis'|'fetch'|null }
const [stockData, setStockData] = useState({});
// Cached data to avoid refetching
```

### Component Structure
```
StockAnalysisApp
├── Header (search, bulk actions)
├── StocksList
│   └── StockRow (for each stock)
│       ├── StockSummary (symbol, company, data status)
│       ├── Actions ([📊 Analyze] [📥 Fetch])
│       └── ExpandablePanel (when expanded)
│           ├── AnalysisPanel OR
│           └── DataFetchingPanel
```

### Key Features
- **User-Driven Metrics**: Dropdown for P/E, EPS, Price, Volume, Dividends, etc.
- **Flexible Date Ranges**: 1M, 3M, 6M, 1Y, 2Y, Custom
- **Dynamic Charts**: Generated based on user selections
- **Unified Fetching**: Single comprehensive system, no "basic vs enhanced"
- **Contextual Actions**: All operations stay with relevant stock

## Files to Modify
1. `/frontend/src/App.jsx` - Remove tabs, implement expandable stock list
2. Create new components in `/frontend/src/components/`:
   - `ExpandablePanel.jsx`
   - `StockRow.jsx` 
   - `AnalysisPanel.jsx`
   - `DataFetchingPanel.jsx`
3. Adapt existing components' content/logic into new structure

## Success Metrics
- Faster task completion with contextual expansion
- Easy side-by-side stock comparison
- User-driven analysis (no predetermined views)
- Simplified codebase (no complex routing/modals)

## Next Steps
1. Create `ExpandablePanel` base component with smooth animations
2. Create `StockRow` component with expand/collapse controls  
3. Refactor `App.jsx` to single stock list layout
4. Implement basic expansion state management

---
**Last Updated**: 2025-01-05  
**Context Session**: Frontend UX restructuring to expandable panels