import { useState, useEffect, useCallback } from 'react'
import * as llmUsage from '@/services/llm-usage'
import type { LlmUsageStats, DailyUsage, ModelUsage, LlmUsageLog } from '@/types'

interface UseLlmUsageResult {
  stats: LlmUsageStats | null
  daily: DailyUsage[]
  models: ModelUsage[]
  logs: LlmUsageLog[]
  loading: boolean
  error: string | null
  refresh: () => void
}

export function useLlmUsage(startDate: string, endDate: string): UseLlmUsageResult {
  const [stats, setStats] = useState<LlmUsageStats | null>(null)
  const [daily, setDaily] = useState<DailyUsage[]>([])
  const [models, setModels] = useState<ModelUsage[]>([])
  const [logs, setLogs] = useState<LlmUsageLog[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const fetchData = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const [statsData, dailyData, modelsData, logsData] = await Promise.all([
        llmUsage.getUsageStats(startDate, endDate),
        llmUsage.getUsageDaily(startDate, endDate),
        llmUsage.getUsageByModel(startDate, endDate),
        llmUsage.getUsageLogs(startDate, endDate, 50, 0),
      ])
      setStats(statsData)
      setDaily(dailyData)
      setModels(modelsData)
      setLogs(logsData)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setLoading(false)
    }
  }, [startDate, endDate])

  useEffect(() => {
    fetchData()
  }, [fetchData])

  return { stats, daily, models, logs, loading, error, refresh: fetchData }
}
