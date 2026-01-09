// API Types
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

export interface UpdateLlmConfigRequest {
  provider: string  // "openai", "anthropic", "ollama", "openai-compatible"
  model: string
  api_key?: string
  base_url?: string
}

export interface GitRepoInfo {
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
  outlook_enabled: boolean
}

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

// API Client - Hybrid mode: uses Tauri Commands when available, falls back to HTTP
import * as tauriApi from './tauri-api'

const API_BASE = '/api'
const TOKEN_KEY = 'recap_auth_token'

function getAuthToken(): string | null {
  return localStorage.getItem(TOKEN_KEY)
}

function getRequiredToken(): string {
  const token = getAuthToken()
  if (!token) {
    window.location.href = '/login'
    throw new Error('No auth token')
  }
  return token
}

// Check if running in Tauri
const isTauri = typeof window !== 'undefined' && '__TAURI__' in window

async function fetchApi<T>(endpoint: string, options?: RequestInit): Promise<T> {
  const token = getAuthToken()
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...(options?.headers as Record<string, string>),
  }

  if (token) {
    headers['Authorization'] = `Bearer ${token}`
  }

  const response = await fetch(`${API_BASE}${endpoint}`, {
    ...options,
    headers,
  })

  if (!response.ok) {
    // Handle 401 Unauthorized - redirect to login
    if (response.status === 401) {
      localStorage.removeItem(TOKEN_KEY)
      window.location.href = '/login'
      throw new Error('Session expired. Please login again.')
    }

    const error = await response.json().catch(() => ({ detail: 'Unknown error' }))
    throw new Error(error.detail || error.error || 'API request failed')
  }

  return response.json()
}

// Work Item types
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
  tags?: string[]
  yearly_goal_id?: string
  synced_to_tempo: boolean
  tempo_worklog_id?: string
  synced_at?: string
  created_at: string
  updated_at: string
  parent_id?: string      // For grouped items: links to parent
  child_count: number     // Number of child items (for aggregated parents)
}

export interface WorkItemListResponse {
  items: WorkItem[]
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
  search?: string
  parent_id?: string      // Get children of a specific parent
  show_all?: boolean      // Show all items including children
}

