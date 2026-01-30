/**
 * Project types
 */

export interface ProjectInfo {
  project_name: string
  project_path: string | null
  source: string
  /** All sources that contributed to this project (for showing multiple badges) */
  sources: string[]
  work_item_count: number
  total_hours: number
  latest_date: string | null
  hidden: boolean
  display_name: string | null
}

export interface ProjectSourceInfo {
  source: string
  item_count: number
  latest_date: string | null
  project_path: string | null
}

export interface ProjectWorkItemSummary {
  id: string
  title: string
  date: string
  hours: number
  source: string
}

export interface ProjectStats {
  total_items: number
  total_hours: number
  date_range: [string, string] | null
}

export interface ProjectDetail {
  project_name: string
  project_path: string | null
  hidden: boolean
  display_name: string | null
  sources: ProjectSourceInfo[]
  recent_items: ProjectWorkItemSummary[]
  stats: ProjectStats
}

export interface SetProjectVisibilityRequest {
  project_name: string
  hidden: boolean
}

export interface ClaudeCodeDirEntry {
  path: string
  session_count: number
}

export interface ProjectDirectories {
  claude_code_dirs: ClaudeCodeDirEntry[]
  git_repo_path: string | null
}

export interface AddManualProjectRequest {
  project_name: string
  git_repo_path: string
  display_name?: string | null
}

export interface ClaudeSessionPathResponse {
  path: string
  is_default: boolean
}

export interface AntigravitySessionPathResponse {
  path: string
  is_default: boolean
}

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

// Project AI summary from cache (renamed to avoid conflict with integrations/ProjectSummary)
export interface ProjectAISummary {
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
  source: string // 'claude_code' | 'antigravity'
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
  diff_text: string | null // null if repo not available locally
}
