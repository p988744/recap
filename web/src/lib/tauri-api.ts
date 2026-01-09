/**
 * Tauri API Client
 *
 * This module provides direct communication with the Rust backend via Tauri commands.
 * It replaces HTTP API calls with invoke() for better performance and security.
 */

import { invoke } from '@tauri-apps/api/core'

// Types (matching Rust structs)

export interface UserResponse {
  id: string
  email: string
  name: string
  username?: string
  employee_id?: string
  department_id?: string
  title?: string
  gitlab_url?: string
  jira_email?: string
  is_active: boolean
  is_admin: boolean
  created_at: string
}

export interface AppStatus {
  has_users: boolean
  user_count: number
  first_user: UserResponse | null
  local_mode: boolean
}

export interface TokenResponse {
  access_token: string
  token_type: string
  expires_in: number
}

export interface RegisterRequest {
  username: string
  password: string
  name: string
  email?: string
  title?: string
}

export interface LoginRequest {
  username: string
  password: string
}

// Auth Commands

/**
 * Get app status (has_users, local_mode, etc.)
 */
export async function getAppStatus(): Promise<AppStatus> {
  return invoke<AppStatus>('get_app_status')
}

/**
 * Register a new user
 */
export async function registerUser(request: RegisterRequest): Promise<UserResponse> {
  return invoke<UserResponse>('register_user', { request })
}

/**
 * Login with username and password
 */
export async function login(request: LoginRequest): Promise<TokenResponse> {
  return invoke<TokenResponse>('login', { request })
}

/**
 * Auto-login for local mode (uses first user)
 */
export async function autoLogin(): Promise<TokenResponse> {
  return invoke<TokenResponse>('auto_login')
}

/**
 * Get current user by token
 */
export async function getCurrentUser(token: string): Promise<UserResponse> {
  return invoke<UserResponse>('get_current_user', { token })
}

// Config Types

export interface ConfigResponse {
  jira_url: string | null
  auth_type: string
  jira_configured: boolean
  tempo_configured: boolean
  llm_provider: string
  llm_model: string
  llm_base_url: string | null
  llm_configured: boolean
  daily_work_hours: number
  normalize_hours: boolean
  gitlab_url: string | null
  gitlab_configured: boolean
  use_git_mode: boolean
  git_repos: string[]
  outlook_enabled: boolean
}

export interface UpdateConfigRequest {
  daily_work_hours?: number
  normalize_hours?: boolean
}

export interface UpdateLlmConfigRequest {
  provider: string
  model: string
  api_key?: string
  base_url?: string
}

export interface UpdateJiraConfigRequest {
  jira_url?: string
  jira_pat?: string
  jira_email?: string
  jira_api_token?: string
  auth_type?: string
  tempo_api_token?: string
}

export interface MessageResponse {
  message: string
}

// Config Commands

/**
 * Get current user configuration
 */
export async function getConfig(token: string): Promise<ConfigResponse> {
  return invoke<ConfigResponse>('get_config', { token })
}

/**
 * Update general config settings
 */
export async function updateConfig(token: string, request: UpdateConfigRequest): Promise<MessageResponse> {
  return invoke<MessageResponse>('update_config', { token, request })
}

/**
 * Update LLM configuration
 */
export async function updateLlmConfig(token: string, request: UpdateLlmConfigRequest): Promise<MessageResponse> {
  return invoke<MessageResponse>('update_llm_config', { token, request })
}

/**
 * Update Jira configuration
 */
export async function updateJiraConfig(token: string, request: UpdateJiraConfigRequest): Promise<MessageResponse> {
  return invoke<MessageResponse>('update_jira_config', { token, request })
}

// Work Items Types

export interface WorkItem {
  id: string
  user_id: string
  source: string
  source_id?: string
  source_url?: string
  title: string
  description?: string
  hours: number
  date: string
  jira_issue_key?: string
  jira_issue_suggested?: string
  jira_issue_title?: string
  category?: string
  tags?: string
  yearly_goal_id?: string
  synced_to_tempo: boolean
  tempo_worklog_id?: string
  synced_at?: string
  created_at: string
  updated_at: string
  parent_id?: string
}

export interface WorkItemWithChildren extends WorkItem {
  child_count: number
}

export interface PaginatedResponse<T> {
  items: T[]
  total: number
  page: number
  per_page: number
  pages: number
}

export interface WorkItemFilters {
  page?: number
  per_page?: number
  source?: string
  category?: string
  jira_mapped?: boolean
  synced_to_tempo?: boolean
  start_date?: string
  end_date?: string
  parent_id?: string
  show_all?: boolean
}

export interface CreateWorkItemRequest {
  title: string
  description?: string
  hours?: number
  date: string
  source?: string
  source_id?: string
  jira_issue_key?: string
  jira_issue_title?: string
  category?: string
  tags?: string[]
}

