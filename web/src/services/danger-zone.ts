/**
 * Danger Zone service - destructive operations that require explicit confirmation
 */

import { invokeAuth } from './client'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

/** Result of a dangerous operation */
export interface DangerousOperationResult {
  success: boolean
  message: string
  details?: {
    work_items_deleted?: number
    snapshots_deleted?: number
    summaries_deleted?: number
    configs_reset?: boolean
  }
}

/** Progress event for recompaction */
export interface RecompactProgress {
  phase: 'counting' | 'scanning' | 'hourly' | 'daily' | 'monthly' | 'complete'
  current: number
  total: number
  message: string
}

/**
 * Clear all synced data (work_items from sync sources, snapshots, summaries)
 * but keep manual work items and user settings.
 *
 * @param confirmation Must be exactly "DELETE_SYNCED_DATA" to proceed
 */
export async function clearSyncedData(confirmation: string): Promise<DangerousOperationResult> {
  return invokeAuth<DangerousOperationResult>('clear_synced_data', { confirmation })
}

/**
 * Clear ALL data and reset all settings to defaults.
 * This is a complete factory reset for the user's account.
 *
 * @param confirmation Must be exactly "FACTORY_RESET" to proceed
 */
export async function factoryReset(confirmation: string): Promise<DangerousOperationResult> {
  return invokeAuth<DangerousOperationResult>('factory_reset', { confirmation })
}

/**
 * Force recompact all summaries with progress reporting.
 *
 * @param confirmation Must be exactly "RECOMPACT" to proceed
 * @param onProgress Callback for progress updates
 */
export async function forceRecompactWithProgress(
  confirmation: string,
  onProgress?: (progress: RecompactProgress) => void
): Promise<DangerousOperationResult> {
  let unlisten: UnlistenFn | undefined

  try {
    // Set up progress listener before starting the operation
    if (onProgress) {
      unlisten = await listen<RecompactProgress>('recompact-progress', (event) => {
        onProgress(event.payload)
      })
    }

    // Execute the recompact operation
    const result = await invokeAuth<DangerousOperationResult>('force_recompact_with_progress', {
      confirmation,
    })

    return result
  } finally {
    // Clean up listener
    if (unlisten) {
      unlisten()
    }
  }
}
