/**
 * Projects service
 */

import { invokeAuth } from './client'
import type {
  ProjectInfo,
  ProjectDetail,
  ProjectDirectories,
  SetProjectVisibilityRequest,
  AddManualProjectRequest,
  ClaudeSessionPathResponse,
  AntigravitySessionPathResponse,
} from '@/types'

/**
 * List all projects (auto-discovered from work items)
 */
export async function listProjects(): Promise<ProjectInfo[]> {
  return invokeAuth<ProjectInfo[]>('list_projects')
}

/**
 * Get detailed information about a specific project
 */
export async function getProjectDetail(projectName: string): Promise<ProjectDetail> {
  return invokeAuth<ProjectDetail>('get_project_detail', { projectName })
}

/**
 * Set project visibility (show/hide)
 */
export async function setProjectVisibility(projectName: string, hidden: boolean): Promise<string> {
  const request: SetProjectVisibilityRequest = { project_name: projectName, hidden }
  return invokeAuth<string>('set_project_visibility', { request })
}

/**
 * Get list of hidden project names
 */
export async function getHiddenProjects(): Promise<string[]> {
  return invokeAuth<string[]>('get_hidden_projects')
}

/**
 * Get project directories (Claude Code session dir + Git repo path)
 */
export async function getProjectDirectories(projectName: string): Promise<ProjectDirectories> {
  return invokeAuth<ProjectDirectories>('get_project_directories', { projectName })
}

/**
 * Get the user's Claude session path setting
 */
export async function getClaudeSessionPath(): Promise<ClaudeSessionPathResponse> {
  return invokeAuth<ClaudeSessionPathResponse>('get_claude_session_path')
}

/**
 * Update the user's Claude session path (null to reset to default)
 */
export async function updateClaudeSessionPath(path: string | null): Promise<string> {
  return invokeAuth<string>('update_claude_session_path', { path })
}

/**
 * Get the user's Antigravity (Gemini Code) session path setting
 */
export async function getAntigravitySessionPath(): Promise<AntigravitySessionPathResponse> {
  return invokeAuth<AntigravitySessionPathResponse>('get_antigravity_session_path')
}

/**
 * Update the user's Antigravity session path (null to reset to default)
 */
export async function updateAntigravitySessionPath(path: string | null): Promise<string> {
  return invokeAuth<string>('update_antigravity_session_path', { path })
}

/**
 * Add a manual project (non-Claude, requires git repo path)
 */
export async function addManualProject(request: AddManualProjectRequest): Promise<string> {
  return invokeAuth<string>('add_manual_project', { request })
}

/**
 * Remove a manually added project
 */
export async function removeManualProject(projectName: string): Promise<string> {
  return invokeAuth<string>('remove_manual_project', { projectName })
}