export interface UpdateWorkItemRequest {
  title?: string
  description?: string
  hours?: number
  date?: string
  jira_issue_key?: string
  jira_issue_title?: string
  category?: string
  tags?: string[]
  synced_to_tempo?: boolean
  tempo_worklog_id?: string
}

// Grouped View Types

export interface WorkLogItem {
  id: string
  title: string
  description?: string
  hours: number
  date: string
  source: string
  synced_to_tempo: boolean
}

export interface JiraIssueGroup {
  jira_key?: string
  jira_title?: string
  total_hours: number
  logs: WorkLogItem[]
}

export interface ProjectGroup {
  project_name: string
  total_hours: number
  issues: JiraIssueGroup[]
}

export interface DateGroup {
  date: string
  total_hours: number
  projects: ProjectGroup[]
}

export interface GroupedWorkItemsResponse {
  by_project: ProjectGroup[]
  by_date: DateGroup[]
  total_hours: number
  total_items: number
}

// Stats Types

export interface DailyHours {
  date: string
  hours: number
  count: number
}

export interface JiraMappingStats {
  mapped: number
  unmapped: number
  percentage: number
}

export interface TempoSyncStats {
  synced: number
  not_synced: number
  percentage: number
}

export interface WorkItemStatsResponse {
  total_items: number
  total_hours: number
  hours_by_source: Record<string, number>
  hours_by_project: Record<string, number>
  hours_by_category: Record<string, number>
  daily_hours: DailyHours[]
  jira_mapping: JiraMappingStats
  tempo_sync: TempoSyncStats
}

// Timeline Types

export interface TimelineCommit {
  hash: string
  message: string
  time: string
  author: string
}

export interface TimelineSession {
  id: string
  project: string
  title: string
  start_time: string
  end_time: string
  hours: number
  commits: TimelineCommit[]
}

export interface TimelineResponse {
  date: string
  sessions: TimelineSession[]
  total_hours: number
  total_commits: number
}

// Batch Sync Types

export interface BatchSyncRequest {
  work_item_ids: string[]
}

export interface BatchSyncResponse {
  synced: number
  failed: number
  errors: string[]
}

// Aggregate Types

export interface AggregateRequest {
  start_date?: string
  end_date?: string
  source?: string
}

export interface AggregateResponse {
  original_count: number
  aggregated_count: number
  deleted_count: number
}

// Work Items Commands

/**
 * List work items with filters
 */
export async function listWorkItems(token: string, filters: WorkItemFilters = {}): Promise<PaginatedResponse<WorkItemWithChildren>> {
  return invoke<PaginatedResponse<WorkItemWithChildren>>('list_work_items', { token, filters })
}

/**
 * Create a new work item
 */
export async function createWorkItem(token: string, request: CreateWorkItemRequest): Promise<WorkItem> {
  return invoke<WorkItem>('create_work_item', { token, request })
}

/**
 * Get a single work item
 */
export async function getWorkItem(token: string, id: string): Promise<WorkItem> {
  return invoke<WorkItem>('get_work_item', { token, id })
}

/**
 * Update a work item
 */
export async function updateWorkItem(token: string, id: string, request: UpdateWorkItemRequest): Promise<WorkItem> {
  return invoke<WorkItem>('update_work_item', { token, id, request })
}

/**
 * Delete a work item
 */
export async function deleteWorkItem(token: string, id: string): Promise<void> {
  return invoke<void>('delete_work_item', { token, id })
}

/**
 * Get work item statistics summary
 */
export async function getStatsSummary(token: string, query: { start_date?: string; end_date?: string } = {}): Promise<WorkItemStatsResponse> {
  return invoke<WorkItemStatsResponse>('get_stats_summary', { token, query })
}

/**
 * Get work items grouped by project and date
 */
export async function getGroupedWorkItems(token: string, query: { start_date?: string; end_date?: string } = {}): Promise<GroupedWorkItemsResponse> {
  return invoke<GroupedWorkItemsResponse>('get_grouped_work_items', { token, query })
}

/**
 * Get timeline data for Gantt chart visualization
 */
export async function getTimelineData(token: string, date: string): Promise<TimelineResponse> {
  return invoke<TimelineResponse>('get_timeline_data', { token, date })
}

/**
 * Batch sync work items to Tempo
 */
export async function batchSyncTempo(token: string, request: BatchSyncRequest): Promise<BatchSyncResponse> {
  return invoke<BatchSyncResponse>('batch_sync_tempo', { token, request })
}

/**
 * Aggregate work items by project + date
 */
