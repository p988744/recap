/**
 * Reports related types
 */

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
  work_items: import('./work-items').WorkItem[]
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

// Tempo Report types

export type TempoReportPeriod = 'daily' | 'weekly' | 'monthly' | 'quarterly' | 'semi_annual'

export interface TempoReportQuery {
  period: TempoReportPeriod
  date?: string
}

export interface TempoProjectSummary {
  project: string
  hours: number
  item_count: number
  summaries: string[]
}

export interface TempoReport {
  period: string
  start_date: string
  end_date: string
  total_hours: number
  total_items: number
  projects: TempoProjectSummary[]
  used_llm: boolean
}

// Legacy report types (from api.ts)

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

export interface LegacyPersonalReport {
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
