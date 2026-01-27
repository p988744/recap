/**
 * Worklog types - for the date-oriented worklog overview page
 */

export interface GitCommitRef {
  hash: string
  message: string
  timestamp: string
}

export interface HourlyBreakdownItem {
  hour_start: string
  hour_end: string
  summary: string
  files_modified: string[]
  git_commits: GitCommitRef[]
}

export interface ManualWorkItem {
  id: string
  title: string
  description?: string
  hours: number
  date: string
}

export interface WorklogDayProject {
  project_path: string
  project_name: string
  daily_summary?: string
  total_commits: number
  total_files: number
  total_hours: number
  has_hourly_data: boolean
}

export interface WorklogDay {
  date: string
  weekday: string
  projects: WorklogDayProject[]
  manual_items: ManualWorkItem[]
}

export interface WorklogOverviewResponse {
  days: WorklogDay[]
}