export interface WorkItemCreate {
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

export interface WorkItemUpdate {
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

export interface DailyHours {
  date: string
  hours: number
  count: number
}

export interface WorkItemStats {
  total_items: number
  total_hours: number
  hours_by_source: Record<string, number>
  hours_by_project: Record<string, number>
  hours_by_category: Record<string, number>
  daily_hours: DailyHours[]
  jira_mapping: {
    mapped: number
    unmapped: number
    percentage: number
  }
  tempo_sync: {
    synced: number
    not_synced: number
    percentage: number
  }
}

// Grouped view types
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

// Report types
export interface WorkItemSummary {
  id: string
  title: string
  hours: number
  date: string
  jira_issue_key?: string
  category?: string
  source: string
}

export interface DailyReport {
  date: string
  total_hours: number
  items: WorkItemSummary[]
}

export interface PersonalReport {
  user_name: string
  user_email: string
  start_date: string
  end_date: string
  total_hours: number
  work_items: WorkItemSummary[]
  daily_breakdown: DailyReport[]
  category_breakdown: Record<string, number>
  jira_issues: Record<string, number>
  source_breakdown: Record<string, number>
}

export interface WeeklyReport {
  start_date: string
  end_date: string
  total_hours: number
  daily_breakdown: DailyReport[]
  category_breakdown: Record<string, number>
  jira_issues: Record<string, number>
}

export interface TeamMemberSummary {
  user_id: string
  user_name: string
  total_hours: number
  work_item_count: number
  category_breakdown: Record<string, number>
}

export interface TeamReport {
  department_name: string
  start_date: string
  end_date: string
  total_hours: number
  member_count: number
  members: TeamMemberSummary[]
  category_breakdown: Record<string, number>
}

export interface PEWorkResult {
  title: string
  period: string
  result_description: string
  weight: number
}

export interface GoalProgress {
  goal_id: string
  goal_title: string
  category: string
  weight: number
  work_item_count: number
  total_hours: number
  work_items: WorkItemSummary[]
}

export interface PEReport {
  user_name: string
  department?: string
  title?: string
  evaluation_period: string
  work_results: PEWorkResult[]
  skills: Array<{ name: string; description: string }>
  goal_progress: GoalProgress[]
  total_hours: number
  jira_issues_count: number
  commits_count: number
  merge_requests_count: number
}

// Timeline types for Gantt chart
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

// Claude Code types
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
}

export interface ClaudeProject {
  path: string
  name: string
  sessions: ClaudeSession[]
}

// Sync types
export interface SyncStatus {
  id: string
  source: string
  source_path?: string
  last_sync_at?: string
  last_item_count: number
  status: 'idle' | 'syncing' | 'error' | 'success'
  error_message?: string
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

export const api = {
  // Health
  health: () => fetchApi<{ status: string; version: string }>('/health'),

  // User Profile
  updateProfile: (data: { name?: string; email?: string; title?: string }) =>
    fetchApi<{ id: string; name: string; email: string; title?: string }>('/users/profile', {
      method: 'PATCH',
      body: JSON.stringify(data),
    }),

  // Config - Use Tauri API when available
  getConfig: async () => {
    if (isTauri) {
      return tauriApi.getConfig(getRequiredToken())
    }
    return fetchApi<ConfigResponse>('/config')
  },
  updateConfig: async (data: Partial<ConfigResponse>) => {
    if (isTauri) {
      return tauriApi.updateConfig(getRequiredToken(), {
        daily_work_hours: data.daily_work_hours,
        normalize_hours: data.normalize_hours,
      })
    }
    return fetchApi<{ message: string }>('/config', {
      method: 'PATCH',
      body: JSON.stringify(data),
    })
  },
  updateLlmConfig: async (data: UpdateLlmConfigRequest) => {
    if (isTauri) {
      await tauriApi.updateLlmConfig(getRequiredToken(), data)
      return { message: 'LLM configuration updated', provider: data.provider, model: data.model }
    }
    return fetchApi<{ message: string; provider: string; model: string }>('/config/llm', {
      method: 'PATCH',
      body: JSON.stringify(data),
    })
  },
  updateJiraConfig: async (data: {
    jira_url?: string
    jira_pat?: string
    jira_email?: string
    jira_api_token?: string
    auth_type?: string
    tempo_api_token?: string
  }) => {
    if (isTauri) {
      await tauriApi.updateJiraConfig(getRequiredToken(), data)
      return { success: true, message: 'Jira configuration updated' }
    }
    return fetchApi<{ success: boolean; message: string }>('/config/jira', {
      method: 'POST',
      body: JSON.stringify(data),
    })
  },
  testJira: () => fetchApi<{ success: boolean; message: string }>('/config/test-jira'),
  getTeams: () => fetchApi<{ teams: Team[]; total: number }>('/config/teams'),

  // Sources
  getSources: () => fetchApi<SourcesResponse>('/sources'),
  addGitRepo: (path: string) =>
    fetchApi<{ success: boolean; message: string }>('/sources/git', {
      method: 'POST',
      body: JSON.stringify({ path }),
    }),
  removeGitRepo: (name: string) =>
    fetchApi<{ success: boolean; message: string }>(`/sources/git/${name}`, {
      method: 'DELETE',
    }),
  setGitMode: () =>
    fetchApi<{ success: boolean; message: string }>('/sources/mode/git', { method: 'POST' }),
  setClaudeMode: () =>
    fetchApi<{ success: boolean; message: string }>('/sources/mode/claude', { method: 'POST' }),

  // GitLab
  configureGitLab: (gitlabUrl: string, gitlabPat: string) =>
    fetchApi<{ message: string }>('/gitlab/config', {
      method: 'POST',
      body: JSON.stringify({ gitlab_url: gitlabUrl, gitlab_pat: gitlabPat }),
    }),
  getGitLabStatus: () =>
    fetchApi<{ configured: boolean; gitlab_url: string | null }>('/gitlab/config/status'),
  removeGitLabConfig: () =>
    fetchApi<{ message: string }>('/gitlab/config', { method: 'DELETE' }),
  getGitLabRemoteProjects: (search?: string, page = 1, perPage = 20) => {
    const params = new URLSearchParams()
    if (search) params.append('search', search)
    params.append('page', String(page))
    params.append('per_page', String(perPage))
    return fetchApi<Array<{
      id: number
      name: string
      path_with_namespace: string
      description: string | null
      web_url: string
      default_branch: string
      last_activity_at: string
    }>>(`/gitlab/remote-projects?${params}`)
  },
  getGitLabTrackedProjects: () =>
    fetchApi<Array<{
      id: string
      gitlab_project_id: number
      name: string
      path_with_namespace: string
      gitlab_url: string
      default_branch: string
      enabled: boolean
      last_synced: string | null
      created_at: string
    }>>('/gitlab/projects'),
  addGitLabProject: (gitlabProjectId: number) =>
    fetchApi<{
      id: string
      gitlab_project_id: number
      name: string
      path_with_namespace: string
      gitlab_url: string
      default_branch: string
      enabled: boolean
      last_synced: string | null
      created_at: string
    }>('/gitlab/projects', {
      method: 'POST',
      body: JSON.stringify({ gitlab_project_id: gitlabProjectId }),
    }),
  removeGitLabProject: (projectId: string) =>
    fetchApi<{ message: string }>(`/gitlab/projects/${projectId}`, { method: 'DELETE' }),
  syncGitLabProject: (projectId: string, since?: string, until?: string) => {
    const params = new URLSearchParams()
    if (since) params.append('since', since)
    if (until) params.append('until', until)
    const query = params.toString()
    return fetchApi<{
      project_id: string
      commits_synced: number
      merge_requests_synced: number
      work_items_created: number
    }>(`/gitlab/projects/${projectId}/sync${query ? `?${query}` : ''}`, { method: 'POST' })
  },
  syncAllGitLabProjects: (since?: string, until?: string) => {
    const params = new URLSearchParams()
    if (since) params.append('since', since)
    if (until) params.append('until', until)
    const query = params.toString()
    return fetchApi<Array<{
      project_id: string
      commits_synced: number
      merge_requests_synced: number
      work_items_created: number
    }>>(`/gitlab/sync-all${query ? `?${query}` : ''}`, { method: 'POST' })
  },

  // Analyze
  analyzeWeek: (useGit?: boolean) => {
    const params = useGit !== undefined ? `?use_git=${useGit}` : ''
    return fetchApi<AnalyzeResponse>(`/analyze/week${params}`)
  },
  analyzeLastWeek: (useGit?: boolean) => {
    const params = useGit !== undefined ? `?use_git=${useGit}` : ''
    return fetchApi<AnalyzeResponse>(`/analyze/last-week${params}`)
  },
  analyzeDays: (days: number, useGit?: boolean) => {
    const params = useGit !== undefined ? `?use_git=${useGit}` : ''
    return fetchApi<AnalyzeResponse>(`/analyze/days/${days}${params}`)
  },
  analyzeRange: (startDate: string, endDate: string, useGit?: boolean) =>
    fetchApi<AnalyzeResponse>('/analyze', {
      method: 'POST',
      body: JSON.stringify({ start_date: startDate, end_date: endDate, use_git: useGit }),
    }),
  getAvailableDates: (limit?: number) => {
    const params = limit ? `?limit=${limit}` : ''
    return fetchApi<string[]>(`/analyze/dates${params}`)
  },

  // Tempo
  testTempo: () => fetchApi<{ success: boolean; message: string }>('/tempo/test'),
  validateIssue: (issueKey: string) =>
    fetchApi<{ success: boolean; message: string }>(`/tempo/validate-issue/${issueKey}`, {
      method: 'POST',
    }),
  syncWorklogs: (entries: Array<{ issue_key: string; date: string; minutes: number; description: string }>, dryRun = false) =>
    fetchApi<{
      success: boolean
      total_entries: number
      successful: number
      failed: number
      results: Array<{
        id?: string  // Tempo worklog ID
        issue_key: string
        date: string
        minutes: number
        hours: number
        description: string
        status: string
        error_message?: string
      }>
      dry_run: boolean
    }>('/tempo/sync', {
      method: 'POST',
      body: JSON.stringify({ entries, dry_run: dryRun }),
    }),

  // Work Items - Use Tauri API when available
  getWorkItems: async (filters?: WorkItemFilters) => {
    if (isTauri) {
      const result = await tauriApi.listWorkItems(getRequiredToken(), filters || {})
      return result as WorkItemListResponse
    }
    const params = new URLSearchParams()
    if (filters) {
      if (filters.page) params.append('page', String(filters.page))
      if (filters.per_page) params.append('per_page', String(filters.per_page))
      if (filters.source) params.append('source', filters.source)
      if (filters.category) params.append('category', filters.category)
      if (filters.jira_mapped !== undefined) params.append('jira_mapped', String(filters.jira_mapped))
      if (filters.synced_to_tempo !== undefined) params.append('synced_to_tempo', String(filters.synced_to_tempo))
      if (filters.start_date) params.append('start_date', filters.start_date)
      if (filters.end_date) params.append('end_date', filters.end_date)
      if (filters.search) params.append('search', filters.search)
      if (filters.parent_id) params.append('parent_id', filters.parent_id)
      if (filters.show_all) params.append('show_all', String(filters.show_all))
    }
    const query = params.toString()
    return fetchApi<WorkItemListResponse>(`/work-items${query ? `?${query}` : ''}`)
  },
  getGroupedWorkItems: async (options?: { start_date?: string; end_date?: string }) => {
    if (isTauri) {
      return tauriApi.getGroupedWorkItems(getRequiredToken(), options || {})
    }
    const params = new URLSearchParams()
    if (options?.start_date) params.append('start_date', options.start_date)
    if (options?.end_date) params.append('end_date', options.end_date)
    const query = params.toString()
    return fetchApi<GroupedWorkItemsResponse>(`/work-items/grouped${query ? `?${query}` : ''}`)
  },
  getWorkItem: async (id: string) => {
    if (isTauri) {
      return tauriApi.getWorkItem(getRequiredToken(), id) as Promise<WorkItem>
    }
    return fetchApi<WorkItem>(`/work-items/${id}`)
  },
  createWorkItem: async (data: WorkItemCreate) => {
    if (isTauri) {
      return tauriApi.createWorkItem(getRequiredToken(), data) as Promise<WorkItem>
    }
    return fetchApi<WorkItem>('/work-items', {
      method: 'POST',
      body: JSON.stringify(data),
    })
  },
  updateWorkItem: async (id: string, data: WorkItemUpdate) => {
    if (isTauri) {
      return tauriApi.updateWorkItem(getRequiredToken(), id, data) as Promise<WorkItem>
    }
    return fetchApi<WorkItem>(`/work-items/${id}`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    })
  },
  deleteWorkItem: async (id: string) => {
    if (isTauri) {
      await tauriApi.deleteWorkItem(getRequiredToken(), id)
      return { message: 'Work item deleted', count: 1 }
    }
    return fetchApi<{ message: string; count: number }>(`/work-items/${id}`, {
      method: 'DELETE',
    })
  },
  mapWorkItemJira: (id: string, jiraIssueKey: string, jiraIssueTitle?: string) =>
    fetchApi<WorkItem>(`/work-items/${id}/map-jira`, {
      method: 'POST',
      body: JSON.stringify({ jira_issue_key: jiraIssueKey, jira_issue_title: jiraIssueTitle }),
    }),
  batchMapWorkItemsJira: (workItemIds: string[], jiraIssueKey: string, jiraIssueTitle?: string) =>
    fetchApi<{ message: string; count: number }>('/work-items/batch-map-jira', {
      method: 'POST',
      body: JSON.stringify({ work_item_ids: workItemIds, jira_issue_key: jiraIssueKey, jira_issue_title: jiraIssueTitle }),
    }),
  getWorkItemStats: async (startDate?: string, endDate?: string) => {
    if (isTauri) {
      return tauriApi.getStatsSummary(getRequiredToken(), { start_date: startDate, end_date: endDate }) as Promise<WorkItemStats>
    }
    const params = new URLSearchParams()
    if (startDate) params.append('start_date', startDate)
    if (endDate) params.append('end_date', endDate)
    const query = params.toString()
    return fetchApi<WorkItemStats>(`/work-items/stats/summary${query ? `?${query}` : ''}`)
  },
  aggregateWorkItems: async (options?: { start_date?: string; end_date?: string; source?: string }) => {
    if (isTauri) {
      return tauriApi.aggregateWorkItems(getRequiredToken(), options || {})
    }
    return fetchApi<{ original_count: number; aggregated_count: number; deleted_count: number }>('/work-items/aggregate', {
      method: 'POST',
      body: JSON.stringify(options || {}),
    })
  },

