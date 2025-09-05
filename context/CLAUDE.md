# Claude Code Context & Session References

This file tracks ongoing development context and session information for Claude Code assistance.

## Current Active Context

**Session**: Frontend UX Restructuring - Expandable Panels Implementation  
**Date**: 2025-01-05  
**Context File**: [FRONTEND_EXPANDABLE_PANELS_CONTEXT.md](./FRONTEND_EXPANDABLE_PANELS_CONTEXT.md)

### Quick Summary
- **Goal**: Transform tab-based navigation to expandable panels system
- **Principle**: User-driven analysis, no artificial "basic vs enhanced" tiers  
- **Architecture**: Single page with contextual expandable stock rows
- **Status**: Ready to implement Phase 1 - Core panel system

### Current Todo Progress
- ✅ Analyzed current frontend structure
- ✅ Designed expandable panels system  
- ✅ Updated UX improvement plan document
- ✅ Created context documentation
- 🔄 **Next**: Create base ExpandablePanel component

## Previous Sessions

*Future sessions and context files will be referenced here*

## Key Project Files

### Documentation
- `context/ARCHITECTURE.md` - Overall system architecture (Phase 3 current)
- `context/FRONTEND_UX_IMPROVEMENT_PLAN.md` - Detailed expandable panels design
- `context/FRONTEND_EXPANDABLE_PANELS_CONTEXT.md` - Current implementation context

### Frontend Structure  
- `frontend/src/App.jsx` - Main app (needs refactor to expandable panels)
- `frontend/src/components/EnhancedStockDetails.jsx` - Stock analysis (adapt content)
- `frontend/src/components/EnhancedDataFetching.jsx` - Data fetching (adapt logic)

### Backend (Stable)
- `src-tauri/src/` - Rust backend with Schwab API integration
- Database schema enhanced for comprehensive fundamental data

## Development Commands

```bash
# Frontend development
cd frontend && npm run dev

# Backend development  
cd src-tauri && cargo run

# Build application
npm run tauri build
```

---
**Maintained by**: Claude Code sessions  
**Last Updated**: 2025-01-05