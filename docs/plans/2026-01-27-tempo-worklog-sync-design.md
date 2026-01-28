# Tempo Worklog Sync UI Design

> Date: 2026-01-27

## Overview

Add UI on the Worklog page to sync work records to Tempo/Jira. The backend APIs already exist (`sync_worklogs_to_tempo`, `upload_single_worklog`, `validate_jira_issue`). This design covers the frontend UI and two new backend tables/commands.

## Design Decisions

| Decision | Choice |
|----------|--------|
| Sync granularity | Both per-project and per-day batch |
| Issue mapping | Persistent mapping (project → Jira issue key) |
| Sync trigger UI | Modal dialog (single + batch) |
| Status display | Badge + sync summary row on ProjectCard |
| Day-level sync | Single multi-row dialog |

## Component Architecture

```
Worklog page (existing)
│
├── DaySection (modified)
│   ├── "Sync Day" button in day header
│   ├── ProjectCard (modified)
│   │   ├── Sync status badge
│   │   ├── Sync summary row
│   │   └── "Sync to Tempo" button
│   └── ManualItemCard (modified)
│       └── Same sync button + status
│
├── TempoSyncModal (NEW)
│   ├── Project info (name, date, summary)
│   ├── Issue key input + validation
│   ├── Hours input (pre-filled from total_hours)
│   ├── Description textarea (pre-filled from daily_summary)
│   ├── Dry-run preview
│   └── Sync / Cancel buttons
│
├── TempoBatchSyncModal (NEW)
│   ├── Day header (date)
│   ├── Multi-row table of unsynced items
│   │   └── Each row: project name, issue key, hours, description
│   ├── Batch dry-run preview
│   └── Sync All / Cancel buttons
│
└── hooks/useTempoSync.ts (NEW)
    ├── Project-to-issue persistent mapping (load/save)
    ├── Single sync logic (validate → dry-run → sync)
    ├── Batch sync logic
    └── Sync status tracking
```

## Data Flow

### Single Project Sync

```
User clicks "Sync to Tempo" on ProjectCard
  → TempoSyncModal opens
  → Auto-fills issue key from persistent mapping (if exists)
  → Auto-fills hours from project.total_hours
  → Auto-fills description from project.daily_summary (plain text)
  → User adjusts fields as needed
  → User clicks "Preview" → calls sync_worklogs_to_tempo with dry_run: true
  → Shows preview result (success/validation errors)
  → User clicks "Confirm Sync" → calls sync_worklogs_to_tempo with dry_run: false
  → On success: updates UI with sync status badge + summary row
  → Saves project_path → issue_key mapping for future use
```

### Day Batch Sync

Same as single, but collects entries from all unsynced projects for the day, calls `sync_worklogs_to_tempo` with the full array, and updates all at once.

### Persistent Mapping

Stored in new `project_issue_mappings` table. When a user syncs project X to PROJ-123, the mapping is saved. Next time, the issue key auto-fills.

### Sync Status

Stored in new `worklog_sync_records` table. ProjectCard reads from this to show badge and summary row.

## UI Design

### ProjectCard (modified)

```
┌─────────────────────────────────────────────────┐
│ ▶ recap-core                        [Sync ↗]   │
│   Fixed SQLite db lock and worklog hours...      │
│   🔀 3 commits · 📄 5 files                     │
│   ✓ Synced to PROJ-123 · 2.5h · 01/27 15:30    │
└─────────────────────────────────────────────────┘
```

- Unsynced: subtle "Sync to Tempo" button on right side of header
- Synced: button changes to "Re-sync", summary row appears below stats

### TempoSyncModal

```
┌─────────────── Sync to Tempo ───────────────────┐
│                                                  │
│  Project:  recap-core                            │
│  Date:     2026-01-27 (Mon)                      │
│                                                  │
│  Issue Key  [PROJ-123    ] [✓ Valid]              │
│  Hours      [2.5        ]                        │
│  Description                                     │
│  [Fixed SQLite db lock and worklog hours     ]   │
│  [                                           ]   │
│                                                  │
│             [Cancel]  [Preview]  [Sync]           │
└──────────────────────────────────────────────────┘
```

### TempoBatchSyncModal

```
┌──────────── Sync Day: 01/27 (Mon) ──────────────┐
│                                                  │
│  Project         Issue Key     Hours  Description│
│  ─────────────────────────────────────────────── │
│  recap-core      [PROJ-123]    [2.5]  [Fixed...] │
│  recap-cli       [PROJ-456]    [1.0]  [Added...] │
│  (manual) Review [         ]   [0.5]  [Code rev] │
│                                                  │
│  Total: 4.0h                                     │
│                                                  │
│             [Cancel]  [Preview All]  [Sync All]  │
└──────────────────────────────────────────────────┘
```

Issue key fields auto-fill from persistent mappings. Validation runs on blur. Rows with invalid/empty issue keys are highlighted and blocked from sync.

## Backend Changes

### New Table: project_issue_mappings

```sql
CREATE TABLE IF NOT EXISTS project_issue_mappings (
    project_path TEXT NOT NULL,
    user_id TEXT NOT NULL,
    jira_issue_key TEXT NOT NULL,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (project_path, user_id)
);
```

### New Table: worklog_sync_records

```sql
CREATE TABLE IF NOT EXISTS worklog_sync_records (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    project_path TEXT NOT NULL,
    date TEXT NOT NULL,
    jira_issue_key TEXT NOT NULL,
    hours REAL NOT NULL,
    description TEXT,
    tempo_worklog_id TEXT,
    synced_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, project_path, date)
);
```

UNIQUE constraint on `(user_id, project_path, date)` enables upsert on re-sync.

### New Tauri Commands (3)

| Command | Input | Output | Purpose |
|---------|-------|--------|---------|
| `get_project_issue_mappings` | token | `Vec<ProjectIssueMapping>` | Load all mappings for current user |
| `save_project_issue_mapping` | token, project_path, jira_issue_key | `ProjectIssueMapping` | Save/update a single mapping |
| `get_worklog_sync_records` | token, date_from, date_to | `Vec<WorklogSyncRecord>` | Load sync records for date range |

No new sync commands — existing `sync_worklogs_to_tempo` handles the upload. After success, frontend calls `save_project_issue_mapping` and backend inserts into `worklog_sync_records`.

## New Files

### Backend (Rust)
- `src-tauri/src/commands/worklog_sync.rs` — 3 new Tauri commands
- DB migration in `recap-core/src/db/mod.rs` — 2 new tables

### Frontend (TypeScript)
- `src/pages/Worklog/components/TempoSyncModal.tsx`
- `src/pages/Worklog/components/TempoBatchSyncModal.tsx`
- `src/pages/Worklog/components/SyncStatusBadge.tsx`
- `src/pages/Worklog/hooks/useTempoSync.ts`
- `src/services/worklog-sync.ts` — Tauri command wrappers
- `src/types/worklog-sync.ts` — Types for new commands

### Modified Files
- `src/pages/Worklog/components/ProjectCard.tsx` — Add sync button + status row
- `src/pages/Worklog/components/ManualItemCard.tsx` — Add sync button + status
- `src/pages/Worklog/components/DaySection.tsx` — Add "Sync Day" button
- `src/pages/Worklog/index.tsx` — Wire up modals and useTempoSync hook
- `src-tauri/src/lib.rs` — Register new commands
