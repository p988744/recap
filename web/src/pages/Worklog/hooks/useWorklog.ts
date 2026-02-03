import { useEffect, useState, useCallback, useMemo } from 'react'
import { worklog, workItems, config as configService } from '@/services'
import type { WorklogDay, HourlyBreakdownItem } from '@/types/worklog'
import type { WorkItem } from '@/types'
import type { TimelineSession } from '@/components/WorkGanttChart'

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
  // Calculate diff to get to the start of the week
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
  return d.toISOString().split('T')[0]
}

function shiftWeek(startDate: string, direction: -1 | 1): { start: string; end: string } {
  const start = new Date(startDate)
  start.setDate(start.getDate() + direction * 7)
  const end = new Date(start)
  end.setDate(start.getDate() + 6)
  return {
    start: formatDate(start),
    end: formatDate(end),
  }
}

// =============================================================================
// Main Hook: useWorklog
// =============================================================================

export function useWorklog(isAuthenticated: boolean) {
  // Week start day from config (default Monday)
  const [weekStartDay, setWeekStartDay] = useState(1)
  const initialRange = getWeekRange(weekStartDay)
  const [startDate, setStartDate] = useState(initialRange.start)
  const [endDate, setEndDate] = useState(initialRange.end)

  // Whether Jira/Tempo is configured
  const [jiraConfigured, setJiraConfigured] = useState(false)

  // Fetch week_start_day from config
  useEffect(() => {
    if (!isAuthenticated) return
    configService.getConfig()
      .then((c) => {
        setJiraConfigured(c.jira_configured)
        if (c.week_start_day !== weekStartDay) {
          setWeekStartDay(c.week_start_day)
          const range = getWeekRange(c.week_start_day)
          setStartDate(range.start)
          setEndDate(range.end)
        }
      })
      .catch(() => {})
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isAuthenticated])

  // Data state
  const [days, setDays] = useState<WorklogDay[]>([])
  const [loading, setLoading] = useState(true)

  // Expanded hourly breakdown state
  const [expandedProject, setExpandedProject] = useState<{ date: string; projectPath: string } | null>(null)
  const [hourlyData, setHourlyData] = useState<HourlyBreakdownItem[]>([])
  const [hourlyLoading, setHourlyLoading] = useState(false)

  // Gantt chart state
  const [ganttDate, setGanttDate] = useState(() => new Date().toISOString().split('T')[0])
  const [ganttSessions, setGanttSessions] = useState<TimelineSession[]>([])
  const [ganttLoading, setGanttLoading] = useState(false)
  const [ganttSources, setGanttSources] = useState<string[]>(['claude_code', 'antigravity'])

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

  // ==========================================================================
  // Fetch overview
  // ==========================================================================

  const fetchOverview = useCallback(async () => {
    setLoading(true)
    try {
      const response = await worklog.getOverview(startDate, endDate)
      setDays(response.days)
    } catch (err) {
      console.error('Failed to fetch worklog overview:', err)
      setDays([])
    } finally {
      setLoading(false)
    }
  }, [startDate, endDate])

  useEffect(() => {
    if (!isAuthenticated) return
    fetchOverview()
  }, [isAuthenticated, fetchOverview])

  // Gantt timeline fetch effect
  useEffect(() => {
    if (!isAuthenticated) return

    async function fetchTimeline() {
      setGanttLoading(true)
      try {
        // Pass sources filter (undefined if all sources selected to use backend defaults)
        const sourcesToPass = ganttSources.length === 2 ? undefined : ganttSources
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
  }, [ganttDate, ganttSources, isAuthenticated])

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
  // Hourly breakdown
  // ==========================================================================

  const toggleHourlyBreakdown = useCallback(async (date: string, projectPath: string) => {
    // Toggle off if same project
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
      fetchOverview()
    } catch (err) {
      console.error('Failed to create work item:', err)
    }
  }, [formData, resetForm, fetchOverview])

  const openEditModal = useCallback((item: WorkItem) => {
    setSelectedItem(item)

    // Extract project_name from title if it has [ProjectName] prefix
    let title = item.title
    let project_name = ''
    if (item.title.startsWith('[') && item.title.includes('] ')) {
      const endIndex = item.title.indexOf('] ')
      project_name = item.title.substring(1, endIndex)
      title = item.title.substring(endIndex + 2)
    }

    setFormData({
      title,
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
      fetchOverview()
    } catch (err) {
      console.error('Failed to update work item:', err)
    }
  }, [selectedItem, formData, resetForm, fetchOverview])

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
      fetchOverview()
    } catch (err) {
      console.error('Failed to delete work item:', err)
    }
  }, [itemToDelete, fetchOverview])

  // For editing manual items inline from the worklog view
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

  return {
    // Date range
    startDate,
    endDate,
    goToPreviousWeek,
    goToNextWeek,
    goToThisWeek,
    isCurrentWeek,
    // Data
    days,
    loading,
    // Hourly breakdown
    expandedProject,
    hourlyData,
    hourlyLoading,
    toggleHourlyBreakdown,
    // Gantt chart
    ganttDate,
    setGanttDate,
    ganttSessions,
    ganttLoading,
    ganttSources,
    setGanttSources,
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
    fetchOverview,
  }
}
