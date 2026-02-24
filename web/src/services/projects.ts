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
  ProjectDescription,
  UpdateProjectDescriptionRequest,
  ProjectTimelineRequest,
  ProjectTimelineResponse,
  CommitDiffResponse,
  GetCommitDiffRequest,
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

/**
 * Response type for project README
 */
export interface ProjectReadmeResponse {
  content: string | null
  file_name: string | null
}

/**
 * Get the README content for a project
 */
export async function getProjectReadme(projectName: string): Promise<ProjectReadmeResponse> {
  return invokeAuth<ProjectReadmeResponse>('get_project_readme', { projectName })
}

/**
 * Get project description (goal, tech stack, etc.)
 */
export async function getProjectDescription(projectName: string): Promise<ProjectDescription | null> {
  return invokeAuth<ProjectDescription | null>('get_project_description', { projectName })
}

/**
 * Update or create project description
 */
export async function updateProjectDescription(request: UpdateProjectDescriptionRequest): Promise<string> {
  return invokeAuth<string>('update_project_description', { request })
}

/**
 * Delete project description
 */
export async function deleteProjectDescription(projectName: string): Promise<string> {
  return invokeAuth<string>('delete_project_description', { projectName })
}

/**
 * Get project timeline with sessions and commits grouped by time period
 */
export async function getProjectTimeline(
  request: ProjectTimelineRequest
): Promise<ProjectTimelineResponse> {
  return invokeAuth<ProjectTimelineResponse>('get_project_timeline', { request })
}

// ============ Timeline Summary API ============

/**
 * Get cached summaries in batch (for timeline view)
 * Queries both work_summaries (compaction) and project_summaries (legacy)
 */
export async function getCachedSummariesBatch(
  projectName: string,
  summaryType: 'report' | 'timeline',
  timeUnit: string,
  periodStarts: string[]
): Promise<Record<string, string>> {
  return invokeAuth<Record<string, string>>('get_cached_summaries_batch', {
    projectName,
    summaryType,
    timeUnit,
    periodStarts,
  })
}

/**
 * Get the diff for a specific commit
 */
export async function getCommitDiff(
  projectPath: string,
  commitHash: string
): Promise<CommitDiffResponse> {
  const request: GetCommitDiffRequest = {
    project_path: projectPath,
    commit_hash: commitHash,
  }
  return invokeAuth<CommitDiffResponse>('get_commit_diff', { request })
}
