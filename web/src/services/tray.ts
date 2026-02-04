/**
 * Tray Service
 *
 * API for controlling the system tray status display.
 */

import { invoke } from '@tauri-apps/api/core'

// =============================================================================
// API Functions
// =============================================================================

/**
 * Update the tray menu's sync status display
 */
export async function updateSyncStatus(
  lastSync: string,
  isSyncing?: boolean
): Promise<void> {
  return invoke<void>('update_tray_sync_status', {
    lastSync,
    isSyncing: isSyncing ?? false,
  })
}

/**
 * Set tray to show syncing state
 */
export async function setSyncing(syncing: boolean): Promise<void> {
  return invoke<void>('set_tray_syncing', { syncing })
}

/**
 * Update tray title with quota percentages
 */
export async function updateTrayQuota(
  claudePercent?: number,
  antigravityPercent?: number
): Promise<void> {
  console.log('[tray] Updating quota:', { claudePercent, antigravityPercent })
  return invoke<void>('update_tray_quota', {
    claudePercent,
    antigravityPercent,
  })
}
