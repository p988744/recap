# Projects Page Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a full-featured Projects page with master-detail layout, project descriptions, timeline view, and AI-powered summaries.

**Architecture:** Left-right split layout (ProjectList | ProjectDetail). Backend uses existing Tauri commands where possible, adds 4 new command modules for descriptions, summaries, timeline, and git diff. Database adds 2 new tables to recap-core.

**Tech Stack:** Rust/Tauri (backend), React/TypeScript (frontend), SQLite, TailwindCSS, shadcn/ui

---

## Phase 1: Database & Types

### Task 1: Add project_descriptions table

**Files:**
- Modify: `web/crates/recap-core/src/db/mod.rs:500-510`

**Step 1: Add CREATE TABLE statement**

After the `project_issue_mappings` table creation (around line 503), add:

```rust
        // Project descriptions for AI context
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS project_descriptions (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                project_name TEXT NOT NULL,
                goal TEXT,
                tech_stack TEXT,
                key_features TEXT,
                notes TEXT,
                orphaned BOOLEAN DEFAULT 0,
                orphaned_at DATETIME,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(user_id, project_name)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
```

**Step 2: Run cargo check to verify syntax**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && cargo check --package recap-core`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add crates/recap-core/src/db/mod.rs
git commit -m "feat(db): add project_descriptions table"
```

---

### Task 2: Add project_summaries table

**Files:**
- Modify: `web/crates/recap-core/src/db/mod.rs` (after project_descriptions)

**Step 1: Add CREATE TABLE statement**

```rust
        // Project summaries cache
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS project_summaries (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                project_name TEXT NOT NULL,
                period_type TEXT NOT NULL,
                period_start DATE NOT NULL,
                period_end DATE NOT NULL,
                summary TEXT NOT NULL,
                data_hash TEXT,
                orphaned BOOLEAN DEFAULT 0,
                orphaned_at DATETIME,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(user_id, project_name, period_type, period_start)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
```

**Step 2: Run cargo check**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && cargo check --package recap-core`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add crates/recap-core/src/db/mod.rs
git commit -m "feat(db): add project_summaries table"
```

---

### Task 3: Add Rust types for descriptions

**Files:**
- Modify: `web/src-tauri/src/commands/projects/types.rs`

**Step 1: Add new types at end of file**

```rust
/// Project description for AI context
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectDescription {
    pub project_name: String,
    pub goal: Option<String>,
    pub tech_stack: Option<String>,
    pub key_features: Option<Vec<String>>,
    pub notes: Option<String>,
}

/// Request to update project description
#[derive(Debug, Deserialize)]
pub struct UpdateProjectDescriptionRequest {
    pub project_name: String,
    pub goal: Option<String>,
    pub tech_stack: Option<String>,
    pub key_features: Option<Vec<String>>,
    pub notes: Option<String>,
}

/// Project summary from cache
#[derive(Debug, Serialize)]
pub struct ProjectSummary {
    pub period_type: String,
    pub period_start: String,
    pub period_end: String,
    pub summary: String,
    pub is_stale: bool,
}

/// Request to generate project summary
#[derive(Debug, Deserialize)]
pub struct GenerateSummaryRequest {
    pub project_name: String,
    pub period_type: String,  // "week" | "month"
    pub period_start: String,
    pub period_end: String,
}

/// Summary freshness status
#[derive(Debug, Serialize)]
pub struct SummaryFreshness {
    pub project_name: String,
    pub has_new_activity: bool,
    pub last_activity_date: Option<String>,
    pub last_summary_date: Option<String>,
}
```

**Step 2: Run cargo check**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && cargo check`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add src-tauri/src/commands/projects/types.rs
git commit -m "feat(projects): add description and summary types"
```

---

### Task 4: Add TypeScript types

**Files:**
- Modify: `web/src/types/projects.ts`

**Step 1: Add new types at end of file**

```typescript
// Project description for AI context
export interface ProjectDescription {
  project_name: string
  goal: string | null
  tech_stack: string | null
  key_features: string[] | null
  notes: string | null
}

export interface UpdateProjectDescriptionRequest {
  project_name: string
  goal?: string | null
  tech_stack?: string | null
  key_features?: string[] | null
  notes?: string | null
}

// Project summary from cache
export interface ProjectSummary {
  period_type: 'week' | 'month'
  period_start: string
  period_end: string
  summary: string
  is_stale: boolean
}

export interface GenerateSummaryRequest {
  project_name: string
  period_type: 'week' | 'month'
  period_start: string
  period_end: string
}

export interface SummaryFreshness {
  project_name: string
  has_new_activity: boolean
  last_activity_date: string | null
  last_summary_date: string | null
}

// Timeline types for project page
export type TimeUnit = 'day' | 'week' | 'month' | 'quarter' | 'year'

export interface TimelineGroup {
  period_label: string
  period_start: string
  period_end: string
  total_hours: number
  sessions: TimelineSessionDetail[]
  standalone_commits: TimelineCommitDetail[]
}

export interface TimelineSessionDetail {
  id: string
  source: string  // 'claude_code' | 'antigravity'
  title: string
  start_time: string
  end_time: string
  hours: number
  commits: TimelineCommitDetail[]
}

export interface TimelineCommitDetail {
  hash: string
  short_hash: string
  message: string
  author: string
  time: string
  files_changed: number
  insertions: number
  deletions: number
}

export interface ProjectTimelineResponse {
  groups: TimelineGroup[]
  next_cursor: string | null
  has_more: boolean
}

export interface ProjectTimelineRequest {
  project_name: string
  time_unit: TimeUnit
  range_start: string
  range_end: string
  sources?: string[]
  cursor?: string
  limit?: number
}

// Git diff types
export interface CommitFileChange {
  path: string
  status: 'added' | 'modified' | 'deleted' | 'renamed'
  insertions: number
  deletions: number
}

export interface CommitDiff {
  hash: string
  files: CommitFileChange[]
  diff_text: string | null  // null if repo not available locally
}
```

