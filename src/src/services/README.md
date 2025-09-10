# Frontend Services Layer

This directory contains the centralized API service layer that separates backend operations from UI components.

## 📁 Files

- `api.js` - Raw API layer with direct `invoke()` calls to Tauri backend
- `dataService.js` - Business logic layer with complex data operations

## 📖 Documentation

**Complete architecture documentation is available at:**
**[`context/frontend_architecture.md`](../../context/frontend_architecture.md)**

This includes:
- Problem analysis and current state
- Solution design with clean architecture
- Migration strategy and implementation checklist
- Usage examples and expected benefits

## 🚀 Quick Start

```javascript
// Import the service you need
import { stockDataService } from './services/dataService.js';

// Use in your component
const result = await stockDataService.loadInitialStockData();
if (result.error) {
  setError(result.error);
} else {
  setStocks(result.stocks);
}
```
