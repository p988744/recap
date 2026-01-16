/**
 * GitLab integration service
 */

import { invokeAuth } from '../client'
import type {
  GitLabConfigStatus,
  ConfigureGitLabRequest,
  GitLabProject,
  AddGitLabProjectRequest,
  SyncGitLabRequest,
  SyncGitLabResponse,
  SearchGitLabProjectsRequest,
  GitLabProjectInfo,
} from '@/types'

/**
 * Get GitLab configuration status
 */
export async function getStatus(): Promise<GitLabConfigStatus> {
  return invokeAuth<GitLabConfigStatus>('get_gitlab_status')
}

/**
 * Configure GitLab
 */
export async function configure(request: ConfigureGitLabRequest): Promise<{ message: string }> {
  return invokeAuth<{ message: string }>('configure_gitlab', { request })
}

/**
 * Remove GitLab configuration
 */
export async function removeConfig(): Promise<{ message: string }> {
  return invokeAuth<{ message: string }>('remove_gitlab_config')
}

/**
 * List user's tracked GitLab projects
 */
export async function listProjects(): Promise<GitLabProject[]> {
  return invokeAuth<GitLabProject[]>('list_gitlab_projects')
}

/**
 * Add a GitLab project to track
 */
export async function addProject(request: AddGitLabProjectRequest): Promise<GitLabProject> {
  return invokeAuth<GitLabProject>('add_gitlab_project', { request })
}

/**
 * Remove a GitLab project from tracking
 */
export async function removeProject(id: string): Promise<{ message: string }> {
  return invokeAuth<{ message: string }>('remove_gitlab_project', { id })
}

/**
 * Sync GitLab data to work items
 */
export async function sync(request: SyncGitLabRequest = {}): Promise<SyncGitLabResponse> {
  return invokeAuth<SyncGitLabResponse>('sync_gitlab', { request })
}

/**
 * Search GitLab projects
 */
export async function searchProjects(request: SearchGitLabProjectsRequest = {}): Promise<GitLabProjectInfo[]> {
  return invokeAuth<GitLabProjectInfo[]>('search_gitlab_projects', { request })
}
