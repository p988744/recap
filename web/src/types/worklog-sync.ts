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
