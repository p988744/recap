/**
 * Quota tracking types
 *
 * Types for tracking Claude Code and Antigravity quota usage.
 */

// Quota provider type
export type QuotaProvider = 'claude' | 'antigravity'

// Window type for quota tracking (matches database format)
export type QuotaWindowType =
  | '5_hour'
  | '7_day'
  | '7_day_opus'
  | '7_day_sonnet'
  | 'monthly'

// A single quota snapshot
export interface QuotaSnapshot {
  provider: QuotaProvider
  model: string | null
  window_type: QuotaWindowType
  used_percent: number
  resets_at: string | null
  extra_credits_used: number | null
  extra_credits_limit: number | null
  fetched_at: string
}

// Response from get_current_quota command
export interface CurrentQuotaResponse {
  snapshots: QuotaSnapshot[]
  provider_available: boolean
}

// Quota settings configuration
export interface QuotaSettings {
  interval_minutes: number
  warning_threshold: number
  critical_threshold: number
  notifications_enabled: boolean
}

// Alert level based on usage
export type AlertLevel = 'normal' | 'warning' | 'critical'

// Claude auth status response
export interface ClaudeAuthStatus {
  /** Whether automatic credential discovery works (Keychain/file) */
  auto_available: boolean
  /** Whether a manual token is configured */
  manual_configured: boolean
  /** Whether the manual token is valid (if configured) */
  manual_valid: boolean
  /** Which auth source is active: "auto", "manual", or "none" */
  active_source: 'auto' | 'manual' | 'none'
}

/**
 * Get alert level based on usage percentage and settings
 */
export function getAlertLevel(
  usedPercent: number,
  settings: QuotaSettings
): AlertLevel {
  if (usedPercent >= settings.critical_threshold) return 'critical'
  if (usedPercent >= settings.warning_threshold) return 'warning'
  return 'normal'
}

/**
 * Format window type for display
 */
export function formatWindowType(windowType: QuotaWindowType): string {
  switch (windowType) {
    case '5_hour':
      return '5小時'
    case '7_day':
      return '本週'
    case '7_day_opus':
      return 'Opus'
    case '7_day_sonnet':
      return 'Sonnet'
    case 'monthly':
      return 'Monthly'
    default:
      return windowType
  }
}

/**
 * Format reset time as relative time string
 */
export function formatResetTime(resetsAt: string | null): string {
  if (!resetsAt) return '-'
  const resetDate = new Date(resetsAt)
  const now = new Date()
  const diffMs = resetDate.getTime() - now.getTime()
  if (diffMs <= 0) return 'Now'
  const diffMins = Math.floor(diffMs / 60000)
  const hours = Math.floor(diffMins / 60)
  const mins = diffMins % 60
  if (hours > 0) return `${hours}h ${mins}m`
  return `${mins}m`
}

// ============================================================================
// Cost Calculation Types
// ============================================================================

/**
 * Daily usage summary
 */
export interface DailyUsage {
  /** Date in YYYY-MM-DD format */
  date: string
  /** Total tokens for this day */
  total_tokens: number
  /** Total cost for this day (USD) */
  total_cost: number
}

/**
 * Per-model usage breakdown
 */
export interface ModelUsage {
  /** Model name (e.g., "claude-opus-4-5-20251101") */
  model: string
  /** Input tokens */
  input_tokens: number
  /** Output tokens */
  output_tokens: number
  /** Cache creation tokens */
  cache_creation_tokens: number
  /** Cache read tokens */
  cache_read_tokens: number
  /** Total cost for this model (USD) */
  total_cost: number
}

/**
 * Cost summary from local JSONL files
 */
export interface CostSummary {
  /** Total cost for today (USD) */
  today_cost: number
  /** Total tokens for today */
  today_tokens: number
  /** Total cost for the last 30 days (USD) */
  last_30_days_cost: number
  /** Total tokens for the last 30 days */
  last_30_days_tokens: number
  /** Daily usage breakdown */
  daily_usage: DailyUsage[]
  /** Per-model breakdown */
  model_breakdown: ModelUsage[]
}

/**
 * Format cost as currency string
 */
export function formatCost(cost: number): string {
  return `$${cost.toFixed(2)}`
}

/**
 * Format token count with K/M suffix
 */
export function formatTokens(tokens: number): string {
  if (tokens >= 1_000_000_000) {
    return `${(tokens / 1_000_000_000).toFixed(1)}B`
  }
  if (tokens >= 1_000_000) {
    return `${(tokens / 1_000_000).toFixed(1)}M`
  }
  if (tokens >= 1_000) {
    return `${(tokens / 1_000).toFixed(1)}K`
  }
  return tokens.toString()
}
