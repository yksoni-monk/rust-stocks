# Data Refresh Frontend Integration Architecture

## Overview

This document outlines the architecture and implementation plan for integrating the data refresh system with the SolidJS frontend. The backend refresh system is already well-designed and fully functional with 7 Tauri commands available for frontend integration.

## Backend Architecture (Already Complete ✅)

### Available Tauri Commands
- `get_data_freshness_status()` - Check system data status
- `check_screening_readiness()` - Verify specific feature readiness
- `start_data_refresh()` - Initiate data refresh operations
- `get_refresh_progress()` - Monitor refresh progress
- `get_last_refresh_result()` - Get results of last refresh
- `cancel_refresh_operation()` - Cancel running refresh
- `get_refresh_duration_estimates()` - Get time estimates

### Three Core Data Types
1. **📈 Market Data (Schwab)**
   - Daily prices, shares outstanding, market cap
   - ~15 minute refresh time
   - Status: Fresh/Stale (daily updates needed)

2. **📋 Financial Data (EDGAR)**
   - Income statements, balance sheets, cash flow
   - ~90 minute refresh time
   - Status: Fresh/Stale (quarterly updates needed)

3. **🧮 Calculated Ratios**
   - P/E, P/S, Piotroski, O'Shaughnessy ratios
   - ~10 minute refresh time
   - Depends on: Market + Financial data being current

## Frontend Architecture Plan

### 1. Store Layer (`src/stores/dataRefreshStore.ts`)

```typescript
interface DataRefreshStore {
  // Status signals
  freshnessStatus: Signal<SystemFreshnessReport | null>;
  refreshProgress: Signal<RefreshProgress | null>;
  lastRefreshResult: Signal<RefreshResult | null>;
  isRefreshing: Signal<boolean>;

  // Actions
  checkDataFreshness: () => Promise<void>;
  startRefresh: (mode: RefreshMode) => Promise<void>;
  cancelRefresh: () => Promise<void>;
  checkScreeningReadiness: (feature: string) => Promise<boolean>;
}

interface SystemFreshnessReport {
  market_data: DataTypeStatus;
  financial_data: DataTypeStatus;
  calculated_ratios: DataTypeStatus;
  overall_status: 'fresh' | 'stale' | 'critical';
  screening_readiness: {
    garp_screening: boolean;
    graham_screening: boolean;
    piotroski_screening: boolean;
    oshaughnessy_screening: boolean;
    blocking_issues: string[];
  };
}

interface DataTypeStatus {
  is_fresh: boolean;
  last_updated: string;
  hours_since_update: number;
  freshness_threshold_hours: number;
  next_recommended_refresh: string;
}
```

### 2. Component Architecture

#### Main Navigation Integration
- Add "Data Management" tab to existing navigation
- Show status indicator (🟢 Fresh / 🟡 Stale / 🔴 Critical) in nav

#### Core Components

##### `DataStatusPanel.tsx` - Overview Dashboard
```typescript
export default function DataStatusPanel() {
  const status = dataRefreshStore.freshnessStatus();

  return (
    <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
      <DataTypeCard
        title="Market Data"
        status={status?.market_data}
        icon="📈"
        refreshAction={() => startRefresh('market')}
      />
      <DataTypeCard
        title="Financial Data"
        status={status?.financial_data}
        icon="📋"
        refreshAction={() => startRefresh('financials')}
      />
      <DataTypeCard
        title="Calculated Ratios"
        status={status?.calculated_ratios}
        icon="🧮"
        refreshAction={() => startRefresh('ratios')}
      />
    </div>
  );
}
```

