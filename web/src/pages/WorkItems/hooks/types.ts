// =============================================================================
// WorkItems Page Types
// =============================================================================

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
  startTime: string
  endTime: string
  hours: number
  commits: TimelineCommit[]
}

export interface ProjectGroupLog {
  id: string
  title: string
  description?: string
  hours: number
  date: string
  source: string
  synced_to_tempo: boolean
}

export interface ProjectGroupIssue {
  jira_key?: string
  jira_title?: string
  total_hours: number
  logs: ProjectGroupLog[]
}

export interface ProjectGroup {
  project_name: string
  total_hours: number
  issues: ProjectGroupIssue[]
}
