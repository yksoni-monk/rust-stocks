# API Validation & TypeScript Bindings Implementation Plan

## PROBLEM STATEMENT
**Current Issue**: Unreliable frontend-backend integration with case sensitivity bugs, type mismatches, and broken event systems that waste hours of debugging time.

**Root Cause**: No type safety between Rust backend and TypeScript frontend, leading to runtime errors and data inconsistencies.

## SOLUTION: Full Type Safety with ts-rs

### Phase 1: Install and Configure ts-rs

#### 1.1 Add Dependencies
```toml
# src-tauri/Cargo.toml
[dependencies]
ts-rs = "7.1"
serde = { version = "1.0", features = ["derive"] }

[dev-dependencies]
# For generating bindings during build
```

#### 1.2 Configure Build Script
```rust
// src-tauri/build.rs
fn main() {
    // Generate TypeScript bindings
    std::env::set_var("TS_RS_EXPORT_DIR", "../src/bindings");
    tauri_build::build();
}
```

### Phase 2: Rust Type Definitions with ts-rs

#### 2.1 Core Data Structures
```rust
// src-tauri/src/types.rs
use ts_rs::TS;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RefreshRequestDto {
    pub mode: RefreshMode,
    pub force_sources: Option<Vec<String>>,
    pub initiated_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum RefreshMode {
    #[serde(rename = "market")]
    Market,
    #[serde(rename = "financials")]
    Financials,
    #[serde(rename = "ratios")]
    Ratios,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RefreshProgress {
    pub session_id: String,
    pub operation_type: RefreshMode, // Use enum instead of string
    pub start_time: String,
    pub total_steps: i32,
    pub completed_steps: i32,
    pub current_step_name: Option<String>,
    pub current_step_progress: f64,
    pub overall_progress_percent: f64,
    pub estimated_completion: Option<String>,
    pub status: RefreshStatus,
    pub initiated_by: String,
    pub elapsed_minutes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum RefreshStatus {
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RefreshCompletedEvent {
    pub mode: RefreshMode,
    pub session_id: String,
    pub status: RefreshStatus,
}
```

#### 2.2 Data Freshness Types
```rust
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SystemFreshnessReport {
    pub overall_status: FreshnessLevel,
    pub data_sources: Vec<DataSourceStatus>,
    pub screening_readiness: ScreeningReadiness,
    pub last_checked: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum FreshnessLevel {
    #[serde(rename = "fresh")]
    Fresh,
    #[serde(rename = "stale")]
    Stale,
    #[serde(rename = "critical")]
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DataSourceStatus {
    pub name: String,
    pub freshness: FreshnessLevel,
    pub records_count: i64,
    pub latest_data_date: Option<String>,
    pub last_refresh: Option<String>,
}
```

### Phase 3: Tauri Command Type Safety

#### 3.1 Update All Commands
```rust
// src-tauri/src/commands/data_refresh.rs
use crate::types::*;

#[tauri::command]
pub async fn start_data_refresh(
    app_handle: tauri::AppHandle,
    request: RefreshRequestDto
) -> Result<String, String> {
    // Implementation with guaranteed type safety
}

#[tauri::command]
pub async fn get_refresh_progress(
    session_id: String
) -> Result<Option<RefreshProgress>, String> {
    // Return typed RefreshProgress instead of generic JSON
}

#[tauri::command]
pub async fn get_data_freshness_status() -> Result<SystemFreshnessReport, String> {
    // Return typed freshness report
}
```

#### 3.2 Event Emission with Types
```rust
// Emit typed events
let event = RefreshCompletedEvent {
    mode: RefreshMode::Ratios, // Enum, not string
    session_id: session_id.clone(),
    status: RefreshStatus::Completed,
};

app_handle.emit("refresh-completed", &event)?;
```

### Phase 4: Frontend TypeScript Integration

#### 4.1 Generated Bindings Usage
```typescript
// src/bindings/RefreshRequestDto.ts (auto-generated)
export interface RefreshRequestDto {
    mode: RefreshMode;
    force_sources?: string[] | null;
    initiated_by?: string | null;
}

export type RefreshMode = "market" | "financials" | "ratios";
export type RefreshStatus = "running" | "completed" | "failed" | "cancelled";
```

