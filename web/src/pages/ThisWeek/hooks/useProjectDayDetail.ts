import { useEffect, useState } from 'react'
import { worklog } from '@/services'
import type { WorklogDayProject, HourlyBreakdownItem } from '@/types/worklog'

export function useProjectDayDetail(date: string, projectPath: string, isAuthenticated: boolean) {
  const [project, setProject] = useState<WorklogDayProject | null>(null)
  const [weekday, setWeekday] = useState<string>('')
  const [hourlyData, setHourlyData] = useState<HourlyBreakdownItem[]>([])
  const [loading, setLoading] = useState(true)
  const [hourlyLoading, setHourlyLoading] = useState(true)

  // Fetch project data and hourly breakdown
  useEffect(() => {
    if (!isAuthenticated || !date || !projectPath) return

    const fetchData = async () => {
      setLoading(true)
      setHourlyLoading(true)

      try {
        // Fetch day overview to get project data
        const [overviewResult, hourlyResult] = await Promise.all([
          worklog.getOverview(date, date).catch(() => null),
          worklog.getHourlyBreakdown(date, projectPath).catch(() => []),
        ])

        // Find the day and project
        const day = overviewResult?.days.find((d) => d.date === date)
        const foundProject = day?.projects.find((p) => p.project_path === projectPath) ?? null

        setProject(foundProject)
        setWeekday(day?.weekday ?? '')
        setHourlyData(hourlyResult)
      } catch (err) {
        console.error('Failed to fetch project day data:', err)
        setProject(null)
        setHourlyData([])
      } finally {
        setLoading(false)
        setHourlyLoading(false)
      }
    }

    fetchData()
  }, [date, projectPath, isAuthenticated])

  return {
    project,
    weekday,
    hourlyData,
    loading,
    hourlyLoading,
  }
}