##### `RefreshControls.tsx` - Manual Refresh Interface
```typescript
export default function RefreshControls() {
  return (
    <div class="bg-white rounded-lg p-6 shadow-sm border">
      <h3 class="text-lg font-semibold mb-4">Refresh Controls</h3>

      {/* Individual Refreshes */}
      <div class="grid grid-cols-1 sm:grid-cols-3 gap-4 mb-6">
        <RefreshButton
          mode="market"
          title="Market Data"
          duration="~15 min"
          icon="📈"
        />
        <RefreshButton
          mode="financials"
          title="Financial Data"
          duration="~90 min"
          icon="📋"
        />
        <RefreshButton
          mode="ratios"
          title="Calculated Ratios"
          duration="~10 min"
          icon="🧮"
        />
      </div>

      {/* Bulk Operations */}
      <div class="border-t pt-4">
        <h4 class="font-medium mb-3">Bulk Operations</h4>
        <div class="flex gap-3">
          <button class="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-lg">
            ⚡ Quick Refresh (~25 min)
          </button>
          <button class="bg-purple-600 hover:bg-purple-700 text-white px-4 py-2 rounded-lg">
            🔋 Full Refresh (~115 min)
          </button>
        </div>
      </div>
    </div>
  );
}
```

##### `RefreshProgress.tsx` - Real-time Progress Monitoring
```typescript
export default function RefreshProgress() {
  const progress = dataRefreshStore.refreshProgress();

  return (
    <Show when={progress}>
      <div class="bg-blue-50 border border-blue-200 rounded-lg p-6">
        <div class="flex justify-between items-center mb-4">
          <h3 class="font-semibold text-blue-900">
            {progress()?.operation_type} in Progress
          </h3>
          <button
            onClick={() => dataRefreshStore.cancelRefresh()}
            class="text-red-600 hover:text-red-700"
          >
            Cancel
          </button>
        </div>

        <div class="space-y-3">
          <div class="flex justify-between text-sm">
            <span>{progress()?.current_step_name}</span>
            <span>{progress()?.overall_progress_percent.toFixed(1)}%</span>
          </div>

          <div class="w-full bg-blue-200 rounded-full h-2">
            <div
              class="bg-blue-600 h-2 rounded-full transition-all duration-300"
              style={`width: ${progress()?.overall_progress_percent}%`}
            />
          </div>

          <div class="flex justify-between text-xs text-blue-700">
            <span>Elapsed: {progress()?.elapsed_minutes} min</span>
            <span>ETA: {progress()?.estimated_completion}</span>
          </div>
        </div>
      </div>
    </Show>
  );
}
```

##### `ScreeningReadinessIndicator.tsx` - Feature Status
```typescript
export default function ScreeningReadinessIndicator() {
  const status = dataRefreshStore.freshnessStatus();

  return (
    <div class="bg-gray-50 rounded-lg p-4">
      <h4 class="font-medium mb-3">Screening Feature Status</h4>
      <div class="grid grid-cols-2 gap-3">
        <FeatureStatus
          name="GARP Screening"
          ready={status?.screening_readiness.garp_screening}
        />
        <FeatureStatus
          name="Graham Screening"
          ready={status?.screening_readiness.graham_screening}
        />
        <FeatureStatus
          name="Piotroski Screening"
          ready={status?.screening_readiness.piotroski_screening}
        />
        <FeatureStatus
          name="O'Shaughnessy Screening"
          ready={status?.screening_readiness.oshaughnessy_screening}
        />
      </div>
    </div>
  );
}
```

### 3. Integration Points

#### Main Navigation Addition
```typescript
// In src/App.tsx or main navigation component
const navigationTabs = [
  { id: 'stocks', label: 'Stocks', icon: '📊' },
  { id: 'screening', label: 'Screening', icon: '🔍' },
  { id: 'data-management', label: 'Data Management', icon: '🔄' }, // NEW
];
```

#### Proactive Data Checking
```typescript
// In screening components (HeroSection.tsx, etc.)
const handleScreeningAction = async (screeningType: string) => {
  // Check if data is ready before proceeding
  const isReady = await dataRefreshStore.checkScreeningReadiness(screeningType);

  if (!isReady) {
    // Show refresh prompt or auto-refresh
    const shouldRefresh = confirm('Data is stale. Refresh now?');
    if (shouldRefresh) {
      await dataRefreshStore.startRefresh('ratios');
      return;
    }
  }

  // Proceed with screening
  recommendationsStore.setScreeningType(screeningType);
  uiStore.openRecommendations();
};
```

