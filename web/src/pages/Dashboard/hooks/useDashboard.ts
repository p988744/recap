import { useEffect, useState, useMemo, useCallback } from 'react'
import { listen } from '@tauri-apps/api/event'
import { workItems, sync, tempo } from '@/services'
import type { WorkItemStatsResponse, WorkItem, SyncStatus as SyncStatusType } from '@/types'
import type { TimelineSession } from '@/components/WorkGanttChart'

// =============================================================================
// Types
// =============================================================================

export type SyncState = 'idle' | 'syncing' | 'success' | 'error'

// =============================================================================
// Utility Functions
// =============================================================================

export function getThisWeekRange() {
  const now = new Date()
  const dayOfWeek = now.getDay()
  const monday = new Date(now)
  monday.setDate(now.getDate() - (dayOfWeek === 0 ? 6 : dayOfWeek - 1))
  monday.setHours(0, 0, 0, 0)

  const sunday = new Date(monday)
  sunday.setDate(monday.getDate() + 6)

  return {
    start: monday.toISOString().split('T')[0],
    end: sunday.toISOString().split('T')[0],
  }
}

export function getHeatmapRange(weeks: number = 53) {
  const now = new Date()
  const end = now.toISOString().split('T')[0]

  const start = new Date(now)
  start.setDate(now.getDate() - weeks * 7)

  return {
    start: start.toISOString().split('T')[0],
    end,
  }
}

// =============================================================================
// Main Hook: useDashboard
// =============================================================================