**Step 2: Run TypeScript check**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && npx tsc --noEmit`
Expected: No errors

**Step 3: Commit**

```bash
git add src/types/projects.ts
git commit -m "feat(projects): add TypeScript types for descriptions, summaries, timeline"
```

---

## Phase 2: Backend Commands

### Task 5: Create descriptions.rs command module

**Files:**
- Create: `web/src-tauri/src/commands/projects/descriptions.rs`

**Step 1: Create the file with CRUD commands**

```rust
//! Project description commands
//!
//! CRUD operations for project descriptions (goal, tech stack, features, notes).

use recap_core::auth::verify_token;
use tauri::State;
use uuid::Uuid;

use super::types::{ProjectDescription, UpdateProjectDescriptionRequest};
use crate::commands::AppState;

/// Get project description
#[tauri::command(rename_all = "camelCase")]
pub async fn get_project_description(
    state: State<'_, AppState>,
    token: String,
    project_name: String,
) -> Result<Option<ProjectDescription>, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let row = sqlx::query_as::<_, (Option<String>, Option<String>, Option<String>, Option<String>)>(
        "SELECT goal, tech_stack, key_features, notes FROM project_descriptions WHERE user_id = ? AND project_name = ?",
    )
    .bind(&claims.sub)
    .bind(&project_name)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    match row {
        Some((goal, tech_stack, key_features_json, notes)) => {
            let key_features: Option<Vec<String>> = key_features_json
                .and_then(|s| serde_json::from_str(&s).ok());

            Ok(Some(ProjectDescription {
                project_name,
                goal,
                tech_stack,
                key_features,
                notes,
            }))
        }
        None => Ok(None),
    }
}