  // Reports - Use Tauri API when available
  getPersonalReport: async (startDate: string, endDate: string) => {
    if (isTauri) {
      const result = await tauriApi.getPersonalReport(getRequiredToken(), { start_date: startDate, end_date: endDate })
      // Transform to match PersonalReport interface
      return {
        user_name: '',  // Will be filled by the component or from another source
        user_email: '',
        start_date: result.start_date,
        end_date: result.end_date,
        total_hours: result.total_hours,
        work_items: result.work_items.map(item => ({
          id: item.id,
          title: item.title,
          hours: item.hours,
          date: item.date,
          jira_issue_key: item.jira_issue_key,
          category: item.category,
          source: item.source,
        })),
        daily_breakdown: result.items_by_date.map(d => ({
          date: d.date,
          total_hours: d.hours,
          items: [],  // Not available in the Tauri response
        })),
        category_breakdown: {},  // Can be calculated from work_items if needed
        jira_issues: {},
        source_breakdown: {},
      } as PersonalReport
    }
    return fetchApi<PersonalReport>(`/reports/personal?start_date=${startDate}&end_date=${endDate}`)
  },
  getWeeklyReport: (weekStart?: string) => {
    const params = weekStart ? `?week_start=${weekStart}` : ''
    return fetchApi<WeeklyReport>(`/reports/weekly${params}`)
  },
  getTeamReport: (startDate: string, endDate: string) =>
    fetchApi<TeamReport>(`/reports/team?start_date=${startDate}&end_date=${endDate}`),
  getPEReport: (year: number, half: 1 | 2) =>
    fetchApi<PEReport>(`/reports/pe?year=${year}&half=${half}`),
  exportMarkdownReport: async (startDate: string, endDate: string) => {
    const token = localStorage.getItem('recap_auth_token')
    const response = await fetch(`/api/reports/export/markdown?start_date=${startDate}&end_date=${endDate}`, {
      headers: token ? { Authorization: `Bearer ${token}` } : {},
    })
    if (!response.ok) throw new Error('Export failed')
    return response.text()
  },

