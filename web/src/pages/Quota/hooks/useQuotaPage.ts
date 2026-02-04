/**
 * useQuotaPage hook
 *
 * Custom hook for managing quota page state and data fetching.
 */

import { useState, useEffect, useCallback } from 'react'
import { quota, tray } from '@/services'
import type { QuotaSnapshot, QuotaSettings } from '@/types/quota'

const LOG_PREFIX = '[useQuotaPage]'

export const DEFAULT_QUOTA_SETTINGS: QuotaSettings = {
  interval_minutes: 15,
  warning_threshold: 80,
  critical_threshold: 95,
  notifications_enabled: true,
}

export interface QuotaPageState {
  // Current quota data
  currentQuota: QuotaSnapshot[]
  providerAvailable: boolean

  // History data (all window types combined)
  history: QuotaSnapshot[]

  // Loading/error states
  loading: boolean
  error: string | null

  // Filter state
  provider: string
  setProvider: (provider: string) => void
  days: number
  setDays: (days: number) => void

  // Actions
  refresh: () => Promise<void>
}

export function useQuotaPage(): QuotaPageState {
  // Current quota state
  const [currentQuota, setCurrentQuota] = useState<QuotaSnapshot[]>([])
  const [providerAvailable, setProviderAvailable] = useState(true)

  // History state
  const [history, setHistory] = useState<QuotaSnapshot[]>([])

  // Loading/error state
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // Filter state
  const [provider, setProvider] = useState<string>('claude')
  const [days, setDays] = useState<number>(7)

  // Fetch current quota
  const fetchCurrent = useCallback(async () => {
    console.log(`${LOG_PREFIX} Fetching current quota...`)
    try {
      const result = await quota.getCurrentQuota()
      console.log(`${LOG_PREFIX} Current quota fetched:`, result)
      setCurrentQuota(result.snapshots)
      setProviderAvailable(result.provider_available)

      // Update tray with primary quota
      const fiveHour = result.snapshots.find(
        (s) => s.provider === 'claude' && s.window_type === '5_hour'
      )
      if (fiveHour) {
        tray.updateTrayQuota(fiveHour.used_percent).catch((err) => {
          console.error(`${LOG_PREFIX} Failed to update tray:`, err)
        })
      }
    } catch (err) {
      console.error(`${LOG_PREFIX} Error fetching current quota:`, err)
      throw err
    }
  }, [])

  // Fetch history data for all window types
  const fetchHistory = useCallback(async () => {
    console.log(`${LOG_PREFIX} Fetching history for all window types, ${days} days`)
    try {
      // Fetch history for primary window types in parallel
      const windowTypes = ['5_hour', '7_day']
      const results = await Promise.all(
        windowTypes.map((wt) => quota.getQuotaHistory(provider, wt, days))
      )
      // Combine all results
      const combined = results.flat()
      console.log(`${LOG_PREFIX} History fetched:`, combined.length, 'points')
      setHistory(combined)
    } catch (err) {
      console.error(`${LOG_PREFIX} Error fetching history:`, err)
      throw err
    }
  }, [provider, days])

  // Combined refresh
  const refresh = useCallback(async () => {
    console.log(`${LOG_PREFIX} Refreshing all data...`)
    setLoading(true)
    setError(null)

    try {
      await Promise.all([fetchCurrent(), fetchHistory()])
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch quota data')
    } finally {
      setLoading(false)
    }
  }, [fetchCurrent, fetchHistory])

  // Initial load
  useEffect(() => {
    refresh()
  }, [refresh])

  return {
    currentQuota,
    providerAvailable,
    history,
    loading,
    error,
    provider,
    setProvider,
    days,
    setDays,
    refresh,
  }
}
