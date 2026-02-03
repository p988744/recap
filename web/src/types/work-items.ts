/**
 * Work Items related types
 */

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
  project_path?: string
  project_name?: string
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
  search?: string
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
  project_name?: string
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
  project_name?: string
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

// Stats types

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

// Timeline types

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

// Batch operations

export interface BatchSyncRequest {
  work_item_ids: string[]
}

export interface BatchSyncResponse {
  synced: number
  failed: number
  errors: string[]
}

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

// Commit-centric worklog

export interface CommitWorklogItem {
  commit_hash: string
  commit_message: string
  commit_time: string
  project_name: string
  hours: number
  jira_issue_key?: string
  jira_issue_title?: string
  synced_to_tempo: boolean
  work_item_id?: string
}

export interface CommitCentricWorklogResponse {
  items: CommitWorklogItem[]
  total_hours: number
  total_commits: number
}
