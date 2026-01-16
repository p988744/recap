/**
 * Sync service
 */

import { invokeAuth } from './client'
import type {
  SyncStatus,
  AutoSyncRequest,
  AutoSyncResponse,
  AvailableProject,
} from '@/types'

/**
 * Get sync status for all sources
 */
export async function getStatus(): Promise<SyncStatus[]> {
  return invokeAuth<SyncStatus[]>('get_sync_status')
}

/**
 * Trigger auto-sync for Claude projects
 */
export async function autoSync(request: AutoSyncRequest = {}): Promise<AutoSyncResponse> {
  return invokeAuth<AutoSyncResponse>('auto_sync', { request })
}

/**
 * List available projects that can be synced
 */
export async function listAvailableProjects(): Promise<AvailableProject[]> {
  return invokeAuth<AvailableProject[]>('list_available_projects')
}