export async function aggregateWorkItems(token: string, request: AggregateRequest = {}): Promise<AggregateResponse> {
  return invoke<AggregateResponse>('aggregate_work_items', { token, request })
}

// Claude Types

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

export interface SyncResult {
  sessions_processed: number
  sessions_skipped: number
  work_items_created: number
  work_items_updated: number
}

// Claude Commands

/**
 * List all Claude Code sessions from local machine
 */
export async function listClaudeSessions(token: string): Promise<ClaudeProject[]> {
  return invoke<ClaudeProject[]>('list_claude_sessions', { token })
}

/**
 * Import selected sessions as work items
 */
export async function importClaudeSessions(token: string, request: ImportSessionsRequest): Promise<ImportResult> {
  return invoke<ImportResult>('import_claude_sessions', { token, request })
}

/**
 * Summarize a session using LLM
 */
export async function summarizeClaudeSession(token: string, request: SummarizeRequest): Promise<SummarizeResult> {
  return invoke<SummarizeResult>('summarize_claude_session', { token, request })
}

/**
 * Sync selected projects - aggregate sessions by project+date
 */
export async function syncClaudeProjects(token: string, request: SyncProjectsRequest): Promise<SyncResult> {
  return invoke<SyncResult>('sync_claude_projects', { token, request })
}

// Reports Types

export interface ReportQuery {
  start_date: string
  end_date: string
}

export interface DailyItems {
  date: string
  hours: number
  count: number
}

export interface PersonalReport {
  start_date: string
  end_date: string
  total_hours: number
  total_items: number
  items_by_date: DailyItems[]
  work_items: WorkItem[]
}

export interface SourceSummary {
  source: string
  hours: number
  count: number
}

export interface SummaryReport {
  start_date: string
  end_date: string
  total_hours: number
  total_items: number
  synced_to_tempo: number
  mapped_to_jira: number
  by_source: SourceSummary[]
}

export interface CategorySummary {
  category: string
  hours: number
  count: number
  percentage: number
}

export interface CategoryReport {
  start_date: string
  end_date: string
  categories: CategorySummary[]
}

export interface ExportResult {
  success: boolean
  file_path?: string
  error?: string
}

// Reports Commands

/**
 * Get personal report for date range
 */
export async function getPersonalReport(token: string, query: ReportQuery): Promise<PersonalReport> {
  return invoke<PersonalReport>('get_personal_report', { token, query })
}

/**
 * Get summary report
 */
export async function getSummaryReport(token: string, query: ReportQuery): Promise<SummaryReport> {
  return invoke<SummaryReport>('get_summary_report', { token, query })
}

/**
 * Get report grouped by category
 */
export async function getCategoryReport(token: string, query: ReportQuery): Promise<CategoryReport> {
  return invoke<CategoryReport>('get_category_report', { token, query })
}

/**
 * Get report grouped by source
 */
export async function getSourceReport(token: string, query: ReportQuery): Promise<CategoryReport> {
  return invoke<CategoryReport>('get_source_report', { token, query })
}

/**
 * Export work items to Excel file
 */
export async function exportExcelReport(token: string, query: ReportQuery): Promise<ExportResult> {
  return invoke<ExportResult>('export_excel_report', { token, query })
}

// Sync Types

export interface SyncStatus {
  id: string
  source: string
  source_path?: string
  last_sync_at?: string
  last_item_count: number
  status: string
  error_message?: string
}

export interface AutoSyncRequest {
  project_paths?: string[]
}

export interface SyncResult {
  success: boolean
  source: string
  items_synced: number
  message?: string
}

export interface AutoSyncResponse {
  success: boolean
  results: SyncResult[]
  total_items: number
}

export interface AvailableProject {
  path: string
  name: string
  source: string
}

// Sync Commands

/**
 * Get sync status for all sources
 */
export async function getSyncStatus(token: string): Promise<SyncStatus[]> {
  return invoke<SyncStatus[]>('get_sync_status', { token })
}

/**
 * Trigger auto-sync for Claude projects
 */
export async function autoSync(token: string, request: AutoSyncRequest = {}): Promise<AutoSyncResponse> {
  return invoke<AutoSyncResponse>('auto_sync', { token, request })
}

/**
 * List available projects that can be synced
 */
export async function listAvailableProjects(token: string): Promise<AvailableProject[]> {
  return invoke<AvailableProject[]>('list_available_projects', { token })
}

// GitLab Types

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
  // Optional fields - if not provided, will be fetched from GitLab API
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

// GitLab Commands

/**
 * Get GitLab configuration status
 */
export async function getGitLabStatus(token: string): Promise<GitLabConfigStatus> {
  return invoke<GitLabConfigStatus>('get_gitlab_status', { token })
}

/**
 * Configure GitLab
 */
