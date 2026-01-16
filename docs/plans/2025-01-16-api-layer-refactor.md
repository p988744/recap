# API Layer Refactoring Plan

**Date:** 2025-01-16
**Status:** In Progress
**Risk Level:** Low (mainly type reorganization, no logic changes)

## Problem Statement

Current issues with `tauri-api.ts` (1116 lines) + `api.ts` (1093 lines):
- Duplicate type definitions across both files
- Mixed responsibilities (types + API calls + utilities)
- Hard to locate specific functionality
- HTTP fallback code no longer needed (Tauri-only)

## Target Architecture

```
src/
├── types/                    # Shared type definitions (single source)
│   ├── index.ts             # Re-export all types
│   ├── auth.ts              # UserResponse, TokenResponse, etc.
│   ├── config.ts            # ConfigResponse, UpdateConfigRequest, etc.
│   ├── work-items.ts        # WorkItem, WorkItemFilters, etc.
│   ├── reports.ts           # PersonalReport, TempoReport, etc.
│   ├── sync.ts              # SyncStatus, SyncResult, etc.
│   └── integrations.ts      # GitLab, Tempo, Claude types
│
├── services/                 # API functions (replace tauri-api.ts + api.ts)
│   ├── index.ts             # Export unified `api` object
│   ├── client.ts            # invoke wrapper + token management
│   ├── auth.ts              # login, register, getCurrentUser
│   ├── config.ts            # getConfig, updateConfig, etc.
│   ├── work-items.ts        # CRUD + stats + timeline
│   ├── reports.ts           # report generation + export
│   ├── sync.ts              # sync operations
│   └── integrations.ts      # GitLab, Tempo, Claude APIs
│
├── lib/
│   ├── api.ts               # DEPRECATED: re-export from services/ for compatibility
│   └── tauri-api.ts         # DEPRECATED: re-export from services/ for compatibility
```

## Migration Steps

### Phase 1: Create types/ directory
1. Create `src/types/` directory structure
2. Extract and deduplicate types from both files
3. Organize by domain (auth, config, work-items, etc.)

### Phase 2: Create services/ directory
1. Create `src/services/` directory structure
2. Create `client.ts` with invoke wrapper and token management
3. Migrate API functions by domain

### Phase 3: Update imports
1. Update all component imports to use new paths
2. Keep old files as re-exports for compatibility
3. Verify build passes

### Phase 4: Cleanup
1. Remove deprecated re-export files (optional, can keep for compatibility)
2. Update CLAUDE.md if needed

## Type Mapping

| Original File | Types | Target File |
|---------------|-------|-------------|
| tauri-api.ts | UserResponse, AppStatus, TokenResponse, RegisterRequest, LoginRequest | types/auth.ts |
| tauri-api.ts | ConfigResponse, UpdateConfigRequest, UpdateLlmConfigRequest, UpdateJiraConfigRequest | types/config.ts |
| tauri-api.ts | WorkItem, WorkItemFilters, CreateWorkItemRequest, UpdateWorkItemRequest, WorkItemStatsResponse | types/work-items.ts |
| tauri-api.ts | PersonalReport, CategoryReport, TempoReport, ExportResult | types/reports.ts |
| tauri-api.ts | SyncStatus, SyncResult, AutoSyncResponse | types/sync.ts |
| tauri-api.ts | GitLabProject, TempoWorklogEntry, ClaudeSession | types/integrations.ts |
| api.ts | (remove duplicates, keep only unique types) | types/*.ts |

## Success Criteria

- [ ] All types defined once in `types/` directory
- [ ] All API functions organized in `services/` directory
- [ ] No duplicate type definitions
- [ ] `npm run build` passes
- [ ] `npm run typecheck` passes
- [ ] Each file under 300 lines

## Rollback Plan

If issues arise:
1. Revert to previous commit
2. Old files remain functional as they are being refactored incrementally
