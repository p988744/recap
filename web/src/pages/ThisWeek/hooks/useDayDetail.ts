import { useEffect, useState, useCallback } from 'react'
import { worklog } from '@/services'
import type { WorklogDay, HourlyBreakdownItem } from '@/types/worklog'

export function useDayDetail(date: string, isAuthenticated: boolean) {
  const [day, setDay] = useState<WorklogDay | null>(null)
  const [loading, setLoading] = useState(true)

  // Expanded hourly breakdown state
  const [expandedProject, setExpandedProject] = useState<string | null>(null)
  const [hourlyData, setHourlyData] = useState<HourlyBreakdownItem[]>([])
  const [hourlyLoading, setHourlyLoading] = useState(false)

  // Fetch day data
  useEffect(() => {
    if (!isAuthenticated || !date) return

    const fetchData = async () => {
      setLoading(true)
      try {
        const result = await worklog.getOverview(date, date)
        // Find the day in the result
        const foundDay = result.days.find((d) => d.date === date) ?? null
        setDay(foundDay)
      } catch (err) {
        console.error('Failed to fetch day data:', err)
        setDay(null)
      } finally {
        setLoading(false)
      }
    }

    fetchData()
  }, [date, isAuthenticated])

  // Toggle hourly breakdown
  const toggleHourlyBreakdown = useCallback(
    async (projectPath: string) => {
      if (expandedProject === projectPath) {
        setExpandedProject(null)
        setHourlyData([])
        return
      }

      setExpandedProject(projectPath)
      setHourlyLoading(true)
      try {
        const data = await worklog.getHourlyBreakdown(date, projectPath)
        setHourlyData(data)
      } catch (err) {
        console.error('Failed to fetch hourly breakdown:', err)
        setHourlyData([])
      } finally {
        setHourlyLoading(false)
      }
    },
    [date, expandedProject]
  )

  // Computed values
  const totalHours =
    (day?.projects.reduce((sum, p) => sum + p.total_hours, 0) ?? 0) +
    (day?.manual_items.reduce((sum, m) => sum + m.hours, 0) ?? 0)

  const totalCommits = day?.projects.reduce((sum, p) => sum + p.total_commits, 0) ?? 0
  const projectCount = (day?.projects.length ?? 0) + (day?.manual_items.length ?? 0)

  return {
    day,
    loading,
    totalHours,
    totalCommits,
    projectCount,
    expandedProject,
    hourlyData,
    hourlyLoading,
    toggleHourlyBreakdown,
  }
}
