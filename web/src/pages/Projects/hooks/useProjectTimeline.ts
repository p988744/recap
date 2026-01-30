import { useState, useEffect, useCallback, useRef } from 'react'
import { projects as projectsService } from '@/services'
import type { TimeUnit, TimelineGroup, ProjectTimelineRequest } from '@/types'

interface UseProjectTimelineOptions {
  projectName: string
  timeUnit?: TimeUnit
  sources?: string[]
  limit?: number
}

interface UseProjectTimelineReturn {
  groups: TimelineGroup[]
  isLoading: boolean
  isLoadingMore: boolean
  error: string | null
  hasMore: boolean
  timeUnit: TimeUnit
  sources: string[]
  setTimeUnit: (unit: TimeUnit) => void
  setSources: (sources: string[]) => void
  loadMore: () => Promise<void>
  refetch: () => Promise<void>
}

// Get default date range based on time unit
function getDefaultDateRange(timeUnit: TimeUnit): { start: string; end: string } {
  const now = new Date()
  const end = now.toISOString().split('T')[0]

  let start: Date
  switch (timeUnit) {
    case 'day':
      // Last 30 days
      start = new Date(now)
      start.setDate(start.getDate() - 30)
      break
    case 'week':
      // Last 12 weeks
      start = new Date(now)
      start.setDate(start.getDate() - 84)
      break
    case 'month':
      // Last 12 months
      start = new Date(now)
      start.setMonth(start.getMonth() - 12)
      break
    case 'quarter':
      // Last 8 quarters (2 years)
      start = new Date(now)
      start.setFullYear(start.getFullYear() - 2)
      break
    case 'year':
      // Last 5 years
      start = new Date(now)
      start.setFullYear(start.getFullYear() - 5)
      break
    default:
      start = new Date(now)
      start.setMonth(start.getMonth() - 3)
  }

  return {
    start: start.toISOString().split('T')[0],
    end,
  }
}

export function useProjectTimeline({
  projectName,
  timeUnit: initialTimeUnit = 'week',
  sources: initialSources = [],
  limit = 10,
}: UseProjectTimelineOptions): UseProjectTimelineReturn {
  const [groups, setGroups] = useState<TimelineGroup[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [isLoadingMore, setIsLoadingMore] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [hasMore, setHasMore] = useState(false)
  const [timeUnit, setTimeUnit] = useState<TimeUnit>(initialTimeUnit)
  const [sources, setSources] = useState<string[]>(initialSources)
  const [cursor, setCursor] = useState<string | null>(null)

  // Track the current fetch to avoid race conditions
  const fetchIdRef = useRef(0)

  const fetchTimeline = useCallback(
    async (options: { append?: boolean } = {}) => {
      const { append = false } = options
      const fetchId = ++fetchIdRef.current

      try {
        if (append) {
          setIsLoadingMore(true)
        } else {
          setIsLoading(true)
          setCursor(null)
        }
        setError(null)

        const { start, end } = getDefaultDateRange(timeUnit)

        const request: ProjectTimelineRequest = {
          project_name: projectName,
          time_unit: timeUnit,
          range_start: start,
          range_end: end,
          sources: sources.length > 0 ? sources : undefined,
          cursor: append ? cursor || undefined : undefined,
          limit,
        }

        const response = await projectsService.getProjectTimeline(request)

        // Only update if this is still the latest fetch
        if (fetchId !== fetchIdRef.current) return

        if (append) {
          setGroups((prev) => [...prev, ...response.groups])
        } else {
          setGroups(response.groups)
        }

        setHasMore(response.has_more)
        setCursor(response.next_cursor)
      } catch (err) {
        if (fetchId !== fetchIdRef.current) return
        setError(err instanceof Error ? err.message : 'Failed to load timeline')
      } finally {
        if (fetchId === fetchIdRef.current) {
          setIsLoading(false)
          setIsLoadingMore(false)
        }
      }
    },
    [projectName, timeUnit, sources, limit, cursor]
  )

  // Initial fetch and refetch when filters change
  useEffect(() => {
    if (projectName) {
      fetchTimeline()
    }
  }, [projectName, timeUnit, sources])

  const loadMore = useCallback(async () => {
    if (hasMore && !isLoadingMore && cursor) {
      await fetchTimeline({ append: true })
    }
  }, [hasMore, isLoadingMore, cursor, fetchTimeline])

  const refetch = useCallback(async () => {
    await fetchTimeline()
  }, [fetchTimeline])

  const handleTimeUnitChange = useCallback((unit: TimeUnit) => {
    setTimeUnit(unit)
    setGroups([])
    setCursor(null)
  }, [])

  const handleSourcesChange = useCallback((newSources: string[]) => {
    setSources(newSources)
    setGroups([])
    setCursor(null)
  }, [])

  return {
    groups,
    isLoading,
    isLoadingMore,
    error,
    hasMore,
    timeUnit,
    sources,
    setTimeUnit: handleTimeUnitChange,
    setSources: handleSourcesChange,
    loadMore,
    refetch,
  }
}