#### Auto-refresh Scheduling
```typescript
// In dataRefreshStore.ts
const setupAutoRefresh = () => {
  // Check data freshness every hour
  setInterval(async () => {
    await checkDataFreshness();

    // Auto-refresh market data if stale (low-cost operation)
    const status = freshnessStatus();
    if (status?.market_data && !status.market_data.is_fresh) {
      console.log('🔄 Auto-refreshing stale market data...');
      await startRefresh('market');
    }
  }, 60 * 60 * 1000); // 1 hour
};
```

## Implementation Plan

### Phase 1: Core Infrastructure (Day 1)
1. ✅ Create `dataRefreshStore.ts` with all API integrations
2. ✅ Add refresh commands to `api.ts` service layer
3. ✅ Create TypeScript interfaces for all data types
4. ✅ Add "Data Management" tab to navigation

### Phase 2: Status Dashboard (Day 1-2)
1. ✅ Implement `DataStatusPanel.tsx` with live status
2. ✅ Create `DataTypeCard.tsx` component for individual data types
3. ✅ Add status indicators throughout the app
4. ✅ Implement auto-refresh status checking

### Phase 3: Refresh Controls (Day 2)
1. ✅ Build `RefreshControls.tsx` with all refresh options
2. ✅ Implement `RefreshButton.tsx` for individual operations
3. ✅ Add bulk refresh operations
4. ✅ Create time estimation display

### Phase 4: Progress Monitoring (Day 2-3)
1. ✅ Implement `RefreshProgress.tsx` with real-time updates
2. ✅ Add progress bars and status indicators
3. ✅ Implement cancel functionality
4. ✅ Add time estimation and ETA display

### Phase 5: Smart Integration (Day 3)
1. ✅ Add proactive data checking to all screening features
2. ✅ Implement auto-refresh for stale market data
3. ✅ Add "data stale" warnings before screening operations
4. ✅ Create smart refresh recommendations

### Phase 6: Polish & Testing (Day 3-4)
1. ✅ Add loading states and error handling
2. ✅ Implement toast notifications for refresh completion
3. ✅ Add keyboard shortcuts for common operations
4. ✅ Comprehensive testing of all refresh scenarios

## Key Benefits

### For Users
- **🎯 Transparency**: Always know the freshness of their data
- **🔄 Control**: Manual refresh capabilities for all data types
- **⚡ Efficiency**: Auto-refresh prevents stale data issues
- **📊 Reliability**: Screening always uses current data

### For System
- **🛡️ Data Quality**: Prevents analysis on stale data
- **⚖️ Load Management**: Intelligent refresh scheduling
- **🔧 Maintainability**: Clean separation of refresh logic
- **📈 Scalability**: Modular architecture for future data sources

## Technical Notes

### Error Handling
- Graceful degradation when refresh operations fail
- Clear error messages for users
- Retry mechanisms for transient failures
- Offline mode detection and handling

### Performance
- Efficient polling for progress updates
- Lazy loading of refresh status
- Debounced refresh triggers
- Background refresh operations

### Security
- User authentication for refresh operations
- Rate limiting for API calls
- Secure storage of refresh tokens
- Audit logging for refresh activities

## Future Enhancements

### Planned Features
- **📅 Scheduled Refreshes**: User-configurable refresh schedules
- **📱 Push Notifications**: Refresh completion notifications
- **📊 Refresh Analytics**: Track refresh patterns and performance
- **🔗 External Triggers**: Webhook-based refresh triggers
- **🎛️ Advanced Controls**: Granular refresh options per data source

This architecture provides a comprehensive, user-friendly data management interface while leveraging the robust backend refresh system already in place.