/**
 * Project types
 */

export interface ProjectInfo {
  project_name: string
  project_path: string | null
  source: string
  work_item_count: number
  total_hours: number
  latest_date: string | null
  hidden: boolean
  display_name: string | null
}

export interface ProjectSourceInfo {
  source: string
  item_count: number
  latest_date: string | null
  project_path: string | null
}

export interface ProjectWorkItemSummary {
  id: string
  title: string
  date: string
  hours: number
  source: string
}

export interface ProjectStats {
  total_items: number
  total_hours: number
  date_range: [string, string] | null
}

export interface ProjectDetail {
  project_name: string
  project_path: string | null
  hidden: boolean
  display_name: string | null
  sources: ProjectSourceInfo[]
  recent_items: ProjectWorkItemSummary[]
  stats: ProjectStats
}

export interface SetProjectVisibilityRequest {
  project_name: string
  hidden: boolean
}

export interface ClaudeCodeDirEntry {
  path: string
  session_count: number
}

export interface ProjectDirectories {
  claude_code_dirs: ClaudeCodeDirEntry[]
  git_repo_path: string | null
}

export interface AddManualProjectRequest {
  project_name: string
  git_repo_path: string
  display_name?: string | null
}

export interface ClaudeSessionPathResponse {
  path: string
  is_default: boolean
}
