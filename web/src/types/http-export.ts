/**
 * HTTP Export types
 */

export interface HttpExportConfig {
  id: string
  name: string
  url: string
  method: 'POST' | 'PUT' | 'PATCH'
  auth_type: 'none' | 'bearer' | 'basic' | 'header'
  auth_header_name?: string
  custom_headers?: string
  payload_template: string
  llm_prompt?: string
  batch_mode: boolean
  batch_wrapper_key: string
  enabled: boolean
  timeout_seconds: number
}

export interface SaveHttpExportConfigRequest {
  id?: string
  name: string
  url: string
  method: string
  auth_type: string
  auth_token?: string
  auth_header_name?: string
  custom_headers?: string
  payload_template: string
  llm_prompt?: string
  batch_mode: boolean
  batch_wrapper_key?: string
  timeout_seconds?: number
}

export interface InlineExportItem {
  id: string
  title: string
  description?: string
  hours: number
  date: string
  source: string
  jira_issue_key?: string
  category?: string
  project_name?: string
}

export interface HttpExportRequest {
  config_id: string
  work_item_ids: string[]
  /** Optional inline items — used when items don't exist in DB (e.g. Worklog page) */
  inline_items?: InlineExportItem[]
  dry_run: boolean
}

export interface HttpExportItemResult {
  work_item_id: string
  work_item_title: string
  status: 'success' | 'error' | 'dry_run'
  http_status?: number
  error_message?: string
  payload_preview?: string
}

export interface HttpExportResponse {
  total: number
  successful: number
  failed: number
  results: HttpExportItemResult[]
  dry_run: boolean
}

export interface ValidateTemplateResponse {
  valid: boolean
  fields_used: string[]
  sample_output?: string
  error?: string
}

export interface TestConnectionResponse {
  success: boolean
  http_status?: number
  message: string
}

export interface ExportHistoryRecord {
  work_item_id: string
  exported_at: string
}