export async function configureGitLab(token: string, request: ConfigureGitLabRequest): Promise<{ message: string }> {
  return invoke<{ message: string }>('configure_gitlab', { token, request })
}

/**
 * Remove GitLab configuration
 */
export async function removeGitLabConfig(token: string): Promise<{ message: string }> {
  return invoke<{ message: string }>('remove_gitlab_config', { token })
}

/**
 * List user's tracked GitLab projects
 */
export async function listGitLabProjects(token: string): Promise<GitLabProject[]> {
  return invoke<GitLabProject[]>('list_gitlab_projects', { token })
}

/**
 * Add a GitLab project to track
 */
export async function addGitLabProject(token: string, request: AddGitLabProjectRequest): Promise<GitLabProject> {
  return invoke<GitLabProject>('add_gitlab_project', { token, request })
}

/**
 * Remove a GitLab project from tracking
 */
export async function removeGitLabProject(token: string, id: string): Promise<{ message: string }> {
  return invoke<{ message: string }>('remove_gitlab_project', { token, id })
}

/**
 * Sync GitLab data to work items
 */
export async function syncGitLab(token: string, request: SyncGitLabRequest = {}): Promise<SyncGitLabResponse> {
  return invoke<SyncGitLabResponse>('sync_gitlab', { token, request })
}

/**
 * Search GitLab projects
 */
export async function searchGitLabProjects(token: string, request: SearchGitLabProjectsRequest = {}): Promise<GitLabProjectInfo[]> {
  return invoke<GitLabProjectInfo[]>('search_gitlab_projects', { token, request })
}

// Tempo Types

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
  message: string
}

// Tempo Commands

/**
 * Test Jira/Tempo connection
 */
export async function testTempoConnection(token: string): Promise<TempoSuccessResponse> {
  return invoke<TempoSuccessResponse>('test_tempo_connection', { token })
}

/**
 * Validate a Jira issue key
 */
export async function validateJiraIssue(token: string, issueKey: string): Promise<ValidateIssueResponse> {
  return invoke<ValidateIssueResponse>('validate_jira_issue', { token, issue_key: issueKey })
}

/**
 * Sync multiple worklogs to Tempo/Jira
 */
export async function syncWorklogsToTempo(token: string, request: SyncWorklogsRequest): Promise<SyncWorklogsResponse> {
  return invoke<SyncWorklogsResponse>('sync_worklogs_to_tempo', { token, request })
}

/**
 * Upload a single worklog entry
 */
export async function uploadSingleWorklog(token: string, request: WorklogEntryRequest): Promise<WorklogEntryResponse> {
  return invoke<WorklogEntryResponse>('upload_single_worklog', { token, request })
}

/**
 * Get worklogs from Tempo for a date range
 */
export async function getTempoWorklogs(token: string, request: GetWorklogsRequest): Promise<unknown[]> {
  return invoke<unknown[]>('get_tempo_worklogs', { token, request })
}

// Users Types

export interface UpdateProfileRequest {
  name?: string
  email?: string
  title?: string
  employee_id?: string
  department_id?: string
}

// Users Commands

/**
 * Get current user profile
 */
export async function getProfile(token: string): Promise<UserResponse> {
  return invoke<UserResponse>('get_profile', { token })
}

/**
 * Update user profile
 */
export async function updateProfile(token: string, request: UpdateProfileRequest): Promise<UserResponse> {
  return invoke<UserResponse>('update_profile', { token, request })
}

// Re-export for convenience
export const tauriApi = {
  // Auth
  getAppStatus,
  registerUser,
  login,
  autoLogin,
  getCurrentUser,
  // Config
  getConfig,
  updateConfig,
  updateLlmConfig,
  updateJiraConfig,
  // Work Items
  listWorkItems,
  createWorkItem,
  getWorkItem,
  updateWorkItem,
  deleteWorkItem,
  getStatsSummary,
  getGroupedWorkItems,
  getTimelineData,
  batchSyncTempo,
  aggregateWorkItems,
  // Claude
  listClaudeSessions,
  importClaudeSessions,
  summarizeClaudeSession,
  syncClaudeProjects,
  // Reports
  getPersonalReport,
  getSummaryReport,
  getCategoryReport,
  getSourceReport,
  exportExcelReport,
  // Sync
  getSyncStatus,
  autoSync,
  listAvailableProjects,
  // GitLab
  getGitLabStatus,
  configureGitLab,
  removeGitLabConfig,
  listGitLabProjects,
  addGitLabProject,
  removeGitLabProject,
  syncGitLab,
  searchGitLabProjects,
  // Tempo
  testTempoConnection,
  validateJiraIssue,
  syncWorklogsToTempo,
  uploadSingleWorklog,
  getTempoWorklogs,
  // Users
  getProfile,
  updateProfile,
}
