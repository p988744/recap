import { useEffect, useState, useMemo } from 'react'
import { workItems, worklog } from '@/services'
import type { WorkItemStatsResponse } from '@/types'
import type { WorklogDay } from '@/types/worklog'
import type { TimelineSession } from '@/components/WorkGanttChart'
import { useSyncContext } from '@/hooks/useAppSync'

// =============================================================================
// Types
// =============================================================================

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

export function useDashboard(isAuthenticated: boolean) {
  const [stats, setStats] = useState<WorkItemStatsResponse | null>(null)
  const [heatmapStats, setHeatmapStats] = useState<WorkItemStatsResponse | null>(null)
  const [worklogDays, setWorklogDays] = useState<WorklogDay[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [weekRange] = useState(getThisWeekRange)
  const [heatmapRange] = useState(() => getHeatmapRange(53))

  // Gantt chart state
  const [ganttDate, setGanttDate] = useState(() => new Date().toISOString().split('T')[0])
  const [ganttSessions, setGanttSessions] = useState<TimelineSession[]>([])
  const [ganttLoading, setGanttLoading] = useState(false)
  const [ganttSources, setGanttSources] = useState<string[]>(['claude_code'])

  // Consume app-level sync state to know when to refetch data
  const { dataSyncState } = useSyncContext()

  // Main data fetch effect — re-runs when app-level sync completes
  useEffect(() => {
    if (!isAuthenticated) return

    async function fetchData() {
      try {
        const [statsResult, heatmapResult, worklogResult] = await Promise.all([
          workItems.getStats({ start_date: weekRange.start, end_date: weekRange.end }).catch(() => null),
          workItems.getStats({ start_date: heatmapRange.start, end_date: heatmapRange.end }).catch(() => null),
          worklog.getOverview(weekRange.start, weekRange.end).catch(() => null),
        ])
        setStats(statsResult)
        setHeatmapStats(heatmapResult)
        setWorklogDays(worklogResult?.days ?? [])
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load data')
      } finally {
        setLoading(false)
      }
    }
    fetchData()
  }, [weekRange, heatmapRange, dataSyncState, isAuthenticated])

  // Gantt timeline fetch effect — also re-runs when app-level sync completes
  useEffect(() => {
    if (!isAuthenticated) return

    async function fetchTimeline() {
      setGanttLoading(true)
      try {
        // Pass sources filter (undefined if all sources selected to use backend defaults)
        const sourcesToPass = ganttSources.length === 1 && ganttSources[0] === 'claude_code' ? undefined : ganttSources
        const result = await workItems.getTimeline(ganttDate, sourcesToPass)
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
  }, [ganttDate, ganttSources, dataSyncState, isAuthenticated])

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
    // Flatten worklog days into per-project activities, sorted by date descending
    const activities: Array<{
      projectName: string
      dailySummary?: string
      date: string
      totalHours: number
      totalCommits: number
      totalFiles: number
    }> = []

    for (const day of worklogDays) {
      for (const project of day.projects) {
        activities.push({
          projectName: project.project_name,
          dailySummary: project.daily_summary,
          date: day.date,
          totalHours: project.total_hours,
          totalCommits: project.total_commits,
          totalFiles: project.total_files,
        })
      }
    }

    // Sort by date descending, take top 5
    activities.sort((a, b) => b.date.localeCompare(a.date))
    return activities.slice(0, 5)
  }, [worklogDays])

  const projectCount = Object.keys(stats?.hours_by_project ?? {}).length

  const daysWorked = useMemo(() => {
    const dates = new Set(worklogDays.map(day => day.date))
    return dates.size
  }, [worklogDays])

  return {
    // State
    stats,
    heatmapStats,
    loading,
    error,
    weekRange,
    // Gantt
    ganttDate,
    setGanttDate,
    ganttSessions,
    ganttLoading,
    ganttSources,
    setGanttSources,
    // Computed
    chartData,
    recentActivities,
    projectCount,
    daysWorked,
  }
}
