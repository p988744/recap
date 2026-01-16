/**
 * Configuration related types
 */

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