#### 4.2 API Service Layer with Full Type Safety
```typescript
// src/services/dataRefreshAPI.ts
import type {
    RefreshRequestDto,
    RefreshProgress,
    SystemFreshnessReport,
    RefreshCompletedEvent
} from '../bindings';

export const dataRefreshAPI = {
    async startDataRefresh(request: RefreshRequestDto): Promise<string> {
        return await invoke('start_data_refresh', { request });
    },

    async getRefreshProgress(sessionId: string): Promise<RefreshProgress | null> {
        return await invoke('get_refresh_progress', { sessionId });
    },

    async getDataFreshnessStatus(): Promise<SystemFreshnessReport> {
        return await invoke('get_data_freshness_status');
    }
};
```

#### 4.3 Event Listener with Type Safety
```typescript
// src/stores/dataRefreshStore.ts
import { listen } from '@tauri-apps/api/event';
import type { RefreshCompletedEvent } from '../bindings';

listen<RefreshCompletedEvent>('refresh-completed', (event) => {
    const { mode, status, session_id } = event.payload;
    // TypeScript ensures correct property access
    console.log(`Refresh completed: ${mode} -> ${status}`);

    // Remove from refreshing set - guaranteed type safety
    setRefreshingCards(prev => {
        const newSet = new Set(prev);
        newSet.delete(mode); // mode is guaranteed to be lowercase enum value
        return newSet;
    });
});
```

### Phase 5: Build Integration

#### 5.1 Automatic Binding Generation
```json
// package.json
{
  "scripts": {
    "build:bindings": "cd src-tauri && cargo test --lib generate_bindings",
    "dev": "npm run build:bindings && vite",
    "build": "npm run build:bindings && vite build"
  }
}
```

#### 5.2 Cargo Test for Binding Generation
```rust
// src-tauri/src/lib.rs
#[cfg(test)]
mod tests {
    use super::types::*;

    #[test]
    fn generate_bindings() {
        RefreshRequestDto::export().unwrap();
        RefreshProgress::export().unwrap();
        SystemFreshnessReport::export().unwrap();
        RefreshCompletedEvent::export().unwrap();
        // Export all types
    }
}
```

### Phase 6: Validation and Error Handling

#### 6.1 Runtime Validation
```rust
// src-tauri/src/validation.rs
use crate::types::*;

impl RefreshRequestDto {
    pub fn validate(&self) -> Result<(), String> {
        match self.mode {
            RefreshMode::Market | RefreshMode::Financials | RefreshMode::Ratios => Ok(()),
        }
    }
}
```

#### 6.2 Frontend Validation
```typescript
// src/utils/validation.ts
import type { RefreshRequestDto } from '../bindings';

export function validateRefreshRequest(request: RefreshRequestDto): boolean {
    return ['market', 'financials', 'ratios'].includes(request.mode);
}
```

## IMPLEMENTATION TIMELINE

### Week 1: Foundation
- [ ] Add ts-rs dependencies
- [ ] Create core type definitions
- [ ] Set up binding generation

### Week 2: Backend Migration
- [ ] Update all Tauri commands with typed parameters
- [ ] Replace string enums with proper Rust enums
- [ ] Fix event emission with typed payloads

### Week 3: Frontend Integration
- [ ] Replace manual types with generated bindings
- [ ] Update API service layer
- [ ] Fix event listeners with proper types

### Week 4: Testing & Validation
- [ ] Add runtime validation
- [ ] Test all API endpoints
- [ ] Verify event system works correctly

## SUCCESS CRITERIA

1. **Zero Runtime Type Errors**: All frontend-backend communication uses generated types
2. **Compile-Time Safety**: TypeScript catches API mismatches during build
3. **Reliable Events**: Event system works consistently across all refresh modes
4. **Maintainable Code**: Single source of truth for API contracts
5. **No More Case Sensitivity Bugs**: Enums handle serialization correctly

## ROLLBACK PLAN

If ts-rs integration fails:
1. Remove current manual TypeScript types
2. Add comprehensive runtime validation
3. Use JSON schema validation for API contracts
4. Implement thorough integration tests

---

**This plan eliminates the root cause of API inconsistencies and provides compile-time guarantees that prevent the type of bugs we've been debugging.**