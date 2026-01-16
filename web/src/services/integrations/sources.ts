/**
 * Data sources service
 */

import { invokeAuth } from '../client'
import type {
  SourcesResponse,
  AddGitRepoResponse,
  SourceModeResponse,
} from '@/types'

/**
 * Get data sources configuration
 */
export async function getSources(): Promise<SourcesResponse> {
  return invokeAuth<SourcesResponse>('get_sources')
}

/**
 * Add a local Git repository
 */
export async function addGitRepo(path: string): Promise<AddGitRepoResponse> {
  return invokeAuth<AddGitRepoResponse>('add_git_repo', { path })
}

/**
 * Remove a local Git repository
 */
export async function removeGitRepo(repoId: string): Promise<SourceModeResponse> {
  return invokeAuth<SourceModeResponse>('remove_git_repo', { repo_id: repoId })
}

/**
 * Set data source mode (git or claude)
 */
export async function setSourceMode(mode: string): Promise<SourceModeResponse> {
  return invokeAuth<SourceModeResponse>('set_source_mode', { mode })
}