export function useDashboard(isAuthenticated: boolean, token: string | null) {
  const [stats, setStats] = useState<WorkItemStatsResponse | null>(null)
  const [heatmapStats, setHeatmapStats] = useState<WorkItemStatsResponse | null>(null)
  const [recentItems, setRecentItems] = useState<WorkItem[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [syncStatus, setSyncStatus] = useState<SyncState>('idle')
  const [syncMessage, setSyncMessage] = useState<string>('')
  const [weekRange] = useState(getThisWeekRange)
  const [heatmapRange] = useState(() => getHeatmapRange(53))
  const [claudeSyncInfo, setClaudeSyncInfo] = useState<string>('')

  // Gantt chart state
  const [ganttDate, setGanttDate] = useState(() => new Date().toISOString().split('T')[0])
  const [ganttSessions, setGanttSessions] = useState<TimelineSession[]>([])
  const [ganttLoading, setGanttLoading] = useState(false)

  // Auto-sync state
  const [autoSyncState, setAutoSyncState] = useState<'idle' | 'syncing' | 'done'>('idle')
  const [syncStatusData, setSyncStatusData] = useState<SyncStatusType[]>([])

  // Last sync time (client-side tracking)
  const [lastSyncTime, setLastSyncTime] = useState<Date | null>(null)

  // Sync function (shared between auto and manual)
  const performSync = useCallback(async () => {
    if (!isAuthenticated || !token) return
    setAutoSyncState('syncing')

    try {
      const result = await sync.autoSync()
      if (result.total_items > 0) {
        const totalCreatedUpdated = result.results.reduce((sum, r) => sum + r.items_synced, 0)
        setClaudeSyncInfo(`已同步 ${totalCreatedUpdated} 筆工作項目`)
        setTimeout(() => setClaudeSyncInfo(''), 4000)
      }

      const statuses = await sync.getStatus().catch(() => [])
      setSyncStatusData(statuses)
      setLastSyncTime(new Date())
    } catch {
      // Silent fail for sync
    } finally {
      setAutoSyncState('done')
    }
  }, [isAuthenticated, token])

  // Auto-sync effect (only runs once on mount)
  useEffect(() => {
    if (autoSyncState === 'idle') {
      performSync()
    }
  }, []) // eslint-disable-line react-hooks/exhaustive-deps

  // Manual sync handler
  const handleManualSync = useCallback(async () => {
    if (autoSyncState === 'syncing') return
    setAutoSyncState('idle') // Reset to allow sync
    await performSync()
  }, [autoSyncState, performSync])

  // Tray "Sync Now" event listener
  useEffect(() => {
    const unlisten = listen('tray-sync-now', () => {
      console.log('Tray sync triggered')
      handleManualSync()
    })

    return () => {
      unlisten.then(fn => fn())
    }
  }, [handleManualSync])

  // Main data fetch effect
  useEffect(() => {
    if (!isAuthenticated || !token) return

    async function fetchData() {
      try {
        const [statsResult, heatmapResult, itemsResult] = await Promise.all([
          workItems.getStats({ start_date: weekRange.start, end_date: weekRange.end }).catch(() => null),
          workItems.getStats({ start_date: heatmapRange.start, end_date: heatmapRange.end }).catch(() => null),
          workItems.list({
            start_date: weekRange.start,
            end_date: weekRange.end,
            per_page: 20
          }).catch(() => null),
        ])
        setStats(statsResult)
        setHeatmapStats(heatmapResult)
        setRecentItems(itemsResult?.items ?? [])
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load data')
      } finally {
        setLoading(false)
      }
    }
    fetchData()
  }, [weekRange, heatmapRange, claudeSyncInfo, isAuthenticated, token])

  // Gantt timeline fetch effect
  useEffect(() => {
    if (!isAuthenticated || !token) return

    async function fetchTimeline() {
      setGanttLoading(true)
      try {
        const result = await workItems.getTimeline(ganttDate)
        const sessions: TimelineSession[] = result.sessions.map(s => ({
          id: s.id,
          project: s.project,
          title: s.title,
          startTime: s.start_time,
          endTime: s.end_time,
          hours: s.hours,
          commits: s.commits.map(c => ({
            hash: c.hash,
            message: c.message,
            time: c.time,
            author: c.author,
          })),
        }))
        setGanttSessions(sessions)
      } catch {
        setGanttSessions([])
      } finally {
        setGanttLoading(false)
      }
    }
    fetchTimeline()
  }, [ganttDate, isAuthenticated, token])

  // Sync to Tempo handler
  const handleSyncToTempo = useCallback(async () => {
    setSyncStatus('syncing')
    setSyncMessage('')

    try {
      const response = await workItems.list({
        jira_mapped: true,
        synced_to_tempo: false,
        per_page: 100,
      })

      const itemsToSync = response.items.filter(item => item.jira_issue_key && item.hours > 0)
      if (itemsToSync.length === 0) {
        setSyncStatus('success')
        setSyncMessage('所有項目都已同步')
        setTimeout(() => setSyncStatus('idle'), 3000)
        return
      }

      const entries = itemsToSync.map(item => ({
        issue_key: item.jira_issue_key!,
        date: item.date.split('T')[0],
        minutes: Math.round(item.hours * 60),
        description: item.title,
      }))

      const result = await tempo.syncWorklogs({ entries, dry_run: false })

      if (result.results.length > 0) {
        for (let i = 0; i < itemsToSync.length; i++) {
          const item = itemsToSync[i]
          const syncResult = result.results[i]

          if (syncResult && syncResult.status === 'success') {
            try {
              await workItems.update(item.id, {
                synced_to_tempo: true,
                tempo_worklog_id: syncResult.id || undefined,
              })
            } catch {
              // Ignore individual update errors
            }
          }
        }
      }

      const newStats = await workItems.getStats({ start_date: weekRange.start, end_date: weekRange.end }).catch(() => null)
      if (newStats) setStats(newStats)

      setSyncStatus('success')
      setSyncMessage(`成功同步 ${result.successful}/${result.total_entries} 筆`)
      setTimeout(() => setSyncStatus('idle'), 3000)
    } catch (err) {
      setSyncStatus('error')
      setSyncMessage(err instanceof Error ? err.message : '同步失敗')
      setTimeout(() => setSyncStatus('idle'), 5000)
    }
  }, [weekRange])

  // Computed values
  const chartData = useMemo(() => {
    if (!stats?.hours_by_project) return []
    return Object.entries(stats.hours_by_project)
      .sort((a, b) => b[1] - a[1])
      .map(([name, hours]) => ({
        name,
        value: hours,
        hours: hours.toFixed(1),
      }))
  }, [stats])

  const recentActivities = useMemo(() => {
    return recentItems.slice(0, 5).map(item => ({
      title: item.title,
      source: item.source,
      date: item.date,
      hours: item.hours,
      jiraKey: item.jira_issue_key,
    }))
  }, [recentItems])

  const projectCount = Object.keys(stats?.hours_by_project ?? {}).length

  const daysWorked = useMemo(() => {
    const dates = new Set(recentItems.map(item => item.date.split('T')[0]))
    return dates.size
  }, [recentItems])

  return {
    // State
    stats,
    heatmapStats,
    loading,
    error,
    weekRange,
    autoSyncState,
    syncStatusData,
    claudeSyncInfo,
    lastSyncTime,
    handleManualSync,
    // Gantt
    ganttDate,
    setGanttDate,
    ganttSessions,
    ganttLoading,
    // Tempo sync
    syncStatus,
    syncMessage,
    handleSyncToTempo,
    // Computed
    chartData,
    recentActivities,
    projectCount,
    daysWorked,
  }
}