  // Timeline for Gantt chart - Use Tauri API when available
  getTimeline: async (date: string) => {
    if (isTauri) {
      return tauriApi.getTimelineData(getRequiredToken(), date)
    }
    return fetchApi<TimelineResponse>(`/work-items/timeline?date=${date}`)
  },

  // Claude Code - Use Tauri API when available
  getClaudeSessions: async () => {
    if (isTauri) {
      return tauriApi.listClaudeSessions(getRequiredToken())
    }
    return fetchApi<ClaudeProject[]>('/claude/sessions')
  },
  importClaudeSessions: async (sessionIds: string[]) => {
    if (isTauri) {
      return tauriApi.importClaudeSessions(getRequiredToken(), { session_ids: sessionIds })
    }
    return fetchApi<{ imported: number; work_items_created: number }>('/claude/sessions/import', {
      method: 'POST',
      body: JSON.stringify({ session_ids: sessionIds }),
    })
  },
  syncClaudeProjects: async (projectPaths: string[]) => {
    if (isTauri) {
      const result = await tauriApi.syncClaudeProjects(getRequiredToken(), { project_paths: projectPaths })
      return {
        synced: result.sessions_processed,
        skipped: result.sessions_skipped,
        work_items_created: result.work_items_created,
      }
    }
    return fetchApi<{ synced: number; skipped: number; work_items_created: number }>('/claude/sync', {
      method: 'POST',
      body: JSON.stringify({ project_paths: projectPaths }),
    })
  },