/// Update or create project description
#[tauri::command(rename_all = "camelCase")]
pub async fn update_project_description(
    state: State<'_, AppState>,
    token: String,
    request: UpdateProjectDescriptionRequest,
) -> Result<String, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    let key_features_json = request
        .key_features
        .map(|f| serde_json::to_string(&f).unwrap_or_default());

    let id = Uuid::new_v4().to_string();

    sqlx::query(
        r#"
        INSERT INTO project_descriptions (id, user_id, project_name, goal, tech_stack, key_features, notes, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(user_id, project_name) DO UPDATE SET
            goal = excluded.goal,
            tech_stack = excluded.tech_stack,
            key_features = excluded.key_features,
            notes = excluded.notes,
            orphaned = 0,
            orphaned_at = NULL,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&id)
    .bind(&claims.sub)
    .bind(&request.project_name)
    .bind(&request.goal)
    .bind(&request.tech_stack)
    .bind(&key_features_json)
    .bind(&request.notes)
    .execute(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok("Description updated".to_string())
}

/// Delete project description
#[tauri::command(rename_all = "camelCase")]
pub async fn delete_project_description(
    state: State<'_, AppState>,
    token: String,
    project_name: String,
) -> Result<String, String> {
    let claims = verify_token(&token).map_err(|e| e.to_string())?;
    let db = state.db.lock().await;

    sqlx::query("DELETE FROM project_descriptions WHERE user_id = ? AND project_name = ?")
        .bind(&claims.sub)
        .bind(&project_name)
        .execute(&db.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok("Description deleted".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_description_serialization() {
        let desc = ProjectDescription {
            project_name: "test".to_string(),
            goal: Some("Test goal".to_string()),
            tech_stack: Some("Rust, React".to_string()),
            key_features: Some(vec!["Feature 1".to_string(), "Feature 2".to_string()]),
            notes: None,
        };

        let json = serde_json::to_string(&desc).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("Test goal"));
    }
}
```

**Step 2: Run cargo check**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && cargo check`
Expected: Compiles (may have unused warning, that's OK)

**Step 3: Commit**

```bash
git add src-tauri/src/commands/projects/descriptions.rs
git commit -m "feat(projects): add description CRUD commands"
```

---

### Task 6: Update projects mod.rs to export descriptions

**Files:**
- Modify: `web/src-tauri/src/commands/projects/mod.rs`

**Step 1: Add module declaration**

```rust
//! Projects commands
//!
//! Tauri commands for project management and visibility.
//!
//! This module is organized into:
//! - `types`: Type definitions for requests/responses
//! - `queries`: List, detail, visibility, and hidden project queries
//! - `descriptions`: Project description CRUD

pub mod descriptions;
pub mod queries;
pub mod types;
```

**Step 2: Run cargo check**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && cargo check`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add src-tauri/src/commands/projects/mod.rs
git commit -m "feat(projects): export descriptions module"
```

---

### Task 7: Register description commands in lib.rs

**Files:**
- Modify: `web/src-tauri/src/lib.rs`

**Step 1: Find the invoke_handler and add new commands**

Search for `invoke_handler` and add to the list:
- `commands::projects::descriptions::get_project_description`
- `commands::projects::descriptions::update_project_description`
- `commands::projects::descriptions::delete_project_description`

**Step 2: Run cargo check**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && cargo check`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(projects): register description commands"
```

---

### Task 8: Add frontend service functions for descriptions

**Files:**
- Modify: `web/src/services/projects.ts`

**Step 1: Add imports and functions**

Add to imports:
```typescript
import type {
  // ... existing imports ...
  ProjectDescription,
  UpdateProjectDescriptionRequest,
} from '@/types'
```

Add functions:
```typescript
/**
 * Get project description (goal, tech stack, etc.)
 */
export async function getProjectDescription(projectName: string): Promise<ProjectDescription | null> {
  return invokeAuth<ProjectDescription | null>('get_project_description', { projectName })
}

/**
 * Update or create project description
 */
export async function updateProjectDescription(request: UpdateProjectDescriptionRequest): Promise<string> {
  return invokeAuth<string>('update_project_description', { request })
}

/**
 * Delete project description
 */
export async function deleteProjectDescription(projectName: string): Promise<string> {
  return invokeAuth<string>('delete_project_description', { projectName })
}
```

**Step 2: Run TypeScript check**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && npx tsc --noEmit`
Expected: No errors

**Step 3: Commit**

```bash
git add src/services/projects.ts
git commit -m "feat(projects): add description service functions"
```

---

## Phase 3: Frontend - Page Structure

### Task 9: Create Projects page directory structure

**Files:**
- Create directories and placeholder files

**Step 1: Create directory structure**

```bash
cd /Users/weifanliao/PycharmProjects/recap/web
mkdir -p src/pages/Projects/components/ProjectList
mkdir -p src/pages/Projects/components/ProjectDetail
mkdir -p src/pages/Projects/components/Timeline
mkdir -p src/pages/Projects/components/Summary
mkdir -p src/pages/Projects/components/Modals
mkdir -p src/pages/Projects/hooks
```

**Step 2: Commit structure**

```bash
git add src/pages/Projects/
git commit -m "chore(projects): create page directory structure"
```

---

### Task 10: Implement main Projects page with left-right layout

**Files:**
- Modify: `web/src/pages/Projects/index.tsx`

**Step 1: Replace placeholder with actual implementation**

```tsx
import { useState } from 'react'
import { FolderKanban } from 'lucide-react'
import { ProjectList } from './components/ProjectList'
import { ProjectDetail } from './components/ProjectDetail'

export function ProjectsPage() {
  const [selectedProject, setSelectedProject] = useState<string | null>(null)

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <header className="flex-shrink-0 pb-6 animate-fade-up opacity-0 delay-1">
        <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
          Projects
        </p>
        <h1 className="font-display text-4xl text-foreground tracking-tight">
          專案
        </h1>
      </header>

      {/* Main content - left/right split */}
      <div className="flex-1 flex gap-6 min-h-0 animate-fade-up opacity-0 delay-2">
        {/* Left panel - Project list */}
        <div className="w-64 flex-shrink-0 flex flex-col min-h-0">
          <ProjectList
            selectedProject={selectedProject}
            onSelectProject={setSelectedProject}
          />
        </div>

        {/* Right panel - Project detail */}
        <div className="flex-1 min-w-0">
          {selectedProject ? (
            <ProjectDetail projectName={selectedProject} />
          ) : (
            <div className="h-full flex flex-col items-center justify-center text-center">
              <div className="w-16 h-16 rounded-full bg-muted/50 flex items-center justify-center mb-6">
                <FolderKanban className="w-8 h-8 text-muted-foreground" strokeWidth={1.5} />
              </div>
              <h2 className="text-lg font-medium text-foreground mb-2">選擇專案</h2>
              <p className="text-sm text-muted-foreground max-w-md">
                從左側列表選擇一個專案，查看詳細資訊和時間軸。
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
```

**Step 2: Run TypeScript check (will fail - that's expected)**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && npx tsc --noEmit`
Expected: Errors about missing ProjectList and ProjectDetail (we'll create them next)

**Step 3: Commit**

```bash
git add src/pages/Projects/index.tsx
git commit -m "feat(projects): implement main page layout"
```

---

### Task 11: Create ProjectList component

**Files:**
- Create: `web/src/pages/Projects/components/ProjectList/index.tsx`

**Step 1: Create the component**

```tsx
import { useState } from 'react'
import { Search, Plus, Eye, EyeOff } from 'lucide-react'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { ScrollArea } from '@/components/ui/scroll-area'
import { useProjects } from '../../hooks/useProjects'
import { ProjectCard } from './ProjectCard'

interface ProjectListProps {
  selectedProject: string | null
  onSelectProject: (projectName: string | null) => void
}

export function ProjectList({ selectedProject, onSelectProject }: ProjectListProps) {
  const [search, setSearch] = useState('')
  const [showHidden, setShowHidden] = useState(false)
  const { projects, isLoading } = useProjects({ showHidden })

  const filteredProjects = projects.filter(p =>
    p.project_name.toLowerCase().includes(search.toLowerCase()) ||
    (p.display_name?.toLowerCase().includes(search.toLowerCase()))
  )

  return (
    <div className="h-full flex flex-col bg-card rounded-lg border">
      {/* Search */}
      <div className="p-3 border-b">
        <div className="relative">
          <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="搜尋專案..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="pl-8 h-9"
          />
        </div>
      </div>

      {/* Project list */}
      <ScrollArea className="flex-1">
        <div className="p-2 space-y-1">
          {isLoading ? (
            <div className="p-4 text-center text-sm text-muted-foreground">
              載入中...
            </div>
          ) : filteredProjects.length === 0 ? (
            <div className="p-4 text-center text-sm text-muted-foreground">
              {search ? '找不到符合的專案' : '尚無專案'}
            </div>
          ) : (
            filteredProjects.map((project) => (
              <ProjectCard
                key={project.project_name}
                project={project}
                isSelected={selectedProject === project.project_name}
                onClick={() => onSelectProject(project.project_name)}
              />
            ))
          )}
        </div>
      </ScrollArea>

      {/* Footer */}
      <div className="p-3 border-t">
        <div className="flex items-center gap-2">
          <Checkbox
            id="show-hidden"
            checked={showHidden}
            onCheckedChange={(checked) => setShowHidden(checked === true)}
          />
          <label
            htmlFor="show-hidden"
            className="text-xs text-muted-foreground cursor-pointer"
          >
            顯示隱藏專案
          </label>
        </div>
      </div>
    </div>
  )
}
```

**Step 2: Commit (will have TypeScript errors until we create dependencies)**

```bash
git add src/pages/Projects/components/ProjectList/index.tsx
git commit -m "feat(projects): add ProjectList component"
```

---

### Task 12: Create ProjectCard component

**Files:**
- Create: `web/src/pages/Projects/components/ProjectList/ProjectCard.tsx`

**Step 1: Create the component**

```tsx
import { Folder, Clock, Eye, EyeOff } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Badge } from '@/components/ui/badge'
import type { ProjectInfo } from '@/types'

interface ProjectCardProps {
  project: ProjectInfo
  isSelected: boolean
  onClick: () => void
}

const SOURCE_COLORS: Record<string, string> = {
  claude_code: 'bg-orange-500/10 text-orange-600 border-orange-500/20',
  antigravity: 'bg-purple-500/10 text-purple-600 border-purple-500/20',
  git: 'bg-green-500/10 text-green-600 border-green-500/20',
  gitlab: 'bg-blue-500/10 text-blue-600 border-blue-500/20',
  manual: 'bg-gray-500/10 text-gray-600 border-gray-500/20',
}

const SOURCE_LABELS: Record<string, string> = {
  claude_code: 'Claude',
  antigravity: 'Gemini',
  git: 'Git',
  gitlab: 'GitLab',
  manual: '手動',
}

export function ProjectCard({ project, isSelected, onClick }: ProjectCardProps) {
  const displayName = project.display_name || project.project_name

  return (
    <button
      onClick={onClick}
      className={cn(
        'w-full text-left p-3 rounded-md transition-colors',
        'hover:bg-accent',
        isSelected && 'bg-accent',
        project.hidden && 'opacity-60'
      )}
    >
      <div className="flex items-start gap-2">
        <Folder className="w-4 h-4 mt-0.5 text-muted-foreground flex-shrink-0" />
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-1.5">
            <span className="font-medium text-sm truncate">{displayName}</span>
            {project.hidden && (
              <EyeOff className="w-3 h-3 text-muted-foreground flex-shrink-0" />
            )}
          </div>

          <div className="flex items-center gap-2 mt-1 text-xs text-muted-foreground">
            <span className="flex items-center gap-1">
              <Clock className="w-3 h-3" />
              {project.total_hours.toFixed(1)}h
            </span>
            <span>·</span>
            <span>{project.work_item_count} 項目</span>
          </div>

          <div className="flex flex-wrap gap-1 mt-2">
            {project.sources.map((source) => (
              <Badge
                key={source}
                variant="outline"
                className={cn('text-[10px] px-1.5 py-0', SOURCE_COLORS[source])}
              >
                {SOURCE_LABELS[source] || source}
              </Badge>
            ))}
          </div>
        </div>
      </div>
    </button>
  )
}
```

**Step 2: Commit**

```bash
git add src/pages/Projects/components/ProjectList/ProjectCard.tsx
git commit -m "feat(projects): add ProjectCard component"
```

---

### Task 13: Create useProjects hook

**Files:**
- Create: `web/src/pages/Projects/hooks/useProjects.ts`

**Step 1: Create the hook**

```typescript
import { useState, useEffect } from 'react'
import { projects as projectsService } from '@/services'
import type { ProjectInfo } from '@/types'

interface UseProjectsOptions {
  showHidden?: boolean
}

interface UseProjectsReturn {
  projects: ProjectInfo[]
  isLoading: boolean
  error: string | null
  refetch: () => Promise<void>
}

export function useProjects(options: UseProjectsOptions = {}): UseProjectsReturn {
  const { showHidden = false } = options
  const [projects, setProjects] = useState<ProjectInfo[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const fetchProjects = async () => {
    try {
      setIsLoading(true)
      setError(null)
      const data = await projectsService.listProjects()

      // Filter hidden projects if needed
      const filtered = showHidden
        ? data
        : data.filter(p => !p.hidden)

      // Sort by total hours descending
      filtered.sort((a, b) => b.total_hours - a.total_hours)

      setProjects(filtered)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load projects')
    } finally {
      setIsLoading(false)
    }
  }

  useEffect(() => {
    fetchProjects()
  }, [showHidden])

  return {
    projects,
    isLoading,
    error,
    refetch: fetchProjects,
  }
}
```

**Step 2: Run TypeScript check**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && npx tsc --noEmit`
Expected: Should pass (or only have errors from unfinished components)

**Step 3: Commit**

```bash
git add src/pages/Projects/hooks/useProjects.ts
git commit -m "feat(projects): add useProjects hook"
```

---

### Task 14: Create ProjectDetail container

**Files:**
- Create: `web/src/pages/Projects/components/ProjectDetail/index.tsx`

**Step 1: Create the component with tabs**

```tsx
import { useState } from 'react'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Card } from '@/components/ui/card'
import { InfoTab } from './InfoTab'
import { TimelineTab } from './TimelineTab'
import { SettingsTab } from './SettingsTab'
import { useProjectDetail } from '../../hooks/useProjectDetail'

interface ProjectDetailProps {
  projectName: string
}

export function ProjectDetail({ projectName }: ProjectDetailProps) {
  const [activeTab, setActiveTab] = useState('info')
  const { detail, isLoading, error, refetch } = useProjectDetail(projectName)

  if (isLoading) {
    return (
      <Card className="h-full flex items-center justify-center">
        <span className="text-muted-foreground">載入中...</span>
      </Card>
    )
  }

  if (error || !detail) {
    return (
      <Card className="h-full flex items-center justify-center">
        <span className="text-destructive">{error || '無法載入專案'}</span>
      </Card>
    )
  }

  const displayName = detail.display_name || detail.project_name

  return (
    <Card className="h-full flex flex-col">
      {/* Header */}
      <div className="p-4 border-b">
        <h2 className="text-xl font-semibold">{displayName}</h2>
        {detail.project_path && (
          <p className="text-xs text-muted-foreground mt-1 font-mono truncate">
            {detail.project_path}
          </p>
        )}
      </div>

      {/* Tabs */}
      <Tabs value={activeTab} onValueChange={setActiveTab} className="flex-1 flex flex-col min-h-0">
        <TabsList className="mx-4 mt-2 w-fit">
          <TabsTrigger value="info">專案資訊</TabsTrigger>
          <TabsTrigger value="timeline">時間軸</TabsTrigger>
          <TabsTrigger value="settings">設定</TabsTrigger>
        </TabsList>

        <div className="flex-1 min-h-0 overflow-hidden">
          <TabsContent value="info" className="h-full m-0 p-4 overflow-auto">
            <InfoTab projectName={projectName} detail={detail} onUpdate={refetch} />
          </TabsContent>
          <TabsContent value="timeline" className="h-full m-0 p-4 overflow-auto">
            <TimelineTab projectName={projectName} />
          </TabsContent>
          <TabsContent value="settings" className="h-full m-0 p-4 overflow-auto">
            <SettingsTab projectName={projectName} detail={detail} onUpdate={refetch} />
          </TabsContent>
        </div>
      </Tabs>
    </Card>
  )
}
```

**Step 2: Commit**

```bash
git add src/pages/Projects/components/ProjectDetail/index.tsx
git commit -m "feat(projects): add ProjectDetail container with tabs"
```

---

### Task 15: Create useProjectDetail hook

**Files:**
- Create: `web/src/pages/Projects/hooks/useProjectDetail.ts`

**Step 1: Create the hook**

```typescript
import { useState, useEffect } from 'react'
import { projects as projectsService } from '@/services'
import type { ProjectDetail } from '@/types'

interface UseProjectDetailReturn {
  detail: ProjectDetail | null
  isLoading: boolean
  error: string | null
  refetch: () => Promise<void>
}

export function useProjectDetail(projectName: string): UseProjectDetailReturn {
  const [detail, setDetail] = useState<ProjectDetail | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const fetchDetail = async () => {
    try {
      setIsLoading(true)
      setError(null)
      const data = await projectsService.getProjectDetail(projectName)
      setDetail(data)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load project detail')
    } finally {
      setIsLoading(false)
    }
  }

  useEffect(() => {
    if (projectName) {
      fetchDetail()
    }
  }, [projectName])

  return {
    detail,
    isLoading,
    error,
    refetch: fetchDetail,
  }
}
```

**Step 2: Commit**

```bash
git add src/pages/Projects/hooks/useProjectDetail.ts
git commit -m "feat(projects): add useProjectDetail hook"
```

---

### Task 16: Create InfoTab component (basic version)

**Files:**
- Create: `web/src/pages/Projects/components/ProjectDetail/InfoTab.tsx`

**Step 1: Create the component**

```tsx
import { useState, useEffect } from 'react'
import { Pencil, Target, Wrench, Star, FileText, RefreshCw } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { projects as projectsService } from '@/services'
import { EditDescriptionModal } from '../Modals/EditDescriptionModal'
import type { ProjectDetail, ProjectDescription } from '@/types'

interface InfoTabProps {
  projectName: string
  detail: ProjectDetail
  onUpdate: () => void
}

export function InfoTab({ projectName, detail, onUpdate }: InfoTabProps) {
  const [description, setDescription] = useState<ProjectDescription | null>(null)
  const [isLoadingDesc, setIsLoadingDesc] = useState(true)
  const [showEditModal, setShowEditModal] = useState(false)

  const fetchDescription = async () => {
    try {
      setIsLoadingDesc(true)
      const data = await projectsService.getProjectDescription(projectName)
      setDescription(data)
    } catch (err) {
      console.error('Failed to load description:', err)
    } finally {
      setIsLoadingDesc(false)
    }
  }

  useEffect(() => {
    fetchDescription()
  }, [projectName])

  const handleDescriptionSaved = () => {
    setShowEditModal(false)
    fetchDescription()
  }

  return (
    <div className="space-y-6">
      {/* Project Description */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between pb-2">
          <CardTitle className="text-base">專案描述</CardTitle>
          <Button variant="ghost" size="sm" onClick={() => setShowEditModal(true)}>
            <Pencil className="w-4 h-4 mr-1" />
            編輯
          </Button>
        </CardHeader>
        <CardContent className="space-y-4">
          {isLoadingDesc ? (
            <p className="text-sm text-muted-foreground">載入中...</p>
          ) : description ? (
            <>
              {description.goal && (
                <div>
                  <div className="flex items-center gap-2 text-sm font-medium mb-1">
                    <Target className="w-4 h-4 text-muted-foreground" />
                    專案目標
                  </div>
                  <p className="text-sm text-muted-foreground pl-6">{description.goal}</p>
                </div>
              )}
              {description.tech_stack && (
                <div>
                  <div className="flex items-center gap-2 text-sm font-medium mb-1">
                    <Wrench className="w-4 h-4 text-muted-foreground" />
                    技術棧
                  </div>
                  <p className="text-sm text-muted-foreground pl-6">{description.tech_stack}</p>
                </div>
              )}
              {description.key_features && description.key_features.length > 0 && (
                <div>
                  <div className="flex items-center gap-2 text-sm font-medium mb-1">
                    <Star className="w-4 h-4 text-muted-foreground" />
                    關鍵功能
                  </div>
                  <ul className="text-sm text-muted-foreground pl-6 list-disc list-inside">
                    {description.key_features.map((feature, i) => (
                      <li key={i}>{feature}</li>
                    ))}
                  </ul>
                </div>
              )}
              {description.notes && (
                <div>
                  <div className="flex items-center gap-2 text-sm font-medium mb-1">
                    <FileText className="w-4 h-4 text-muted-foreground" />
                    備註
                  </div>
                  <p className="text-sm text-muted-foreground pl-6 whitespace-pre-wrap">{description.notes}</p>
                </div>
              )}
              {!description.goal && !description.tech_stack && !description.key_features?.length && !description.notes && (
                <p className="text-sm text-muted-foreground">尚未填寫專案描述</p>
              )}
            </>
          ) : (
            <p className="text-sm text-muted-foreground">
              尚未填寫專案描述。點擊編輯按鈕新增。
            </p>
          )}
        </CardContent>
      </Card>

      {/* Stats */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-base">統計</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-3 gap-4 text-center">
            <div>
              <p className="text-2xl font-semibold">{detail.stats.total_hours.toFixed(1)}</p>
              <p className="text-xs text-muted-foreground">總時數</p>
            </div>
            <div>
              <p className="text-2xl font-semibold">{detail.stats.total_items}</p>
              <p className="text-xs text-muted-foreground">工作項目</p>
            </div>
            <div>
              <p className="text-2xl font-semibold">{detail.sources.length}</p>
              <p className="text-xs text-muted-foreground">資料來源</p>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Sources */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-base">資料來源</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-2">
            {detail.sources.map((source) => (
              <div key={source.source} className="flex items-center justify-between text-sm">
                <span className="capitalize">{source.source.replace('_', ' ')}</span>
                <div className="flex items-center gap-4 text-muted-foreground">
                  <span>{source.item_count} 項目</span>
                  {source.latest_date && (
                    <span>最近: {source.latest_date}</span>
                  )}
                </div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>

      {/* Edit Modal */}
      <EditDescriptionModal
        open={showEditModal}
        onOpenChange={setShowEditModal}
        projectName={projectName}
        initialData={description}
        onSaved={handleDescriptionSaved}
      />
    </div>
  )
}
```

**Step 2: Commit**

```bash
git add src/pages/Projects/components/ProjectDetail/InfoTab.tsx
git commit -m "feat(projects): add InfoTab component"
```

---

### Task 17: Create EditDescriptionModal

**Files:**
- Create: `web/src/pages/Projects/components/Modals/EditDescriptionModal.tsx`

**Step 1: Create the component**

```tsx
import { useState, useEffect } from 'react'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { Label } from '@/components/ui/label'
import { projects as projectsService } from '@/services'
import type { ProjectDescription } from '@/types'

interface EditDescriptionModalProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  projectName: string
  initialData: ProjectDescription | null
  onSaved: () => void
}

export function EditDescriptionModal({
  open,
  onOpenChange,
  projectName,
  initialData,
  onSaved,
}: EditDescriptionModalProps) {
  const [goal, setGoal] = useState('')
  const [techStack, setTechStack] = useState('')
  const [keyFeatures, setKeyFeatures] = useState('')
  const [notes, setNotes] = useState('')
  const [isSaving, setIsSaving] = useState(false)

  // Reset form when modal opens
  useEffect(() => {
    if (open) {
      setGoal(initialData?.goal || '')
      setTechStack(initialData?.tech_stack || '')
      setKeyFeatures(initialData?.key_features?.join('\n') || '')
      setNotes(initialData?.notes || '')
    }
  }, [open, initialData])

  const handleSave = async () => {
    try {
      setIsSaving(true)

      // Parse key features (one per line)
      const features = keyFeatures
        .split('\n')
        .map(s => s.trim())
        .filter(s => s.length > 0)

      await projectsService.updateProjectDescription({
        project_name: projectName,
        goal: goal.trim() || null,
        tech_stack: techStack.trim() || null,
        key_features: features.length > 0 ? features : null,
        notes: notes.trim() || null,
      })

      onSaved()
    } catch (err) {
      console.error('Failed to save description:', err)
    } finally {
      setIsSaving(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>編輯專案描述</DialogTitle>
        </DialogHeader>

        <div className="space-y-4 py-4">
          <div className="space-y-2">
            <Label htmlFor="goal">專案目標</Label>
            <Textarea
              id="goal"
              placeholder="描述專案要解決的問題或達成的目標"
              value={goal}
              onChange={(e) => setGoal(e.target.value)}
              rows={2}
            />
            <p className="text-xs text-muted-foreground">
              這會作為 AI 生成摘要的背景資訊
            </p>
          </div>

          <div className="space-y-2">
            <Label htmlFor="tech-stack">技術棧</Label>
            <Input
              id="tech-stack"
              placeholder="例：Tauri, React, Rust, SQLite"
              value={techStack}
              onChange={(e) => setTechStack(e.target.value)}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="key-features">關鍵功能</Label>
            <Textarea
              id="key-features"
              placeholder="每行一項功能&#10;例：&#10;自動捕獲工作 session&#10;Git commit 追蹤&#10;工作摘要生成"
              value={keyFeatures}
              onChange={(e) => setKeyFeatures(e.target.value)}
              rows={4}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="notes">備註</Label>
            <Textarea
              id="notes"
              placeholder="其他補充資訊、開發計畫、注意事項等"
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              rows={3}
            />
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            取消
          </Button>
          <Button onClick={handleSave} disabled={isSaving}>
            {isSaving ? '儲存中...' : '儲存'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
```

**Step 2: Commit**

```bash
git add src/pages/Projects/components/Modals/EditDescriptionModal.tsx
git commit -m "feat(projects): add EditDescriptionModal"
```

---

### Task 18: Create TimelineTab placeholder

**Files:**
- Create: `web/src/pages/Projects/components/ProjectDetail/TimelineTab.tsx`

**Step 1: Create placeholder component**

```tsx
import { Construction } from 'lucide-react'

interface TimelineTabProps {
  projectName: string
}

export function TimelineTab({ projectName }: TimelineTabProps) {
  return (
    <div className="h-full flex flex-col items-center justify-center text-center">
      <Construction className="w-12 h-12 text-muted-foreground mb-4" />
      <h3 className="text-lg font-medium mb-2">時間軸功能開發中</h3>
      <p className="text-sm text-muted-foreground max-w-md">
        此功能將在後續版本中推出，敬請期待。
      </p>
    </div>
  )
}
```

**Step 2: Commit**

```bash
git add src/pages/Projects/components/ProjectDetail/TimelineTab.tsx
git commit -m "feat(projects): add TimelineTab placeholder"
```

---

### Task 19: Create SettingsTab component

**Files:**
- Create: `web/src/pages/Projects/components/ProjectDetail/SettingsTab.tsx`

**Step 1: Create the component**

```tsx
import { useState } from 'react'
import { Eye, EyeOff, Trash2 } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from '@/components/ui/alert-dialog'
import { projects as projectsService } from '@/services'
import type { ProjectDetail } from '@/types'

interface SettingsTabProps {
  projectName: string
  detail: ProjectDetail
  onUpdate: () => void
}

export function SettingsTab({ projectName, detail, onUpdate }: SettingsTabProps) {
  const [isVisible, setIsVisible] = useState(!detail.hidden)
  const [isSaving, setIsSaving] = useState(false)

  const handleVisibilityChange = async (visible: boolean) => {
    try {
      setIsSaving(true)
      await projectsService.setProjectVisibility(projectName, !visible)
      setIsVisible(visible)
      onUpdate()
    } catch (err) {
      console.error('Failed to update visibility:', err)
      setIsVisible(!visible) // Revert on error
    } finally {
      setIsSaving(false)
    }
  }

  const handleRemoveProject = async () => {
    try {
      await projectsService.removeManualProject(projectName)
      onUpdate()
      // TODO: Navigate back to project list
    } catch (err) {
      console.error('Failed to remove project:', err)
    }
  }

  return (
    <div className="space-y-6">
      {/* Project Settings */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">專案設定</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label>顯示名稱</Label>
            <Input
              value={detail.display_name || detail.project_name}
              disabled
              className="bg-muted"
            />
            <p className="text-xs text-muted-foreground">
              顯示名稱目前無法修改
            </p>
          </div>

          <div className="space-y-2">
            <Label>專案路徑</Label>
            <Input
              value={detail.project_path || '無'}
              disabled
              className="bg-muted font-mono text-xs"
            />
          </div>

          <div className="flex items-center justify-between">
            <div className="space-y-0.5">
              <Label>可見性</Label>
              <p className="text-xs text-muted-foreground">
                在專案列表中顯示此專案
              </p>
            </div>
            <Switch
              checked={isVisible}
              onCheckedChange={handleVisibilityChange}
              disabled={isSaving}
            />
          </div>
        </CardContent>
      </Card>

      {/* Danger Zone */}
      <Card className="border-destructive/50">
        <CardHeader>
          <CardTitle className="text-base text-destructive">危險區域</CardTitle>
          <CardDescription>
            以下操作無法復原，請謹慎操作
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium">移除專案</p>
              <p className="text-xs text-muted-foreground">
                從 Recap 中移除此專案（不會刪除實際檔案）
              </p>
            </div>
            <AlertDialog>
              <AlertDialogTrigger asChild>
                <Button variant="destructive" size="sm">
                  <Trash2 className="w-4 h-4 mr-1" />
                  移除專案
                </Button>
              </AlertDialogTrigger>
              <AlertDialogContent>
                <AlertDialogHeader>
                  <AlertDialogTitle>確定要移除專案？</AlertDialogTitle>
                  <AlertDialogDescription>
                    此操作會將專案從 Recap 中移除。工作項目紀錄不會被刪除，
                    但專案描述和摘要會一併刪除。
                  </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                  <AlertDialogCancel>取消</AlertDialogCancel>
                  <AlertDialogAction onClick={handleRemoveProject}>
                    確認移除
                  </AlertDialogAction>
                </AlertDialogFooter>
              </AlertDialogContent>
            </AlertDialog>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
```

**Step 2: Commit**

```bash
git add src/pages/Projects/components/ProjectDetail/SettingsTab.tsx
git commit -m "feat(projects): add SettingsTab component"
```

---

### Task 20: Create hooks index file and verify compilation

**Files:**
- Create: `web/src/pages/Projects/hooks/index.ts`

**Step 1: Create index file**

```typescript
export * from './useProjects'
export * from './useProjectDetail'
```

**Step 2: Run full TypeScript check**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && npx tsc --noEmit`
Expected: No errors

**Step 3: Run dev server to verify UI**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && npm run dev`
Expected: Server starts without errors

**Step 4: Commit**

```bash
git add src/pages/Projects/hooks/index.ts
git commit -m "feat(projects): add hooks index file"
```

---

### Task 21: Final integration test

**Step 1: Build the entire project**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && cargo build`
Expected: Builds without errors

**Step 2: Run all tests**

Run: `cd /Users/weifanliao/PycharmProjects/recap/web && cargo test --workspace && npm test`
Expected: All tests pass

**Step 3: Create summary commit**

```bash
git add -A
git commit -m "feat(projects): complete Phase 1-3 Projects page implementation

- Added project_descriptions and project_summaries tables
- Added description CRUD commands (Rust + TypeScript)
- Implemented Projects page with master-detail layout
- Added ProjectList with search and filtering
- Added InfoTab with description editing
- Added SettingsTab with visibility toggle
- Added TimelineTab placeholder for future implementation"
```

---

## Remaining Phases (Future Work)

### Phase 4: Timeline Implementation
- Task 22-27: Implement timeline backend (multi-unit grouping, cursor pagination)
- Task 28-33: Implement timeline frontend (controls, groups, sessions, commits)

### Phase 5: Summary Implementation
- Task 34-37: Implement summary backend (caching, generation, freshness check)
- Task 38-41: Implement summary frontend (cards, generation UI)

### Phase 6: Git Diff Implementation
- Task 42-45: Implement git diff backend and frontend

---

## Test Commands Reference

```bash
# TypeScript type check
npx tsc --noEmit

# Rust compile check
cargo check

# Rust tests
cargo test --workspace

# Frontend tests
npm test

# Full build
cargo build

# Dev server
npm run dev

# Tauri dev (with backend)
cargo tauri dev
```
