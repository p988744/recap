import { useEffect, useState, useCallback, useMemo } from 'react'
import { worklog, workItems, config as configService } from '@/services'
import type { WorklogDay, HourlyBreakdownItem } from '@/types/worklog'
import type { WorkItem, WorkItemStatsResponse } from '@/types'
import { useSyncContext } from '@/hooks/useAppSync'

// =============================================================================
// Types
// =============================================================================

export interface WorkItemFormData {
  title: string
  description: string
  hours: number
  date: string
  jira_issue_key: string
  category: string
  project_name: string
}

// =============================================================================
// Date Range Utilities
// =============================================================================

function getWeekRange(weekStartDay: number = 1): { start: string; end: string } {
  const today = new Date()
  const day = today.getDay()
  const diff = (day - weekStartDay + 7) % 7
  const start = new Date(today)
  start.setDate(today.getDate() - diff)
  const end = new Date(start)
  end.setDate(start.getDate() + 6)
  return {
    start: formatDate(start),
    end: formatDate(end),
  }
}

function formatDate(d: Date): string {
  // Use local date to avoid timezone issues
  const year = d.getFullYear()
  const month = String(d.getMonth() + 1).padStart(2, '0')
  const day = String(d.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
}

function shiftWeek(startDate: string, direction: -1 | 1): { start: string; end: string } {
  // Parse as local date to avoid timezone issues
  const start = new Date(startDate + 'T00:00:00')
  start.setDate(start.getDate() + direction * 7)
  const end = new Date(start)
  end.setDate(start.getDate() + 6)
  return {
    start: formatDate(start),
    end: formatDate(end),
  }
}

function getWeekNumber(dateStr: string): number {
  const date = new Date(dateStr + 'T00:00:00')
  const firstDayOfYear = new Date(date.getFullYear(), 0, 1)
  const pastDaysOfYear = (date.getTime() - firstDayOfYear.getTime()) / 86400000
  return Math.ceil((pastDaysOfYear + firstDayOfYear.getDay() + 1) / 7)
}

// =============================================================================
// Main Hook: useThisWeek
// =============================================================================

export function useThisWeek(isAuthenticated: boolean) {
  // Week start day from config (default Monday)
  const [weekStartDay, setWeekStartDay] = useState(1)
  const initialRange = getWeekRange(weekStartDay)
  const [startDate, setStartDate] = useState(initialRange.start)
  const [endDate, setEndDate] = useState(initialRange.end)

  // Whether Jira/Tempo is configured
  const [jiraConfigured, setJiraConfigured] = useState(false)

  // Data state
  const [days, setDays] = useState<WorklogDay[]>([])
  const [stats, setStats] = useState<WorkItemStatsResponse | null>(null)
  const [loading, setLoading] = useState(true)

  // Expanded days state (for collapsible day cards)
  const [expandedDays, setExpandedDays] = useState<Set<string>>(new Set())

  // Expanded hourly breakdown state
  const [expandedProject, setExpandedProject] = useState<{ date: string; projectPath: string } | null>(null)
  const [hourlyData, setHourlyData] = useState<HourlyBreakdownItem[]>([])
  const [hourlyLoading, setHourlyLoading] = useState(false)

  // CRUD state
  const [selectedItem, setSelectedItem] = useState<WorkItem | null>(null)
  const [showCreateModal, setShowCreateModal] = useState(false)
  const [showEditModal, setShowEditModal] = useState(false)
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false)
  const [itemToDelete, setItemToDelete] = useState<WorkItem | null>(null)
  const [createDate, setCreateDate] = useState<string>('')
  const [formData, setFormData] = useState<WorkItemFormData>({
    title: '',
    description: '',
    hours: 0,
    date: new Date().toISOString().split('T')[0],
    jira_issue_key: '',
    category: '',
    project_name: '',
  })

  // Consume app-level sync state to know when to refetch data
  const { dataSyncState, summaryState, backendStatus } = useSyncContext()

  // Today's date for comparison
  const today = useMemo(() => new Date().toISOString().split('T')[0], [])

  // ==========================================================================
  // Fetch config on mount
  // ==========================================================================

  useEffect(() => {
    if (!isAuthenticated) return
    configService.getConfig()
      .then((c) => {
        console.log('[ThisWeek] Config loaded, week_start_day:', c.week_start_day)
        setJiraConfigured(c.jira_configured)
        if (c.week_start_day !== weekStartDay) {
          setWeekStartDay(c.week_start_day)
          const range = getWeekRange(c.week_start_day)
          console.log('[ThisWeek] Week range:', range.start, '-', range.end)
          setStartDate(range.start)
          setEndDate(range.end)
        }
      })
      .catch(() => {})
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isAuthenticated])

  // ==========================================================================
  // Initialize expanded days (today is expanded by default)
  // ==========================================================================

  useEffect(() => {
    // When days change, ensure today is expanded
    if (days.length > 0) {
      setExpandedDays(prev => {
        const newSet = new Set(prev)
        // Only auto-expand today if it's in the current week
        const todayDay = days.find(d => d.date === today)
        if (todayDay) {
          newSet.add(today)
        }
        return newSet
      })
    }
  }, [days, today])

  // ==========================================================================
  // Fetch overview and stats
  // ==========================================================================

  const fetchData = useCallback(async () => {
    setLoading(true)
    try {
      const [worklogResult, statsResult] = await Promise.all([
        worklog.getOverview(startDate, endDate).catch(() => null),
        workItems.getStats({ start_date: startDate, end_date: endDate }).catch(() => null),
      ])
      setDays(worklogResult?.days ?? [])
      setStats(statsResult)
    } catch (err) {
      console.error('Failed to fetch data:', err)
      setDays([])
      setStats(null)
    } finally {
      setLoading(false)
    }
  }, [startDate, endDate])

  useEffect(() => {
    if (!isAuthenticated) return
    fetchData()
  }, [isAuthenticated, fetchData, dataSyncState, summaryState, backendStatus?.last_sync_at])

  // ==========================================================================
  // Date navigation
  // ==========================================================================

  const goToPreviousWeek = useCallback(() => {
    const range = shiftWeek(startDate, -1)
    setStartDate(range.start)
    setEndDate(range.end)
  }, [startDate])

  const goToNextWeek = useCallback(() => {
    const range = shiftWeek(startDate, 1)
    setStartDate(range.start)
    setEndDate(range.end)
  }, [startDate])

  const goToThisWeek = useCallback(() => {
    const range = getWeekRange(weekStartDay)
    setStartDate(range.start)
    setEndDate(range.end)
  }, [weekStartDay])

  const isCurrentWeek = useMemo(() => {
    const range = getWeekRange(weekStartDay)
    return startDate === range.start && endDate === range.end
  }, [startDate, endDate, weekStartDay])

  // ==========================================================================
  // Day expansion
  // ==========================================================================

  const toggleDay = useCallback((date: string) => {
    setExpandedDays(prev => {
      const newSet = new Set(prev)
      if (newSet.has(date)) {
        newSet.delete(date)
      } else {
        newSet.add(date)
      }
      return newSet
    })
  }, [])

  const isDayExpanded = useCallback((date: string) => {
    return expandedDays.has(date)
  }, [expandedDays])

  // ==========================================================================
  // Hourly breakdown
  // ==========================================================================

  const toggleHourlyBreakdown = useCallback(async (date: string, projectPath: string) => {
    if (expandedProject?.date === date && expandedProject?.projectPath === projectPath) {
      setExpandedProject(null)
      setHourlyData([])
      return
    }

    setExpandedProject({ date, projectPath })
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
  }, [expandedProject])

  // ==========================================================================
  // CRUD operations
  // ==========================================================================

  const resetForm = useCallback(() => {
    setFormData({
      title: '',
      description: '',
      hours: 0,
      date: new Date().toISOString().split('T')[0],
      jira_issue_key: '',
      category: '',
      project_name: '',
    })
  }, [])

  const openCreateModal = useCallback((date?: string) => {
    resetForm()
    if (date) {
      setFormData(prev => ({ ...prev, date }))
      setCreateDate(date)
    }
    setShowCreateModal(true)
  }, [resetForm])

  const closeCreateModal = useCallback(() => {
    setShowCreateModal(false)
    resetForm()
  }, [resetForm])

  const handleCreate = useCallback(async (e: React.FormEvent) => {
    e.preventDefault()
    try {
      await workItems.create({
        title: formData.title,
        description: formData.description || undefined,
        hours: formData.hours,
        date: formData.date,
        jira_issue_key: formData.jira_issue_key || undefined,
        category: formData.category || undefined,
        project_name: formData.project_name || undefined,
      })
      setShowCreateModal(false)
      resetForm()
      fetchData()
    } catch (err) {
      console.error('Failed to create work item:', err)
    }
  }, [formData, resetForm, fetchData])

  const openEditModal = useCallback((item: WorkItem) => {
    setSelectedItem(item)

    // Derive project_name from project_path for manual items
    let project_name = ''
    if (item.project_path?.includes('manual-projects')) {
      const segments = item.project_path.split(/[/\\]/)
      project_name = segments[segments.length - 1] || ''
    } else if (item.project_path) {
      const segments = item.project_path.split(/[/\\]/)
      project_name = segments[segments.length - 1] || ''
    }

    // Legacy: check title prefix for backward compatibility
    if (!project_name && item.title.startsWith('[') && item.title.includes('] ')) {
      const endIndex = item.title.indexOf('] ')
      project_name = item.title.substring(1, endIndex)
    }

    setFormData({
      title: item.title,
      description: item.description || '',
      hours: item.hours,
      date: item.date,
      jira_issue_key: item.jira_issue_key || '',
      category: item.category || '',
      project_name,
    })
    setShowEditModal(true)
  }, [])

  const closeEditModal = useCallback(() => {
    setShowEditModal(false)
    setSelectedItem(null)
    resetForm()
  }, [resetForm])

  const handleUpdate = useCallback(async (e: React.FormEvent) => {
    e.preventDefault()
    if (!selectedItem) return
    try {
      await workItems.update(selectedItem.id, {
        title: formData.title,
        description: formData.description || undefined,
        hours: formData.hours,
        date: formData.date,
        jira_issue_key: formData.jira_issue_key || undefined,
        category: formData.category || undefined,
        project_name: formData.project_name || undefined,
      })
      setShowEditModal(false)
      setSelectedItem(null)
      resetForm()
      fetchData()
    } catch (err) {
      console.error('Failed to update work item:', err)
    }
  }, [selectedItem, formData, resetForm, fetchData])

  const confirmDelete = useCallback((item: WorkItem) => {
    setItemToDelete(item)
    setShowDeleteConfirm(true)
  }, [])

  const closeDeleteConfirm = useCallback(() => {
    setShowDeleteConfirm(false)
    setItemToDelete(null)
  }, [])

  const handleDelete = useCallback(async () => {
    if (!itemToDelete) return
    try {
      await workItems.remove(itemToDelete.id)
      setShowDeleteConfirm(false)
      setItemToDelete(null)
      fetchData()
    } catch (err) {
      console.error('Failed to delete work item:', err)
    }
  }, [itemToDelete, fetchData])

  const openEditManualItem = useCallback(async (id: string) => {
    try {
      const item = await workItems.get(id)
      openEditModal(item)
    } catch (err) {
      console.error('Failed to get work item:', err)
    }
  }, [openEditModal])

  const confirmDeleteManualItem = useCallback(async (id: string) => {
    try {
      const item = await workItems.get(id)
      confirmDelete(item)
    } catch (err) {
      console.error('Failed to get work item:', err)
    }
  }, [confirmDelete])

  // ==========================================================================
  // Computed values
  // ==========================================================================

  const weekNumber = useMemo(() => getWeekNumber(startDate), [startDate])

  // Count unique projects from worklog days (same source as timeline heatmap)
  const projectCount = useMemo(() => {
    const projectPaths = new Set<string>()
    days.forEach(d => {
      d.projects.forEach(p => projectPaths.add(p.project_path))
    })
    return projectPaths.size
  }, [days])

  const daysWorked = useMemo(() => {
    const dates = new Set(days.filter(d => d.projects.length > 0 || d.manual_items.length > 0).map(d => d.date))
    return dates.size
  }, [days])

  return {
    // Week info
    weekNumber,
    startDate,
    endDate,
    goToPreviousWeek,
    goToNextWeek,
    goToThisWeek,
    isCurrentWeek,
    today,
    // Data
    days,
    stats,
    loading,
    projectCount,
    daysWorked,
    // Day expansion
    expandedDays,
    toggleDay,
    isDayExpanded,
    // Hourly breakdown
    expandedProject,
    hourlyData,
    hourlyLoading,
    toggleHourlyBreakdown,
    // Week config
    weekStartDay,
    // CRUD
    showCreateModal,
    showEditModal,
    showDeleteConfirm,
    selectedItem,
    itemToDelete,
    formData,
    createDate,
    setFormData,
    openCreateModal,
    closeCreateModal,
    handleCreate,
    openEditModal,
    closeEditModal,
    handleUpdate,
    confirmDelete,
    closeDeleteConfirm,
    handleDelete,
    openEditManualItem,
    confirmDeleteManualItem,
    // Config
    jiraConfigured,
    // Refresh
    fetchData,
  }
}