  // Sync
  getSyncStatus: () => fetchApi<SyncStatus[]>('/sync/status'),
  autoSync: (projectPaths?: string[]) =>
    fetchApi<AutoSyncResponse>('/sync/auto', {
      method: 'POST',
      body: JSON.stringify({ project_paths: projectPaths }),
    }),
  getAvailableProjects: () => fetchApi<AvailableProject[]>('/sync/projects'),

  // Tempo
  testTempoConnection: () =>
    fetchApi<{ success: boolean; message: string }>('/tempo/test'),
  validateJiraIssue: (issueKey: string) =>
    fetchApi<{ valid: boolean; issue_key: string; summary?: string; message: string }>(
      `/tempo/validate/${issueKey}`
    ),
  syncWorklogsToTempo: (entries: TempoWorklogEntry[], dryRun = false) =>
    fetchApi<TempoSyncResponse>('/tempo/sync', {
      method: 'POST',
      body: JSON.stringify({ entries, dry_run: dryRun }),
    }),
  uploadSingleWorklog: (entry: TempoWorklogEntry) =>
    fetchApi<TempoWorklogResult>('/tempo/upload', {
      method: 'POST',
      body: JSON.stringify(entry),
    }),

  // Reports - Excel Export - Use Tauri API when available
  exportExcel: async (startDate: string, endDate: string) => {
    if (isTauri) {
      const result = await tauriApi.exportExcelReport(getRequiredToken(), { start_date: startDate, end_date: endDate })
      if (!result.success || !result.file_path) {
        throw new Error(result.error || 'Failed to export Excel')
      }
      // For Tauri, we return the file path instead of a blob
      // The frontend should handle opening the file location
      return {
        filePath: result.file_path,
        filename: result.file_path.split('/').pop() || `work_report_${startDate}_${endDate}.xlsx`,
      }
    }
    const token = getAuthToken()
    const response = await fetch(
      `${API_BASE}/reports/export/excel?start_date=${startDate}&end_date=${endDate}`,
      {
        headers: token ? { Authorization: `Bearer ${token}` } : {},
      }
    )
    if (!response.ok) {
      throw new Error('Failed to export Excel')
    }
    const blob = await response.blob()
    const filename = response.headers
      .get('content-disposition')
      ?.match(/filename="(.+)"/)?.[1] || `work_report_${startDate}_${endDate}.xlsx`
    return { blob, filename }
  },
}

// Tempo types
export interface TempoWorklogEntry {
  issue_key: string
  date: string
  minutes: number
  description: string
}

export interface TempoWorklogResult {
  id?: string
  issue_key: string
  date: string
  minutes: number
  hours: number
  description: string
  status: 'success' | 'error' | 'pending'
  error_message?: string
}

export interface TempoSyncResponse {
  success: boolean
  total_entries: number
  successful: number
  failed: number
  results: TempoWorklogResult[]
  dry_run: boolean
}
