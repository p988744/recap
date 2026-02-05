/**
 * Quota tracking service
 *
 * Provides functions for fetching and managing quota data from Claude Code
 * and Antigravity providers.
 */

import { invokeAuth } from './client'
import type { CurrentQuotaResponse, QuotaSnapshot, ClaudeAuthStatus, CostSummary } from '@/types/quota'

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
 * @param windowType Window type (e.g., '5_hour', '7_day')
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

// ============================================================================
// Claude OAuth Token Management (Fallback)
// ============================================================================

/**
 * Get the manually configured Claude OAuth token
 * @returns The token if set, or null if not configured
 */
export async function getClaudeOAuthToken(): Promise<string | null> {
  console.log(`${LOG_PREFIX} Getting Claude OAuth token...`)
  try {
    const result = await invokeAuth<string | null>('get_claude_oauth_token', {})
    console.log(`${LOG_PREFIX} Claude OAuth token ${result ? 'found' : 'not configured'}`)
    return result
  } catch (error) {
    console.error(`${LOG_PREFIX} Failed to get Claude OAuth token:`, error)
    throw error
  }
}

/**
 * Set the Claude OAuth token manually
 * This is used as a fallback when automatic credential discovery fails.
 * @param oauthToken The OAuth token to set, or null/empty to clear
 */
export async function setClaudeOAuthToken(oauthToken: string | null): Promise<void> {
  console.log(`${LOG_PREFIX} Setting Claude OAuth token...`)
  try {
    await invokeAuth<void>('set_claude_oauth_token', {
      oauth_token: oauthToken || null,
    })
    console.log(`${LOG_PREFIX} Claude OAuth token ${oauthToken ? 'set' : 'cleared'}`)
  } catch (error) {
    console.error(`${LOG_PREFIX} Failed to set Claude OAuth token:`, error)
    throw error
  }
}

/**
 * Check Claude auth status (automatic vs manual credentials)
 * @returns Status object indicating which auth source is available/active
 */
export async function checkClaudeAuthStatus(): Promise<ClaudeAuthStatus> {
  console.log(`${LOG_PREFIX} Checking Claude auth status...`)
  try {
    const result = await invokeAuth<ClaudeAuthStatus>('check_claude_auth_status', {})
    console.log(`${LOG_PREFIX} Claude auth status:`, result)
    return result
  } catch (error) {
    console.error(`${LOG_PREFIX} Failed to check Claude auth status:`, error)
    throw error
  }
}

// ============================================================================
// Cost Calculation (from local JSONL files)
// ============================================================================

/**
 * Get cost summary from local Claude Code JSONL files.
 * Calculates token usage and costs without API calls.
 * @param days Number of days to calculate (default: 30)
 * @returns Cost summary including daily and per-model breakdown
 */
export async function getCostSummary(days?: number): Promise<CostSummary> {
  console.log(`${LOG_PREFIX} Calculating cost summary for ${days ?? 30} days...`)
  try {
    const result = await invokeAuth<CostSummary>('get_cost_summary', {
      days,
    })
    console.log(`${LOG_PREFIX} Cost summary: today=$${result.today_cost.toFixed(2)}, 30d=$${result.last_30_days_cost.toFixed(2)}`)
    return result
  } catch (error) {
    console.error(`${LOG_PREFIX} Failed to get cost summary:`, error)
    throw error
  }
}
