/**
 * Background Sync Service
 *
 * API for controlling the background sync service.
 */

import { invokeAuth } from './client'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

// =============================================================================
// Types
// =============================================================================

export interface BackgroundSyncConfig {
  enabled: boolean
  interval_minutes: number
  compaction_interval_hours: number
  sync_git: boolean
  sync_claude: boolean
  sync_antigravity: boolean
  sync_gitlab: boolean
  sync_jira: boolean
  auto_generate_summaries: boolean
}

export interface BackgroundSyncStatus {
  is_running: boolean
  is_syncing: boolean
  is_compacting: boolean
  last_sync_at: string | null
  last_compaction_at: string | null
  next_sync_at: string | null
  next_compaction_at: string | null
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

/** Progress event for sync operations */
export interface SyncProgress {
  phase: 'sources' | 'snapshots' | 'compaction' | 'summaries' | 'complete'
  current_source: string | null
  current: number
  total: number
  message: string
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

/**
 * Trigger an immediate sync with progress reporting.
 * Emits "sync-progress" events during the sync operation.
 *
 * @param onProgress Callback for progress updates
 */
export async function triggerSyncWithProgress(
  onProgress?: (progress: SyncProgress) => void
): Promise<TriggerSyncResponse> {
  let unlisten: UnlistenFn | undefined

  try {
    if (onProgress) {
      unlisten = await listen<SyncProgress>('sync-progress', (event) => {
        onProgress(event.payload)
      })
    }

    const result = await invokeAuth<TriggerSyncResponse>('trigger_sync_with_progress')
    return result
  } finally {
    if (unlisten) {
      unlisten()
    }
  }
}
