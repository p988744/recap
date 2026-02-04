/**
 * Integration types for GitLab, Tempo, Claude, Sources
 */

// ============ Sources ============

export interface GitRepoInfo {
  id: string
  path: string
  name: string
  valid: boolean
  last_commit?: string
  last_commit_date?: string
}

export interface SourcesResponse {
  mode: string
  git_repos: GitRepoInfo[]
  claude_connected: boolean
  claude_path?: string
}

export interface AddGitRepoResponse {
  success: boolean
  message: string
  repo?: GitRepoInfo
}

export interface SourceModeResponse {
  success: boolean
  message: string
}

// ============ GitLab ============

export interface GitLabConfigStatus {
  configured: boolean
  gitlab_url?: string
}

export interface ConfigureGitLabRequest {
  gitlab_url: string
  gitlab_pat: string
}

export interface GitLabProject {
  id: string
  user_id: string
  gitlab_project_id: number
  name: string
  path_with_namespace: string
  gitlab_url: string
  default_branch: string
  enabled: boolean
  last_synced?: string
  created_at: string
}

export interface AddGitLabProjectRequest {
  gitlab_project_id: number
  name?: string
  path_with_namespace?: string
  gitlab_url?: string
  default_branch?: string
}

export interface SyncGitLabRequest {
  project_id?: string
  start_date?: string
  end_date?: string
}

export interface SyncGitLabResponse {
  synced_commits: number
  synced_merge_requests: number
  work_items_created: number
}

export interface SearchGitLabProjectsRequest {
  search?: string
}

export interface GitLabProjectInfo {
  id: number
  name: string
  path_with_namespace: string
  web_url: string
  default_branch?: string
}

// ============ Tempo ============

export interface TempoSuccessResponse {
  success: boolean
  message: string
}

export interface WorklogEntryRequest {
  issue_key: string
  date: string
  minutes: number
  description: string
}

export interface WorklogEntryResponse {
  id?: string
  issue_key: string
  date: string
  minutes: number
  hours: number
  description: string
  status: string
  error_message?: string
}

export interface SyncWorklogsRequest {
  entries: WorklogEntryRequest[]
  dry_run?: boolean
}

export interface SyncWorklogsResponse {
  success: boolean
  total_entries: number
  successful: number
  failed: number
  results: WorklogEntryResponse[]
  dry_run: boolean
}

export interface GetWorklogsRequest {
  date_from: string
  date_to: string
}

export interface ValidateIssueResponse {
  valid: boolean
  issue_key: string
  summary?: string
  description?: string
  assignee?: string
  issue_type?: string
  message: string
}

export interface JiraIssueItem {
  key: string
  summary: string
  issue_type?: string
}

export interface JiraIssueDetail {
  key: string
  summary: string
  description?: string
  assignee?: string
  issue_type?: string
}

export interface SearchIssuesRequest {
  query: string
  max_results?: number
}

export interface SearchIssuesResponse {
  issues: JiraIssueItem[]
  total: number
}

// ============ Claude ============

export interface ToolUsage {
  tool_name: string
  count: number
  details: string[]
}

export interface ClaudeSession {
  session_id: string
  agent_id: string
  slug: string
  cwd: string
  git_branch?: string
  first_message?: string
  message_count: number
  first_timestamp?: string
  last_timestamp?: string
  file_path: string
  file_size: number
  tool_usage: ToolUsage[]
  files_modified: string[]
  commands_run: string[]
  user_messages: string[]
}

export interface ClaudeProject {
  path: string
  name: string
  sessions: ClaudeSession[]
}

export interface ImportSessionsRequest {
  session_ids: string[]
}

export interface ImportResult {
  imported: number
  work_items_created: number
}

export interface SummarizeRequest {
  session_file_path: string
}

export interface SummarizeResult {
  summary: string
  success: boolean
  error?: string
}

export interface SyncProjectsRequest {
  project_paths: string[]
}

export interface ClaudeSyncResult {
  sessions_processed: number
  sessions_skipped: number
  work_items_created: number
  work_items_updated: number
}

// ============ Antigravity (Gemini Code) ============
// Uses local HTTP API when Antigravity app is running

export interface AntigravityApiStatus {
  running: boolean
  api_url?: string
  healthy: boolean
  session_count?: number
}

export interface AntigravitySession {
  session_id: string
  summary?: string
  cwd: string
  git_branch?: string
  git_repo?: string
  step_count: number
  first_timestamp?: string
  last_timestamp?: string
  status: string
}

export interface AntigravityProject {
  path: string
  name: string
  sessions: AntigravitySession[]
}

export interface AntigravitySyncProjectsRequest {
  project_paths: string[]
}

export interface AntigravitySyncResult {
  sessions_processed: number
  sessions_skipped: number
  work_items_created: number
  work_items_updated: number
}

// ============ Teams (Legacy) ============

export interface TeamMember {
  account_id: string
  display_name: string
  email: string
}

export interface Team {
  name: string
  jira_group: string
  tempo_team_id?: number
  members: TeamMember[]
  member_count: number
  last_synced?: string
}

// ============ Analyze (Legacy) ============

export interface DailyEntry {
  date: string
  minutes: number
  hours: number
  todos: string[]
  summaries: string[]
  description: string
}

export interface ProjectSummary {
  project_name: string
  project_path: string
  total_minutes: number
  total_hours: number
  daily_entries: DailyEntry[]
  jira_id?: string
  jira_id_suggestions: string[]
}

export interface AnalyzeResponse {
  start_date: string
  end_date: string
  total_minutes: number
  total_hours: number
  dates_covered: string[]
  projects: ProjectSummary[]
  mode: string
}
