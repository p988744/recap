/**
 * Background Sync Service
 *
 * API for controlling the background sync service.
 */

import { invokeAuth } from './client'

// =============================================================================
// Types
// =============================================================================

export interface BackgroundSyncConfig {
  enabled: boolean
  interval_minutes: number
  sync_git: boolean
  sync_claude: boolean
  sync_gitlab: boolean
  sync_jira: boolean
}

export interface BackgroundSyncStatus {
  is_running: boolean
  is_syncing: boolean
  last_sync_at: string | null
  next_sync_at: string | null
  last_result: string | null
  last_error: string | null
}

export interface SyncResult {
  source: string
  success: boolean
  items_synced: number
  projects_scanned: number
  items_created: number
  error: string | null
}

export interface TriggerSyncResponse {
  results: SyncResult[]
  total_items: number
}

export type UpdateConfigRequest = Partial<BackgroundSyncConfig>

// =============================================================================
// API Functions
// =============================================================================

/**
 * Get the current background sync configuration
 */
export async function getConfig(): Promise<BackgroundSyncConfig> {
  return invokeAuth<BackgroundSyncConfig>('get_background_sync_config')
}

/**
 * Update the background sync configuration
 */
export async function updateConfig(config: UpdateConfigRequest): Promise<BackgroundSyncConfig> {
  return invokeAuth<BackgroundSyncConfig>('update_background_sync_config', { config })
}

/**
 * Get the current background sync status
 */
export async function getStatus(): Promise<BackgroundSyncStatus> {
  return invokeAuth<BackgroundSyncStatus>('get_background_sync_status')
}

/**
 * Start the background sync service
 */
export async function start(): Promise<void> {
  return invokeAuth<void>('start_background_sync')
}

/**
 * Stop the background sync service
 */
export async function stop(): Promise<void> {
  return invokeAuth<void>('stop_background_sync')
}

/**
 * Trigger an immediate sync
 */
export async function triggerSync(): Promise<TriggerSyncResponse> {
  return invokeAuth<TriggerSyncResponse>('trigger_background_sync')
}
