# Tempo Worklog Sync UI — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add UI on the Worklog page to sync work records to Tempo/Jira, with per-project and per-day batch sync, persistent issue key mapping, and sync status display.

**Architecture:** Two new SQLite tables (`project_issue_mappings`, `worklog_sync_records`) store persistent state. Three new Tauri commands expose CRUD for those tables. The frontend adds a `useTempoSync` hook, two modal components (single + batch), and modifies existing `ProjectCard`/`DaySection`/`ManualItemCard` to show sync buttons and status.

**Tech Stack:** Rust (sqlx, Tauri commands), TypeScript/React, shadcn/ui (Dialog, Input, Label, Badge, Button)

**Design doc:** `docs/plans/2026-01-27-tempo-worklog-sync-design.md`

---

## Task 1: Database migrations — two new tables

**Files:**
- Modify: `web/crates/recap-core/src/db/mod.rs:486` (before `log::info!("Database migrations completed")`)

**Step 1: Add project_issue_mappings table migration**

Add after the `llm_usage_logs` index (line ~484), before `log::info!`:

```rust
        // Create project_issue_mappings table for Tempo sync
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS project_issue_mappings (
                project_path TEXT NOT NULL,
                user_id TEXT NOT NULL,
                jira_issue_key TEXT NOT NULL,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (project_path, user_id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
```

**Step 2: Add worklog_sync_records table migration**

```rust
        // Create worklog_sync_records table for tracking Tempo sync status
        sqlx::query(
            r#"
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
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_sync_records_user_date ON worklog_sync_records(user_id, date)")
            .execute(&self.pool)
            .await?;
```

**Step 3: Verify compilation**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && cargo check -p recap-core 2>&1 | tail -5`
Expected: `Finished` with no errors

**Step 4: Commit**

```bash
git add web/crates/recap-core/src/db/mod.rs
git commit -m "feat(db): add project_issue_mappings and worklog_sync_records tables"
```

---

## Task 2: Backend — worklog_sync Tauri commands

**Files:**
- Create: `web/src-tauri/src/commands/worklog_sync.rs`
- Modify: `web/src-tauri/src/commands/mod.rs:21` (add `pub mod worklog_sync;`)
- Modify: `web/src-tauri/src/lib.rs:116-119` (register new commands)

**Step 1: Create `worklog_sync.rs` with types and 3 commands**

Create `web/src-tauri/src/commands/worklog_sync.rs`:

```rust
//! Worklog Sync commands
//!
//! Tauri commands for managing project-to-issue mappings and worklog sync records.

use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use recap_core::auth::verify_token;

use super::AppState;

// Types

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectIssueMapping {
    pub project_path: String,
    pub user_id: String,
    pub jira_issue_key: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorklogSyncRecord {
    pub id: String,
    pub user_id: String,
    pub project_path: String,
    pub date: String,
    pub jira_issue_key: String,
    pub hours: f64,
    pub description: Option<String>,
    pub tempo_worklog_id: Option<String>,
    pub synced_at: String,
}

#[derive(Debug, Deserialize)]
pub struct SaveMappingRequest {
    pub project_path: String,
    pub jira_issue_key: String,
}

#[derive(Debug, Deserialize)]
pub struct GetSyncRecordsRequest {
    pub date_from: String,
    pub date_to: String,
}

#[derive(Debug, Deserialize)]
pub struct SaveSyncRecordRequest {
    pub project_path: String,
    pub date: String,
    pub jira_issue_key: String,
    pub hours: f64,
    pub description: Option<String>,
    pub tempo_worklog_id: Option<String>,
}

// Commands

/// Get all project-to-issue mappings for the current user
#[tauri::command]
pub async fn get_project_issue_mappings(
    state: State<'_, AppState>,
    token: String,
) -> Result<Vec<ProjectIssueMapping>, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let mappings = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT project_path, user_id, jira_issue_key, COALESCE(updated_at, '') FROM project_issue_mappings WHERE user_id = ?"
    )
    .bind(&claims.sub)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(mappings
        .into_iter()
        .map(|(project_path, user_id, jira_issue_key, updated_at)| ProjectIssueMapping {
            project_path,
            user_id,
            jira_issue_key,
            updated_at,
        })
        .collect())
}

