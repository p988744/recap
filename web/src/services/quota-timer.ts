/**
 * Quota Timer Service
 *
 * Frontend API for managing background quota polling.
 */

import { invoke } from '@tauri-apps/api/core'
import { getRequiredToken } from './client'

// ============================================================================
// Types
// ============================================================================

/** Configuration for quota polling */
export interface QuotaPollingConfig {
  enabled: boolean
  interval_minutes: number
  warning_threshold: number
  critical_threshold: number
  notify_on_threshold: boolean
  update_tray: boolean
}

/** Request to update polling configuration */
export interface UpdatePollingConfigRequest {
  enabled?: boolean
  interval_minutes?: number
  warning_threshold?: number
  critical_threshold?: number
  notify_on_threshold?: boolean
  update_tray?: boolean
}

/** Response for polling status */
export interface PollingStatusResponse {
  is_running: boolean
  is_polling: boolean
  last_poll_at: string | null
  next_poll_at: string | null
  last_error: string | null
  claude_percent: number | null
  config: QuotaPollingConfig
}

// ============================================================================
// API Functions
// ============================================================================

/**
 * Start the quota polling service
 */
export async function startQuotaPolling(): Promise<PollingStatusResponse> {
  return invoke<PollingStatusResponse>('start_quota_polling', {
    token: getRequiredToken(),
  })
}

/**
 * Stop the quota polling service
 */
export async function stopQuotaPolling(): Promise<PollingStatusResponse> {
  return invoke<PollingStatusResponse>('stop_quota_polling', {
    token: getRequiredToken(),
  })
}

/**
 * Get the current polling status
 */
export async function getQuotaPollingStatus(): Promise<PollingStatusResponse> {
  return invoke<PollingStatusResponse>('get_quota_polling_status', {
    token: getRequiredToken(),
  })
}

/**
 * Update the polling configuration
 */
export async function updateQuotaPollingConfig(
  config: UpdatePollingConfigRequest
): Promise<PollingStatusResponse> {
  return invoke<PollingStatusResponse>('update_quota_polling_config', {
    token: getRequiredToken(),
    config,
  })
}

/**
 * Trigger a manual quota poll
 */
export async function triggerQuotaPoll(): Promise<PollingStatusResponse> {
  return invoke<PollingStatusResponse>('trigger_quota_poll', {
    token: getRequiredToken(),
  })
}

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Format the next poll time for display
 */
export function formatNextPollTime(isoString: string | null): string {
  if (!isoString) return '—'

  try {
    const date = new Date(isoString)
    const now = new Date()
    const diffMs = date.getTime() - now.getTime()

    if (diffMs < 0) return '即將進行'

    const diffMins = Math.round(diffMs / 60000)
    if (diffMins < 1) return '不到 1 分鐘'
    if (diffMins === 1) return '1 分鐘後'
    if (diffMins < 60) return `${diffMins} 分鐘後`

    const diffHours = Math.round(diffMins / 60)
    if (diffHours === 1) return '1 小時後'
    return `${diffHours} 小時後`
  } catch {
    return '—'
  }
}

/**
 * Format the last poll time for display
 */
export function formatLastPollTime(isoString: string | null): string {
  if (!isoString) return '從未'

  try {
    const date = new Date(isoString)
    const now = new Date()
    const diffMs = now.getTime() - date.getTime()

    if (diffMs < 0) return '剛剛'

    const diffMins = Math.round(diffMs / 60000)
    if (diffMins < 1) return '剛剛'
    if (diffMins === 1) return '1 分鐘前'
    if (diffMins < 60) return `${diffMins} 分鐘前`

    const diffHours = Math.round(diffMins / 60)
    if (diffHours === 1) return '1 小時前'
    if (diffHours < 24) return `${diffHours} 小時前`

    const diffDays = Math.round(diffHours / 24)
    if (diffDays === 1) return '1 天前'
    return `${diffDays} 天前`
  } catch {
    return '—'
  }
}

/**
 * Get available interval options
 */
export function getIntervalOptions(): Array<{ value: number; label: string }> {
  return [
    { value: 5, label: '5 分鐘' },
    { value: 10, label: '10 分鐘' },
    { value: 15, label: '15 分鐘' },
    { value: 30, label: '30 分鐘' },
    { value: 60, label: '1 小時' },
  ]
}
