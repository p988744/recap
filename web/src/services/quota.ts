/**
 * Quota tracking service
 *
 * Provides functions for fetching and managing quota data from Claude Code
 * and Antigravity providers.
 */

import { invokeAuth } from './client'
import type { CurrentQuotaResponse, QuotaSnapshot } from '@/types/quota'

const LOG_PREFIX = '[quota]'

/**
 * Fetch current quota from provider (live fetch)
 */
export async function getCurrentQuota(): Promise<CurrentQuotaResponse> {
  console.log(`${LOG_PREFIX} Fetching current quota...`)
  try {
    const result = await invokeAuth<CurrentQuotaResponse>('get_current_quota', {})
    console.log(`${LOG_PREFIX} Current quota fetched:`, result)
    return result
  } catch (error) {
    console.error(`${LOG_PREFIX} Failed to fetch current quota:`, error)
    throw error
  }
}

/**
 * Get stored quota snapshots from database
 * @param provider Optional filter by provider
 */
export async function getStoredQuota(
  provider?: string
): Promise<QuotaSnapshot[]> {
  console.log(`${LOG_PREFIX} Getting stored quota, provider=${provider}`)
  try {
    const result = await invokeAuth<QuotaSnapshot[]>('get_stored_quota', {
      provider,
    })
    console.log(`${LOG_PREFIX} Stored quota retrieved:`, result.length, 'snapshots')
    return result
  } catch (error) {
    console.error(`${LOG_PREFIX} Failed to get stored quota:`, error)
    throw error
  }
}

/**
 * Get quota history for charting
 * @param provider Provider name (e.g., 'claude', 'antigravity')
 * @param windowType Window type (e.g., 'five_hour', 'seven_day')
 * @param days Number of days of history (default: 7)
 */
export async function getQuotaHistory(
  provider: string,
  windowType: string,
  days?: number
): Promise<QuotaSnapshot[]> {
  console.log(`${LOG_PREFIX} Getting quota history: ${provider}/${windowType}, ${days ?? 7} days`)
  try {
    const result = await invokeAuth<QuotaSnapshot[]>('get_quota_history', {
      provider,
      window_type: windowType,
      days,
    })
    console.log(`${LOG_PREFIX} History retrieved:`, result.length, 'points')
    return result
  } catch (error) {
    console.error(`${LOG_PREFIX} Failed to get quota history:`, error)
    throw error
  }
}

/**
 * Check if a quota provider is available (has OAuth token)
 * @param provider Provider name to check
 */
export async function checkProviderAvailable(
  provider: string
): Promise<boolean> {
  console.log(`${LOG_PREFIX} Checking provider availability: ${provider}`)
  try {
    const result = await invokeAuth<boolean>('check_quota_provider_available', {
      provider,
    })
    console.log(`${LOG_PREFIX} Provider ${provider} available: ${result}`)
    return result
  } catch (error) {
    console.error(`${LOG_PREFIX} Failed to check provider:`, error)
    return false
  }
}
