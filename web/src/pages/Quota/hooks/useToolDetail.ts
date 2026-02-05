/**
 * useToolDetail hook
 *
 * Custom hook for the tool detail page, fetching history and cost data.
 */

import { useState, useEffect, useCallback } from 'react'
import { quota } from '@/services'
import type { QuotaSnapshot, CostSummary, AccountInfo } from '@/types/quota'
import { getToolById, QuotaTool } from '../tools'

const LOG_PREFIX = '[useToolDetail]'

export interface ToolDetailState {
  // Tool config
  tool: QuotaTool | null

  // Current quota snapshots for this tool
  snapshots: QuotaSnapshot[]
  accountInfo: AccountInfo | null
  providerAvailable: boolean

  // History data
  history: QuotaSnapshot[]

  // Cost data
  costSummary: CostSummary | null

  // Loading/error states
  loading: boolean
  error: string | null

  // Filter state
  days: number
  setDays: (days: number) => void

  // Actions
  refresh: () => Promise<void>
}

export function useToolDetail(toolId: string): ToolDetailState {
  // Tool config
  const tool = getToolById(toolId) ?? null

  // Current quota state
  const [snapshots, setSnapshots] = useState<QuotaSnapshot[]>([])
  const [accountInfo, setAccountInfo] = useState<AccountInfo | null>(null)
  const [providerAvailable, setProviderAvailable] = useState(true)

  // History state
  const [history, setHistory] = useState<QuotaSnapshot[]>([])

  // Cost state
  const [costSummary, setCostSummary] = useState<CostSummary | null>(null)

  // Loading/error state
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // Filter state
  const [days, setDays] = useState<number>(7)

  // Fetch current quota and filter for this tool
  const fetchCurrent = useCallback(async () => {
    if (!tool) return
    console.log(`${LOG_PREFIX} Fetching current quota for ${toolId}...`)
    try {
      const result = await quota.getCurrentQuota()
      const toolSnapshots = result.snapshots.filter(
        (s) => s.provider === toolId
      )
      console.log(
        `${LOG_PREFIX} Current quota fetched: ${toolSnapshots.length} snapshots`
      )
      setSnapshots(toolSnapshots)
      setProviderAvailable(result.provider_available)
      setAccountInfo(result.account_info)
    } catch (err) {
      console.error(`${LOG_PREFIX} Error fetching current quota:`, err)
      throw err
    }
  }, [toolId, tool])

  // Fetch history data for all window types
  const fetchHistory = useCallback(async () => {
    if (!tool) return
    console.log(`${LOG_PREFIX} Fetching history for ${toolId}, ${days} days`)
    try {
      // Fetch history for primary window types in parallel
      const windowTypes = ['5_hour', '7_day']
      const results = await Promise.all(
        windowTypes.map((wt) => quota.getQuotaHistory(toolId, wt, days))
      )
      // Combine all results
      const combined = results.flat()
      console.log(`${LOG_PREFIX} History fetched: ${combined.length} points`)
      setHistory(combined)
    } catch (err) {
      console.error(`${LOG_PREFIX} Error fetching history:`, err)
      throw err
    }
  }, [toolId, tool, days])

  // Fetch cost summary (only for tools that have cost data)
  const fetchCostSummary = useCallback(async () => {
    if (!tool || !tool.hasCost) {
      setCostSummary(null)
      return
    }
    console.log(`${LOG_PREFIX} Fetching cost summary for ${toolId}...`)
    try {
      const result = await quota.getCostSummary(30)
      console.log(
        `${LOG_PREFIX} Cost summary fetched: today=$${result.today_cost.toFixed(2)}`
      )
      setCostSummary(result)
    } catch (err) {
      console.error(`${LOG_PREFIX} Error fetching cost summary:`, err)
      // Don't throw - cost data is supplementary
    }
  }, [toolId, tool])

  // Combined refresh
  const refresh = useCallback(async () => {
    if (!tool) {
      setError(`Unknown tool: ${toolId}`)
      setLoading(false)
      return
    }

    console.log(`${LOG_PREFIX} Refreshing all data for ${toolId}...`)
    setLoading(true)
    setError(null)

    try {
      await Promise.all([fetchCurrent(), fetchHistory(), fetchCostSummary()])
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch data')
    } finally {
      setLoading(false)
    }
  }, [tool, toolId, fetchCurrent, fetchHistory, fetchCostSummary])

  // Initial load and when days change
  useEffect(() => {
    refresh()
  }, [refresh])

  return {
    tool,
    snapshots,
    accountInfo,
    providerAvailable,
    history,
    costSummary,
    loading,
    error,
    days,
    setDays,
    refresh,
  }
}
