/**
 * Notification Service
 *
 * API for sending system notifications.
 */

import { invoke } from '@tauri-apps/api/core'

// =============================================================================
// API Functions
// =============================================================================

/**
 * Send a sync notification
 * @param success - Whether the sync was successful
 * @param message - The notification message
 */
export async function sendSyncNotification(
  success: boolean,
  message: string
): Promise<void> {
  return invoke<void>('send_sync_notification', { success, message })
}

/**
 * Send an auth required notification
 * @param message - The notification message
 */
export async function sendAuthNotification(message: string): Promise<void> {
  return invoke<void>('send_auth_notification', { message })
}

/**
 * Send a source error notification
 * @param source - The source name (e.g., "GitLab", "Jira")
 * @param error - The error message
 */
export async function sendSourceErrorNotification(
  source: string,
  error: string
): Promise<void> {
  return invoke<void>('send_source_error_notification', { source, error })
}