/// Save or update a project-to-issue mapping
#[tauri::command]
pub async fn save_project_issue_mapping(
    state: State<'_, AppState>,
    token: String,
    request: SaveMappingRequest,
) -> Result<ProjectIssueMapping, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    sqlx::query(
        r#"
        INSERT INTO project_issue_mappings (project_path, user_id, jira_issue_key, updated_at)
        VALUES (?, ?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(project_path, user_id) DO UPDATE SET
            jira_issue_key = excluded.jira_issue_key,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&request.project_path)
    .bind(&claims.sub)
    .bind(&request.jira_issue_key)
    .execute(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(ProjectIssueMapping {
        project_path: request.project_path,
        user_id: claims.sub,
        jira_issue_key: request.jira_issue_key,
        updated_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// Get worklog sync records for a date range
#[tauri::command]
pub async fn get_worklog_sync_records(
    state: State<'_, AppState>,
    token: String,
    request: GetSyncRecordsRequest,
) -> Result<Vec<WorklogSyncRecord>, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let records = sqlx::query_as::<_, (String, String, String, String, String, f64, Option<String>, Option<String>, String)>(
        r#"
        SELECT id, user_id, project_path, date, jira_issue_key, hours,
               description, tempo_worklog_id, COALESCE(synced_at, '')
        FROM worklog_sync_records
        WHERE user_id = ? AND date >= ? AND date <= ?
        ORDER BY date, project_path
        "#,
    )
    .bind(&claims.sub)
    .bind(&request.date_from)
    .bind(&request.date_to)
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(records
        .into_iter()
        .map(|(id, user_id, project_path, date, jira_issue_key, hours, description, tempo_worklog_id, synced_at)| {
            WorklogSyncRecord {
                id,
                user_id,
                project_path,
                date,
                jira_issue_key,
                hours,
                description,
                tempo_worklog_id,
                synced_at,
            }
        })
        .collect())
}

/// Save a worklog sync record (called after successful Tempo upload)
#[tauri::command]
pub async fn save_worklog_sync_record(
    state: State<'_, AppState>,
    token: String,
    request: SaveSyncRecordRequest,
) -> Result<WorklogSyncRecord, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let id = Uuid::new_v4().to_string();

    sqlx::query(
        r#"
        INSERT INTO worklog_sync_records (id, user_id, project_path, date, jira_issue_key, hours, description, tempo_worklog_id, synced_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(user_id, project_path, date) DO UPDATE SET
            jira_issue_key = excluded.jira_issue_key,
            hours = excluded.hours,
            description = excluded.description,
            tempo_worklog_id = excluded.tempo_worklog_id,
            synced_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&id)
    .bind(&claims.sub)
    .bind(&request.project_path)
    .bind(&request.date)
    .bind(&request.jira_issue_key)
    .bind(&request.hours)
    .bind(&request.description)
    .bind(&request.tempo_worklog_id)
    .execute(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(WorklogSyncRecord {
        id,
        user_id: claims.sub,
        project_path: request.project_path,
        date: request.date,
        jira_issue_key: request.jira_issue_key,
        hours: request.hours,
        description: request.description,
        tempo_worklog_id: request.tempo_worklog_id,
        synced_at: chrono::Utc::now().to_rfc3339(),
    })
}
```

**Step 2: Register module in `mod.rs`**

In `web/src-tauri/src/commands/mod.rs`, add after line 21 (`pub mod work_items;`):

```rust
pub mod worklog_sync;
```

**Step 3: Register commands in `lib.rs`**

In `web/src-tauri/src/lib.rs`, add after line 118 (after `commands::snapshots::get_hourly_breakdown,`):

```rust
            // Worklog Sync
            commands::worklog_sync::get_project_issue_mappings,
            commands::worklog_sync::save_project_issue_mapping,
            commands::worklog_sync::get_worklog_sync_records,
            commands::worklog_sync::save_worklog_sync_record,
```

**Step 4: Verify compilation**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && cargo check 2>&1 | tail -5`
Expected: `Finished` with no errors

**Step 5: Commit**

```bash
git add web/src-tauri/src/commands/worklog_sync.rs web/src-tauri/src/commands/mod.rs web/src-tauri/src/lib.rs
git commit -m "feat(backend): add worklog sync commands for mappings and sync records"
```

---

## Task 3: Frontend types — worklog-sync types

**Files:**
- Create: `web/src/types/worklog-sync.ts`
- Modify: `web/src/types/index.ts:140` (add re-exports)

**Step 1: Create type definitions**

Create `web/src/types/worklog-sync.ts`:

```typescript
/**
 * Types for worklog sync to Tempo/Jira
 */

export interface ProjectIssueMapping {
  project_path: string
  user_id: string
  jira_issue_key: string
  updated_at: string
}

export interface WorklogSyncRecord {
  id: string
  user_id: string
  project_path: string
  date: string
  jira_issue_key: string
  hours: number
  description?: string
  tempo_worklog_id?: string
  synced_at: string
}

export interface SaveMappingRequest {
  project_path: string
  jira_issue_key: string
}

export interface GetSyncRecordsRequest {
  date_from: string
  date_to: string
}

export interface SaveSyncRecordRequest {
  project_path: string
  date: string
  jira_issue_key: string
  hours: number
  description?: string
  tempo_worklog_id?: string
}

/** Data passed to TempoSyncModal for a single project */
export interface TempoSyncTarget {
  projectPath: string
  projectName: string
  date: string
  weekday: string
  hours: number
  description: string
}

/** Data for a row in TempoBatchSyncModal */
export interface BatchSyncRow {
  projectPath: string
  projectName: string
  issueKey: string
  hours: number
  description: string
  isManual: boolean
  /** id of the ManualWorkItem, if applicable */
  manualItemId?: string
}
```

**Step 2: Add re-exports in `types/index.ts`**

Add after line 140 (`} from './worklog'`):

```typescript

// Worklog Sync types
export type {
  ProjectIssueMapping,
  WorklogSyncRecord,
  SaveMappingRequest,
  GetSyncRecordsRequest,
  SaveSyncRecordRequest,
  TempoSyncTarget,
  BatchSyncRow,
} from './worklog-sync'
```

**Step 3: Verify build**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && npx tsc --noEmit 2>&1 | tail -5`
Expected: No errors

**Step 4: Commit**

```bash
git add web/src/types/worklog-sync.ts web/src/types/index.ts
git commit -m "feat(types): add worklog sync types for mappings and sync records"
```

---

## Task 4: Frontend service — worklog-sync API wrapper

**Files:**
- Create: `web/src/services/worklog-sync.ts`
- Modify: `web/src/services/index.ts:27` (add re-export)

**Step 1: Create service**

Create `web/src/services/worklog-sync.ts`:

```typescript
/**
 * Worklog Sync service — project-issue mappings and sync records
 */

import { invokeAuth } from './client'
import type {
  ProjectIssueMapping,
  WorklogSyncRecord,
  SaveMappingRequest,
  GetSyncRecordsRequest,
  SaveSyncRecordRequest,
} from '@/types'

/** Get all project-to-issue mappings for current user */
export async function getMappings(): Promise<ProjectIssueMapping[]> {
  return invokeAuth<ProjectIssueMapping[]>('get_project_issue_mappings')
}

/** Save or update a project-to-issue mapping */
export async function saveMapping(request: SaveMappingRequest): Promise<ProjectIssueMapping> {
  return invokeAuth<ProjectIssueMapping>('save_project_issue_mapping', { request })
}

/** Get worklog sync records for a date range */
export async function getSyncRecords(request: GetSyncRecordsRequest): Promise<WorklogSyncRecord[]> {
  return invokeAuth<WorklogSyncRecord[]>('get_worklog_sync_records', { request })
}

/** Save a sync record after successful Tempo upload */
export async function saveSyncRecord(request: SaveSyncRecordRequest): Promise<WorklogSyncRecord> {
  return invokeAuth<WorklogSyncRecord>('save_worklog_sync_record', { request })
}
```

**Step 2: Add re-export in `services/index.ts`**

Add after line 26 (`export * as worklog from './worklog'`):

```typescript
export * as worklogSync from './worklog-sync'
```

**Step 3: Verify build**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && npx tsc --noEmit 2>&1 | tail -5`
Expected: No errors

**Step 4: Commit**

```bash
git add web/src/services/worklog-sync.ts web/src/services/index.ts
git commit -m "feat(services): add worklog-sync service for mappings and records"
```

---

## Task 5: Frontend hook — useTempoSync

**Files:**
- Create: `web/src/pages/Worklog/hooks/useTempoSync.ts`
- Modify: `web/src/pages/Worklog/hooks/index.ts` (add re-export)

**Step 1: Create hook**

Create `web/src/pages/Worklog/hooks/useTempoSync.ts`:

```typescript
import { useEffect, useState, useCallback } from 'react'
import { worklogSync, tempo } from '@/services'
import type {
  ProjectIssueMapping,
  WorklogSyncRecord,
  TempoSyncTarget,
  BatchSyncRow,
  WorklogDay,
  SyncWorklogsResponse,
} from '@/types'

export function useTempoSync(
  isAuthenticated: boolean,
  startDate: string,
  endDate: string,
  days: WorklogDay[],
  onSyncComplete: () => void,
) {
  // Persistent mappings: projectPath → issueKey
  const [mappings, setMappings] = useState<Record<string, string>>({})
  // Sync records for current date range
  const [syncRecords, setSyncRecords] = useState<WorklogSyncRecord[]>([])
  // Modal state
  const [syncTarget, setSyncTarget] = useState<TempoSyncTarget | null>(null)
  const [batchSyncDate, setBatchSyncDate] = useState<string | null>(null)
  const [batchSyncWeekday, setBatchSyncWeekday] = useState<string>('')
  // Loading / result
  const [syncing, setSyncing] = useState(false)
  const [syncResult, setSyncResult] = useState<SyncWorklogsResponse | null>(null)

  // ---- Load mappings and sync records ----

  const loadMappings = useCallback(async () => {
    try {
      const list = await worklogSync.getMappings()
      const map: Record<string, string> = {}
      for (const m of list) {
        map[m.project_path] = m.jira_issue_key
      }
      setMappings(map)
    } catch {
      // ignore — Jira may not be configured
    }
  }, [])

  const loadSyncRecords = useCallback(async () => {
    try {
      const records = await worklogSync.getSyncRecords({
        date_from: startDate,
        date_to: endDate,
      })
      setSyncRecords(records)
    } catch {
      setSyncRecords([])
    }
  }, [startDate, endDate])

  useEffect(() => {
    if (!isAuthenticated) return
    loadMappings()
    loadSyncRecords()
  }, [isAuthenticated, loadMappings, loadSyncRecords])

  // ---- Lookup helpers ----

  /** Get sync record for a project + date */
  const getSyncRecord = useCallback(
    (projectPath: string, date: string): WorklogSyncRecord | undefined => {
      return syncRecords.find(
        (r) => r.project_path === projectPath && r.date === date,
      )
    },
    [syncRecords],
  )

  /** Get saved issue key for a project path */
  const getMappedIssueKey = useCallback(
    (projectPath: string): string => {
      return mappings[projectPath] ?? ''
    },
    [mappings],
  )

  // ---- Single project sync modal ----

  const openSyncModal = useCallback(
    (target: TempoSyncTarget) => {
      setSyncTarget(target)
      setSyncResult(null)
    },
    [],
  )

  const closeSyncModal = useCallback(() => {
    setSyncTarget(null)
    setSyncResult(null)
  }, [])

  const executeSingleSync = useCallback(
    async (issueKey: string, hours: number, description: string, dryRun: boolean) => {
      if (!syncTarget) return null
      setSyncing(true)
      setSyncResult(null)
      try {
        const minutes = Math.round(hours * 60)
        const result = await tempo.syncWorklogs({
          entries: [
            {
              issue_key: issueKey,
              date: syncTarget.date,
              minutes,
              description,
            },
          ],
          dry_run: dryRun,
        })
        setSyncResult(result)

        if (!dryRun && result.success) {
          // Save mapping
          await worklogSync.saveMapping({
            project_path: syncTarget.projectPath,
            jira_issue_key: issueKey,
          })
          // Save sync record
          const tempoWorklogId = result.results[0]?.id ?? undefined
          await worklogSync.saveSyncRecord({
            project_path: syncTarget.projectPath,
            date: syncTarget.date,
            jira_issue_key: issueKey,
            hours,
            description,
            tempo_worklog_id: tempoWorklogId,
          })
          // Refresh
          await loadMappings()
          await loadSyncRecords()
          onSyncComplete()
        }
        return result
      } catch (err) {
        console.error('Tempo sync failed:', err)
        return null
      } finally {
        setSyncing(false)
      }
    },
    [syncTarget, loadMappings, loadSyncRecords, onSyncComplete],
  )

  // ---- Batch sync modal ----

  const openBatchSyncModal = useCallback(
    (date: string, weekday: string) => {
      setBatchSyncDate(date)
      setBatchSyncWeekday(weekday)
      setSyncResult(null)
    },
    [],
  )

  const closeBatchSyncModal = useCallback(() => {
    setBatchSyncDate(null)
    setBatchSyncWeekday('')
    setSyncResult(null)
  }, [])

  const executeBatchSync = useCallback(
    async (rows: BatchSyncRow[], dryRun: boolean) => {
      if (!batchSyncDate) return null
      setSyncing(true)
      setSyncResult(null)
      try {
        const entries = rows
          .filter((r) => r.issueKey.trim() !== '')
          .map((r) => ({
            issue_key: r.issueKey.trim(),
            date: batchSyncDate,
            minutes: Math.round(r.hours * 60),
            description: r.description,
          }))

        if (entries.length === 0) return null

        const result = await tempo.syncWorklogs({
          entries,
          dry_run: dryRun,
        })
        setSyncResult(result)

        if (!dryRun && result.success) {
          // Save mappings and sync records for each successful entry
          for (let i = 0; i < rows.length; i++) {
            const row = rows[i]
            const entryResult = result.results[i]
            if (!row.issueKey.trim() || entryResult?.status !== 'success') continue

            await worklogSync.saveMapping({
              project_path: row.projectPath,
              jira_issue_key: row.issueKey.trim(),
            })
            await worklogSync.saveSyncRecord({
              project_path: row.projectPath,
              date: batchSyncDate,
              jira_issue_key: row.issueKey.trim(),
              hours: row.hours,
              description: row.description,
              tempo_worklog_id: entryResult.id ?? undefined,
            })
          }
          await loadMappings()
          await loadSyncRecords()
          onSyncComplete()
        }
        return result
      } catch (err) {
        console.error('Batch sync failed:', err)
        return null
      } finally {
        setSyncing(false)
      }
    },
    [batchSyncDate, loadMappings, loadSyncRecords, onSyncComplete],
  )

  return {
    // State
    mappings,
    syncRecords,
    syncing,
    syncResult,
    // Lookups
    getSyncRecord,
    getMappedIssueKey,
    // Single sync modal
    syncTarget,
    openSyncModal,
    closeSyncModal,
    executeSingleSync,
    // Batch sync modal
    batchSyncDate,
    batchSyncWeekday,
    openBatchSyncModal,
    closeBatchSyncModal,
    executeBatchSync,
  }
}
```

**Step 2: Update hooks/index.ts**

Replace content of `web/src/pages/Worklog/hooks/index.ts`:

```typescript
export * from './useWorklog'
export * from './useTempoSync'
```

**Step 3: Verify build**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && npx tsc --noEmit 2>&1 | tail -5`
Expected: No errors

**Step 4: Commit**

```bash
git add web/src/pages/Worklog/hooks/useTempoSync.ts web/src/pages/Worklog/hooks/index.ts
git commit -m "feat(hook): add useTempoSync hook for sync state and operations"
```

---

## Task 6: TempoSyncModal — single project sync dialog

**Files:**
- Create: `web/src/pages/Worklog/components/TempoSyncModal.tsx`
- Modify: `web/src/pages/Worklog/components/index.ts` (add re-export)

**Step 1: Create TempoSyncModal**

Create `web/src/pages/Worklog/components/TempoSyncModal.tsx`:

```tsx
import { useState, useEffect, useCallback } from 'react'
import { Check, AlertCircle, Loader2 } from 'lucide-react'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { tempo } from '@/services'
import type { TempoSyncTarget, SyncWorklogsResponse } from '@/types'

interface TempoSyncModalProps {
  target: TempoSyncTarget | null
  defaultIssueKey: string
  syncing: boolean
  syncResult: SyncWorklogsResponse | null
  onSync: (issueKey: string, hours: number, description: string, dryRun: boolean) => Promise<SyncWorklogsResponse | null>
  onClose: () => void
}

export function TempoSyncModal({
  target,
  defaultIssueKey,
  syncing,
  syncResult,
  onSync,
  onClose,
}: TempoSyncModalProps) {
  const [issueKey, setIssueKey] = useState('')
  const [hours, setHours] = useState(0)
  const [description, setDescription] = useState('')
  const [validating, setValidating] = useState(false)
  const [issueValid, setIssueValid] = useState<boolean | null>(null)
  const [issueSummary, setIssueSummary] = useState('')

  // Initialize form when target changes
  useEffect(() => {
    if (target) {
      setIssueKey(defaultIssueKey)
      setHours(target.hours)
      setDescription(target.description)
      setIssueValid(null)
      setIssueSummary('')
    }
  }, [target, defaultIssueKey])

  // Validate issue key on blur
  const validateIssue = useCallback(async () => {
    const key = issueKey.trim()
    if (!key) {
      setIssueValid(null)
      setIssueSummary('')
      return
    }
    setValidating(true)
    try {
      const result = await tempo.validateIssue(key)
      setIssueValid(result.valid)
      setIssueSummary(result.valid ? (result.summary ?? '') : result.message)
    } catch {
      setIssueValid(false)
      setIssueSummary('Validation failed')
    } finally {
      setValidating(false)
    }
  }, [issueKey])

  const handlePreview = () => onSync(issueKey.trim(), hours, description, true)
  const handleSync = () => onSync(issueKey.trim(), hours, description, false)

  const canSync = issueKey.trim() !== '' && hours > 0 && !syncing
  const showResult = syncResult !== null

  if (!target) return null

  return (
    <Dialog open={!!target} onOpenChange={(open) => { if (!open) onClose() }}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Sync to Tempo</DialogTitle>
        </DialogHeader>

        <div className="space-y-4 py-2">
          {/* Project info */}
          <div className="flex gap-4 text-sm">
            <div>
              <span className="text-muted-foreground">Project: </span>
              <span className="font-medium">{target.projectName}</span>
            </div>
            <div>
              <span className="text-muted-foreground">Date: </span>
              <span className="font-medium">{target.date} ({target.weekday})</span>
            </div>
          </div>

          {/* Issue Key */}
          <div className="space-y-1.5">
            <Label htmlFor="issue-key">Issue Key</Label>
            <div className="flex items-center gap-2">
              <Input
                id="issue-key"
                value={issueKey}
                onChange={(e) => { setIssueKey(e.target.value); setIssueValid(null) }}
                onBlur={validateIssue}
                placeholder="e.g. PROJ-123"
                className="flex-1"
              />
              {validating && <Loader2 className="w-4 h-4 animate-spin text-muted-foreground" />}
              {issueValid === true && <Check className="w-4 h-4 text-green-600" />}
              {issueValid === false && <AlertCircle className="w-4 h-4 text-destructive" />}
            </div>
            {issueSummary && (
              <p className={`text-xs ${issueValid ? 'text-muted-foreground' : 'text-destructive'}`}>
                {issueSummary}
              </p>
            )}
          </div>

          {/* Hours */}
          <div className="space-y-1.5">
            <Label htmlFor="hours">Hours</Label>
            <Input
              id="hours"
              type="number"
              step="0.25"
              min="0"
              value={hours}
              onChange={(e) => setHours(parseFloat(e.target.value) || 0)}
            />
          </div>

          {/* Description */}
          <div className="space-y-1.5">
            <Label htmlFor="description">Description</Label>
            <textarea
              id="description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              rows={3}
              className="flex w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
            />
          </div>

          {/* Result */}
          {showResult && (
            <div className={`rounded-md p-3 text-sm ${
              syncResult.dry_run
                ? 'bg-blue-50 text-blue-800 border border-blue-200'
                : syncResult.success
                  ? 'bg-green-50 text-green-800 border border-green-200'
                  : 'bg-red-50 text-red-800 border border-red-200'
            }`}>
              {syncResult.dry_run ? (
                <p>Preview: {syncResult.total_entries} entry ready to sync ({hours}h to {issueKey})</p>
              ) : syncResult.success ? (
                <p>Synced successfully! {syncResult.successful} worklog uploaded.</p>
              ) : (
                <p>Failed: {syncResult.results[0]?.error_message ?? 'Unknown error'}</p>
              )}
            </div>
          )}
        </div>

        <DialogFooter className="gap-2">
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="outline" onClick={handlePreview} disabled={!canSync}>
            {syncing ? <Loader2 className="w-4 h-4 animate-spin mr-2" /> : null}
            Preview
          </Button>
          <Button onClick={handleSync} disabled={!canSync}>
            {syncing ? <Loader2 className="w-4 h-4 animate-spin mr-2" /> : null}
            Sync
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
```

**Step 2: Add re-export in components/index.ts**

In `web/src/pages/Worklog/components/index.ts`, add:

```typescript
export { TempoSyncModal } from './TempoSyncModal'
```

**Step 3: Verify build**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && npx tsc --noEmit 2>&1 | tail -5`
Expected: No errors

**Step 4: Commit**

```bash
git add web/src/pages/Worklog/components/TempoSyncModal.tsx web/src/pages/Worklog/components/index.ts
git commit -m "feat(ui): add TempoSyncModal for single project sync"
```

---

## Task 7: TempoBatchSyncModal — day-level batch sync dialog

**Files:**
- Create: `web/src/pages/Worklog/components/TempoBatchSyncModal.tsx`
- Modify: `web/src/pages/Worklog/components/index.ts` (add re-export)

**Step 1: Create TempoBatchSyncModal**

Create `web/src/pages/Worklog/components/TempoBatchSyncModal.tsx`:

```tsx
import { useState, useEffect, useCallback } from 'react'
import { Check, AlertCircle, Loader2 } from 'lucide-react'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { tempo } from '@/services'
import type { BatchSyncRow, SyncWorklogsResponse } from '@/types'

interface TempoBatchSyncModalProps {
  open: boolean
  date: string
  weekday: string
  initialRows: BatchSyncRow[]
  syncing: boolean
  syncResult: SyncWorklogsResponse | null
  onSync: (rows: BatchSyncRow[], dryRun: boolean) => Promise<SyncWorklogsResponse | null>
  onClose: () => void
}

type ValidationState = Record<string, { valid: boolean | null; summary: string; loading: boolean }>

export function TempoBatchSyncModal({
  open,
  date,
  weekday,
  initialRows,
  syncing,
  syncResult,
  onSync,
  onClose,
}: TempoBatchSyncModalProps) {
  const [rows, setRows] = useState<BatchSyncRow[]>([])
  const [validation, setValidation] = useState<ValidationState>({})

  // Initialize rows when modal opens
  useEffect(() => {
    if (open) {
      setRows(initialRows)
      setValidation({})
    }
  }, [open, initialRows])

  const updateRow = useCallback((index: number, field: keyof BatchSyncRow, value: string | number) => {
    setRows((prev) =>
      prev.map((r, i) => (i === index ? { ...r, [field]: value } : r)),
    )
    // Clear validation when issue key changes
    if (field === 'issueKey') {
      setValidation((prev) => {
        const next = { ...prev }
        delete next[`${index}`]
        return next
      })
    }
  }, [])

  const validateIssue = useCallback(async (index: number) => {
    const key = rows[index]?.issueKey.trim()
    if (!key) return

    setValidation((prev) => ({
      ...prev,
      [`${index}`]: { valid: null, summary: '', loading: true },
    }))

    try {
      const result = await tempo.validateIssue(key)
      setValidation((prev) => ({
        ...prev,
        [`${index}`]: {
          valid: result.valid,
          summary: result.valid ? (result.summary ?? '') : result.message,
          loading: false,
        },
      }))
    } catch {
      setValidation((prev) => ({
        ...prev,
        [`${index}`]: { valid: false, summary: 'Validation failed', loading: false },
      }))
    }
  }, [rows])

  const totalHours = rows.reduce((sum, r) => sum + r.hours, 0)
  const filledRows = rows.filter((r) => r.issueKey.trim() !== '')
  const canSync = filledRows.length > 0 && !syncing

  const handlePreview = () => onSync(rows, true)
  const handleSync = () => onSync(rows, false)

  const showResult = syncResult !== null

  return (
    <Dialog open={open} onOpenChange={(o) => { if (!o) onClose() }}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>
            Sync Day: {date.slice(5).replace('-', '/')} ({weekday})
          </DialogTitle>
        </DialogHeader>

        <div className="space-y-4 py-2">
          {/* Table */}
          <div className="border rounded-md overflow-hidden">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b bg-muted/50">
                  <th className="text-left px-3 py-2 font-medium">Project</th>
                  <th className="text-left px-3 py-2 font-medium">Issue Key</th>
                  <th className="text-left px-3 py-2 font-medium w-20">Hours</th>
                  <th className="text-left px-3 py-2 font-medium">Description</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((row, i) => {
                  const v = validation[`${i}`]
                  return (
                    <tr key={i} className="border-b last:border-0">
                      <td className="px-3 py-2">
                        <span className="font-medium truncate block max-w-[140px]">
                          {row.isManual ? `(manual) ${row.projectName}` : row.projectName}
                        </span>
                      </td>
                      <td className="px-3 py-2">
                        <div className="flex items-center gap-1">
                          <Input
                            value={row.issueKey}
                            onChange={(e) => updateRow(i, 'issueKey', e.target.value)}
                            onBlur={() => validateIssue(i)}
                            placeholder="PROJ-123"
                            className="h-8 text-xs"
                          />
                          {v?.loading && <Loader2 className="w-3 h-3 animate-spin text-muted-foreground shrink-0" />}
                          {v?.valid === true && <Check className="w-3 h-3 text-green-600 shrink-0" />}
                          {v?.valid === false && <AlertCircle className="w-3 h-3 text-destructive shrink-0" />}
                        </div>
                      </td>
                      <td className="px-3 py-2">
                        <Input
                          type="number"
                          step="0.25"
                          min="0"
                          value={row.hours}
                          onChange={(e) => updateRow(i, 'hours', parseFloat(e.target.value) || 0)}
                          className="h-8 text-xs w-20"
                        />
                      </td>
                      <td className="px-3 py-2">
                        <Input
                          value={row.description}
                          onChange={(e) => updateRow(i, 'description', e.target.value)}
                          className="h-8 text-xs"
                        />
                      </td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
          </div>

          {/* Total */}
          <div className="text-sm text-muted-foreground text-right">
            Total: <span className="font-medium text-foreground">{totalHours.toFixed(1)}h</span>
            {' '}({filledRows.length}/{rows.length} entries with issue keys)
          </div>

          {/* Result */}
          {showResult && (
            <div className={`rounded-md p-3 text-sm ${
              syncResult.dry_run
                ? 'bg-blue-50 text-blue-800 border border-blue-200'
                : syncResult.success
                  ? 'bg-green-50 text-green-800 border border-green-200'
                  : 'bg-red-50 text-red-800 border border-red-200'
            }`}>
              {syncResult.dry_run ? (
                <p>Preview: {syncResult.total_entries} entries ready ({totalHours.toFixed(1)}h total)</p>
              ) : (
                <p>
                  {syncResult.successful} synced, {syncResult.failed} failed
                  {syncResult.failed > 0 && (
                    <> — {syncResult.results.filter(r => r.status === 'error').map(r => `${r.issue_key}: ${r.error_message}`).join(', ')}</>
                  )}
                </p>
              )}
            </div>
          )}
        </div>

        <DialogFooter className="gap-2">
          <Button variant="outline" onClick={onClose}>Cancel</Button>
          <Button variant="outline" onClick={handlePreview} disabled={!canSync}>
            {syncing ? <Loader2 className="w-4 h-4 animate-spin mr-2" /> : null}
            Preview All
          </Button>
          <Button onClick={handleSync} disabled={!canSync}>
            {syncing ? <Loader2 className="w-4 h-4 animate-spin mr-2" /> : null}
            Sync All
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
```

**Step 2: Add re-export**

In `web/src/pages/Worklog/components/index.ts`, add:

```typescript
export { TempoBatchSyncModal } from './TempoBatchSyncModal'
```

**Step 3: Verify build**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && npx tsc --noEmit 2>&1 | tail -5`

**Step 4: Commit**

```bash
git add web/src/pages/Worklog/components/TempoBatchSyncModal.tsx web/src/pages/Worklog/components/index.ts
git commit -m "feat(ui): add TempoBatchSyncModal for day-level batch sync"
```

---

## Task 8: Modify ProjectCard — add sync button and status row

**Files:**
- Modify: `web/src/pages/Worklog/components/ProjectCard.tsx`

**Step 1: Update ProjectCard**

Replace the entire file `web/src/pages/Worklog/components/ProjectCard.tsx`:

```tsx
import { ChevronDown, ChevronRight, GitCommit, FileCode, Upload, RefreshCw, Check } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { WorklogDayProject, HourlyBreakdownItem } from '@/types/worklog'
import type { WorklogSyncRecord } from '@/types'
import { MarkdownSummary } from '@/components/MarkdownSummary'
import { HourlyBreakdown } from './HourlyBreakdown'

interface ProjectCardProps {
  project: WorklogDayProject
  date: string
  isExpanded: boolean
  hourlyData: HourlyBreakdownItem[]
  hourlyLoading: boolean
  onToggleHourly: () => void
  syncRecord?: WorklogSyncRecord
  onSyncToTempo?: () => void
}

export function ProjectCard({
  project,
  isExpanded,
  hourlyData,
  hourlyLoading,
  onToggleHourly,
  syncRecord,
  onSyncToTempo,
}: ProjectCardProps) {
  const hasHourly = project.has_hourly_data
  const isSynced = !!syncRecord

  return (
    <div className="border border-border rounded-lg bg-white/60">
      {/* Card header */}
      <div className="flex items-start">
        <button
          className="flex-1 px-4 py-3 flex items-start gap-3 text-left hover:bg-muted/30 transition-colors rounded-l-lg"
          onClick={hasHourly ? onToggleHourly : undefined}
          disabled={!hasHourly}
        >
          {/* Expand icon */}
          <div className="mt-0.5 text-muted-foreground">
            {hasHourly ? (
              isExpanded ? (
                <ChevronDown className="w-4 h-4" strokeWidth={1.5} />
              ) : (
                <ChevronRight className="w-4 h-4" strokeWidth={1.5} />
              )
            ) : (
              <div className="w-4 h-4" />
            )}
          </div>

          {/* Content */}
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-1">
              <span className="text-sm font-medium text-foreground truncate">
                {project.project_name}
              </span>
            </div>

            {/* Summary */}
            {project.daily_summary && (
              <MarkdownSummary content={project.daily_summary} />
            )}

            {/* Stats row */}
            <div className="flex items-center gap-4 mt-2">
              {project.total_commits > 0 && (
                <span className="flex items-center gap-1 text-xs text-muted-foreground">
                  <GitCommit className="w-3 h-3" strokeWidth={1.5} />
                  {project.total_commits} commits
                </span>
              )}
              {project.total_files > 0 && (
                <span className="flex items-center gap-1 text-xs text-muted-foreground">
                  <FileCode className="w-3 h-3" strokeWidth={1.5} />
                  {project.total_files} files
                </span>
              )}
            </div>

            {/* Sync status row */}
            {isSynced && (
              <div className="flex items-center gap-1.5 mt-2 text-xs text-green-700">
                <Check className="w-3 h-3" strokeWidth={2} />
                <span>
                  Synced to {syncRecord.jira_issue_key} · {syncRecord.hours}h · {syncRecord.synced_at.slice(5, 16).replace('T', ' ')}
                </span>
              </div>
            )}
          </div>
        </button>

        {/* Sync button */}
        {onSyncToTempo && (
          <div className="px-2 py-3">
            <Button
              variant="ghost"
              size="sm"
              className="h-7 text-xs text-muted-foreground hover:text-foreground"
              onClick={(e) => { e.stopPropagation(); onSyncToTempo() }}
            >
              {isSynced ? (
                <><RefreshCw className="w-3 h-3 mr-1" strokeWidth={1.5} />Re-sync</>
              ) : (
                <><Upload className="w-3 h-3 mr-1" strokeWidth={1.5} />Sync</>
              )}
            </Button>
          </div>
        )}
      </div>

      {/* Hourly breakdown (expanded) */}
      {isExpanded && (
        <div className="border-t border-border">
          <HourlyBreakdown items={hourlyData} loading={hourlyLoading} />
        </div>
      )}
    </div>
  )
}
```

**Step 2: Verify build**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && npx tsc --noEmit 2>&1 | tail -5`

**Step 3: Commit**

```bash
git add web/src/pages/Worklog/components/ProjectCard.tsx
git commit -m "feat(ui): add sync button and status row to ProjectCard"
```

---

## Task 9: Modify ManualItemCard — add sync button and status

**Files:**
- Modify: `web/src/pages/Worklog/components/ManualItemCard.tsx`

**Step 1: Update ManualItemCard**

Replace the entire file `web/src/pages/Worklog/components/ManualItemCard.tsx`:

```tsx
import { Pencil, Trash2, Upload, RefreshCw, Check } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { ManualWorkItem } from '@/types/worklog'
import type { WorklogSyncRecord } from '@/types'

interface ManualItemCardProps {
  item: ManualWorkItem
  onEdit: () => void
  onDelete: () => void
  syncRecord?: WorklogSyncRecord
  onSyncToTempo?: () => void
}

export function ManualItemCard({ item, onEdit, onDelete, syncRecord, onSyncToTempo }: ManualItemCardProps) {
  const isSynced = !!syncRecord

  return (
    <div className="group/item border border-border rounded-lg bg-white/60 px-4 py-3 flex items-start gap-3">
      {/* Indicator dot */}
      <div className="mt-1.5 w-2 h-2 rounded-full bg-muted-foreground/30 shrink-0" />

      {/* Content */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-foreground">{item.title}</span>
          {item.hours > 0 && (
            <span className="text-xs text-muted-foreground">{item.hours}h</span>
          )}
        </div>
        {item.description && (
          <p className="text-sm text-muted-foreground mt-0.5 line-clamp-1">{item.description}</p>
        )}
        {/* Sync status row */}
        {isSynced && (
          <div className="flex items-center gap-1.5 mt-1.5 text-xs text-green-700">
            <Check className="w-3 h-3" strokeWidth={2} />
            <span>
              Synced to {syncRecord.jira_issue_key} · {syncRecord.hours}h · {syncRecord.synced_at.slice(5, 16).replace('T', ' ')}
            </span>
          </div>
        )}
      </div>

      {/* Actions */}
      <div className="flex items-center gap-1 opacity-0 group-hover/item:opacity-100 transition-opacity shrink-0">
        {onSyncToTempo && (
          <Button variant="ghost" size="icon" className="h-7 w-7" onClick={onSyncToTempo} title={isSynced ? 'Re-sync to Tempo' : 'Sync to Tempo'}>
            {isSynced ? <RefreshCw className="w-3 h-3" strokeWidth={1.5} /> : <Upload className="w-3 h-3" strokeWidth={1.5} />}
          </Button>
        )}
        <Button variant="ghost" size="icon" className="h-7 w-7" onClick={onEdit}>
          <Pencil className="w-3 h-3" strokeWidth={1.5} />
        </Button>
        <Button variant="ghost" size="icon" className="h-7 w-7 text-destructive" onClick={onDelete}>
          <Trash2 className="w-3 h-3" strokeWidth={1.5} />
        </Button>
      </div>
    </div>
  )
}
```

**Step 2: Verify build**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && npx tsc --noEmit 2>&1 | tail -5`

**Step 3: Commit**

```bash
git add web/src/pages/Worklog/components/ManualItemCard.tsx
git commit -m "feat(ui): add sync button and status to ManualItemCard"
```

---

## Task 10: Modify DaySection — add "Sync Day" button and pass sync props

**Files:**
- Modify: `web/src/pages/Worklog/components/DaySection.tsx`

**Step 1: Update DaySection**

Replace the entire file `web/src/pages/Worklog/components/DaySection.tsx`:

```tsx
import { Plus, Upload } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { WorklogDay } from '@/types/worklog'
import type { HourlyBreakdownItem } from '@/types/worklog'
import type { WorklogSyncRecord, TempoSyncTarget } from '@/types'
import { ProjectCard } from './ProjectCard'
import { ManualItemCard } from './ManualItemCard'

interface DaySectionProps {
  day: WorklogDay
  expandedProject: { date: string; projectPath: string } | null
  hourlyData: HourlyBreakdownItem[]
  hourlyLoading: boolean
  onToggleHourly: (date: string, projectPath: string) => void
  onAddManualItem: (date: string) => void
  onEditManualItem: (id: string) => void
  onDeleteManualItem: (id: string) => void
  getSyncRecord?: (projectPath: string, date: string) => WorklogSyncRecord | undefined
  onSyncProject?: (target: TempoSyncTarget) => void
  onSyncDay?: (date: string, weekday: string) => void
}

export function DaySection({
  day,
  expandedProject,
  hourlyData,
  hourlyLoading,
  onToggleHourly,
  onAddManualItem,
  onEditManualItem,
  onDeleteManualItem,
  getSyncRecord,
  onSyncProject,
  onSyncDay,
}: DaySectionProps) {
  const isEmpty = day.projects.length === 0 && day.manual_items.length === 0

  return (
    <section className="group">
      {/* Day header */}
      <div className="flex items-baseline justify-between mb-4">
        <div className="flex items-baseline gap-3">
          <h2 className="font-display text-lg text-foreground tracking-tight">
            {day.date.slice(5).replace('-', '/')}
          </h2>
          <span className="text-xs text-muted-foreground">{day.weekday}</span>
        </div>
        <div className="flex items-center gap-1">
          {onSyncDay && !isEmpty && (
            <Button
              variant="ghost"
              size="sm"
              className="h-7 text-xs text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity"
              onClick={() => onSyncDay(day.date, day.weekday)}
            >
              <Upload className="w-3 h-3 mr-1" strokeWidth={1.5} />
              Sync Day
            </Button>
          )}
          <Button
            variant="ghost"
            size="sm"
            className="h-7 text-xs text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity"
            onClick={() => onAddManualItem(day.date)}
          >
            <Plus className="w-3 h-3 mr-1" strokeWidth={1.5} />
            Add
          </Button>
        </div>
      </div>

      {/* Content */}
      {isEmpty ? (
        <div className="py-6 text-center">
          <p className="text-sm text-muted-foreground">No records</p>
        </div>
      ) : (
        <div className="space-y-3">
          {/* Project cards */}
          {day.projects.map((project) => {
            const isExpanded =
              expandedProject?.date === day.date &&
              expandedProject?.projectPath === project.project_path
            return (
              <ProjectCard
                key={project.project_path}
                project={project}
                date={day.date}
                isExpanded={isExpanded}
                hourlyData={isExpanded ? hourlyData : []}
                hourlyLoading={isExpanded ? hourlyLoading : false}
                onToggleHourly={() => onToggleHourly(day.date, project.project_path)}
                syncRecord={getSyncRecord?.(project.project_path, day.date)}
                onSyncToTempo={
                  onSyncProject
                    ? () =>
                        onSyncProject({
                          projectPath: project.project_path,
                          projectName: project.project_name,
                          date: day.date,
                          weekday: day.weekday,
                          hours: project.total_hours,
                          description: project.daily_summary ?? '',
                        })
                    : undefined
                }
              />
            )
          })}

          {/* Manual items */}
          {day.manual_items.map((item) => (
            <ManualItemCard
              key={item.id}
              item={item}
              onEdit={() => onEditManualItem(item.id)}
              onDelete={() => onDeleteManualItem(item.id)}
              syncRecord={getSyncRecord?.(`manual:${item.id}`, day.date)}
              onSyncToTempo={
                onSyncProject
                  ? () =>
                      onSyncProject({
                        projectPath: `manual:${item.id}`,
                        projectName: item.title,
                        date: day.date,
                        weekday: day.weekday,
                        hours: item.hours,
                        description: item.description ?? item.title,
                      })
                  : undefined
              }
            />
          ))}
        </div>
      )}

      {/* Divider */}
      <div className="mt-8 h-px bg-border" />
    </section>
  )
}
```

**Step 2: Verify build**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && npx tsc --noEmit 2>&1 | tail -5`

**Step 3: Commit**

```bash
git add web/src/pages/Worklog/components/DaySection.tsx
git commit -m "feat(ui): add Sync Day button and sync props to DaySection"
```

---

## Task 11: Wire everything together in WorklogPage

**Files:**
- Modify: `web/src/pages/Worklog/index.tsx`

**Step 1: Update WorklogPage to use useTempoSync and render modals**

Replace the entire file `web/src/pages/Worklog/index.tsx`:

```tsx
import { useMemo } from 'react'
import { Plus } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useAuth } from '@/lib/auth'
import { useWorklog, useTempoSync } from './hooks'
import { DateRangeBar, DaySection, TempoSyncModal, TempoBatchSyncModal } from './components'
import {
  CreateModal,
  EditModal,
  DeleteModal,
} from '../WorkItems/components/Modals'
import type { BatchSyncRow } from '@/types'

export function WorklogPage() {
  const { isAuthenticated } = useAuth()

  const wl = useWorklog(isAuthenticated)

  const ts = useTempoSync(
    isAuthenticated,
    wl.startDate,
    wl.endDate,
    wl.days,
    wl.fetchOverview,
  )

  // Build batch sync rows for the selected day
  const batchRows: BatchSyncRow[] = useMemo(() => {
    if (!ts.batchSyncDate) return []
    const day = wl.days.find((d) => d.date === ts.batchSyncDate)
    if (!day) return []

    const rows: BatchSyncRow[] = []
    for (const p of day.projects) {
      // Skip already synced projects
      const existing = ts.getSyncRecord(p.project_path, day.date)
      if (existing) continue
      rows.push({
        projectPath: p.project_path,
        projectName: p.project_name,
        issueKey: ts.getMappedIssueKey(p.project_path),
        hours: p.total_hours,
        description: p.daily_summary ?? '',
        isManual: false,
      })
    }
    for (const m of day.manual_items) {
      const existing = ts.getSyncRecord(`manual:${m.id}`, day.date)
      if (existing) continue
      rows.push({
        projectPath: `manual:${m.id}`,
        projectName: m.title,
        issueKey: ts.getMappedIssueKey(`manual:${m.id}`),
        hours: m.hours,
        description: m.description ?? m.title,
        isManual: true,
        manualItemId: m.id,
      })
    }
    return rows
  }, [ts.batchSyncDate, wl.days, ts])

  // Loading state
  if (wl.loading) {
    return (
      <div className="space-y-12">
        <header className="animate-fade-up opacity-0 delay-1">
          <div className="flex items-start justify-between mb-6">
            <div>
              <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
                Record
              </p>
              <h1 className="font-display text-4xl text-foreground tracking-tight">Worklog</h1>
            </div>
          </div>
          <DateRangeBar
            startDate={wl.startDate}
            endDate={wl.endDate}
            isCurrentWeek={wl.isCurrentWeek}
            onPrev={wl.goToPreviousWeek}
            onNext={wl.goToNextWeek}
            onToday={wl.goToThisWeek}
          />
        </header>
        <div className="flex items-center justify-center h-48">
          <div className="w-6 h-6 border border-border border-t-foreground/60 rounded-full animate-spin" />
        </div>
      </div>
    )
  }

  return (
    <div className="space-y-12">
      {/* Header */}
      <header className="animate-fade-up opacity-0 delay-1">
        <div className="flex items-start justify-between mb-6">
          <div>
            <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
              Record
            </p>
            <h1 className="font-display text-4xl text-foreground tracking-tight">Worklog</h1>
          </div>
          <Button onClick={() => wl.openCreateModal()}>
            <Plus className="w-4 h-4 mr-2" strokeWidth={1.5} />
            Add Item
          </Button>
        </div>
        <DateRangeBar
          startDate={wl.startDate}
          endDate={wl.endDate}
          isCurrentWeek={wl.isCurrentWeek}
          onPrev={wl.goToPreviousWeek}
          onNext={wl.goToNextWeek}
          onToday={wl.goToThisWeek}
        />
      </header>

      {/* Day sections */}
      <section className="space-y-8 animate-fade-up opacity-0 delay-2">
        {wl.days.length === 0 ? (
          <div className="py-16 text-center">
            <p className="text-sm text-muted-foreground mb-4">No records this week</p>
            <Button variant="outline" onClick={() => wl.openCreateModal()}>
              <Plus className="w-4 h-4 mr-2" strokeWidth={1.5} />
              Add Manual Item
            </Button>
          </div>
        ) : (
          wl.days.map((day) => (
            <DaySection
              key={day.date}
              day={day}
              expandedProject={wl.expandedProject}
              hourlyData={wl.hourlyData}
              hourlyLoading={wl.hourlyLoading}
              onToggleHourly={wl.toggleHourlyBreakdown}
              onAddManualItem={wl.openCreateModal}
              onEditManualItem={wl.openEditManualItem}
              onDeleteManualItem={wl.confirmDeleteManualItem}
              getSyncRecord={ts.getSyncRecord}
              onSyncProject={ts.openSyncModal}
              onSyncDay={ts.openBatchSyncModal}
            />
          ))
        )}
      </section>

      {/* CRUD Modals — reuse from WorkItems */}
      <CreateModal
        open={wl.showCreateModal}
        onOpenChange={(open) => { if (!open) wl.closeCreateModal() }}
        formData={wl.formData}
        setFormData={wl.setFormData}
        onSubmit={wl.handleCreate}
        onCancel={wl.closeCreateModal}
      />

      <EditModal
        open={wl.showEditModal}
        onOpenChange={(open) => { if (!open) wl.closeEditModal() }}
        formData={wl.formData}
        setFormData={wl.setFormData}
        onSubmit={wl.handleUpdate}
        onCancel={wl.closeEditModal}
      />

      <DeleteModal
        open={wl.showDeleteConfirm}
        onOpenChange={(open) => { if (!open) wl.closeDeleteConfirm() }}
        itemToDelete={wl.itemToDelete}
        onConfirm={wl.handleDelete}
        onCancel={wl.closeDeleteConfirm}
      />

      {/* Tempo Sync Modals */}
      <TempoSyncModal
        target={ts.syncTarget}
        defaultIssueKey={ts.syncTarget ? ts.getMappedIssueKey(ts.syncTarget.projectPath) : ''}
        syncing={ts.syncing}
        syncResult={ts.syncResult}
        onSync={ts.executeSingleSync}
        onClose={ts.closeSyncModal}
      />

      <TempoBatchSyncModal
        open={!!ts.batchSyncDate}
        date={ts.batchSyncDate ?? ''}
        weekday={ts.batchSyncWeekday}
        initialRows={batchRows}
        syncing={ts.syncing}
        syncResult={ts.syncResult}
        onSync={ts.executeBatchSync}
        onClose={ts.closeBatchSyncModal}
      />
    </div>
  )
}
```

**Step 2: Verify build**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && npx tsc --noEmit 2>&1 | tail -5`
Expected: No errors

**Step 3: Commit**

```bash
git add web/src/pages/Worklog/index.tsx
git commit -m "feat(worklog): wire up Tempo sync modals and useTempoSync hook"
```

---

## Task 12: Full build verification

**Step 1: Check Rust compilation**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && cargo check 2>&1 | tail -10`
Expected: `Finished` with no errors

**Step 2: Check TypeScript compilation**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && npx tsc --noEmit 2>&1 | tail -10`
Expected: No errors

**Step 3: Check frontend build**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && npm run build 2>&1 | tail -10`
Expected: Build successful

**Step 4: Run existing tests**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && cargo test -p recap-core 2>&1 | tail -10`
Expected: All tests pass

**Step 5: Run frontend tests if available**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && npx vitest run 2>&1 | tail -10`
Expected: All tests pass (no new tests broken)
