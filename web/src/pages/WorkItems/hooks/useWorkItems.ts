import { useEffect, useState, useMemo, useCallback } from 'react'
import { workItems } from '@/services'
import type {
  WorkItem,
  WorkItemWithChildren,
  WorkItemFilters,
  WorkItemStatsResponse,
  GroupedWorkItemsResponse,
} from '@/types'
import type { ViewMode } from '@/components/ViewModeSwitcher'
import type { TimelineSession, ProjectGroup } from './types'
import type { QuickPickItem } from './useRecentManualItems'

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

export interface SettingsMessage {
  type: 'success' | 'error'
  text: string
}

// =============================================================================
// Helpers
// =============================================================================

export function deriveProjectName(item: { project_path?: string | null; title: string }): string {
  if (item.project_path) {
    const segments = item.project_path.split(/[/\\]/)
    return segments[segments.length - 1] || ''
  }
  // Legacy: [ProjectName] Title 格式
  if (item.title.startsWith('[') && item.title.includes('] ')) {
    return item.title.substring(1, item.title.indexOf('] '))
  }
  return ''
}

// =============================================================================
// Main Hook: useWorkItems
// =============================================================================

export function useWorkItems(isAuthenticated: boolean, token: string | null) {
  // Core state
  const [items, setItems] = useState<WorkItemWithChildren[]>([])
  const [stats, setStats] = useState<WorkItemStatsResponse | null>(null)
  const [loading, setLoading] = useState(true)
  const [page, setPage] = useState(1)
  const [totalPages, setTotalPages] = useState(1)
  const [total, setTotal] = useState(0)
  const [filters, setFilters] = useState<WorkItemFilters>({ per_page: 20 })

  // View mode state
  const [viewMode, setViewMode] = useState<ViewMode>('list')
  const [groupedData, setGroupedData] = useState<GroupedWorkItemsResponse | null>(null)
  const [timelineDate, setTimelineDate] = useState(() => new Date().toISOString().split('T')[0])
  const [timelineSessions, setTimelineSessions] = useState<TimelineSession[]>([])
  const [timelineLoading, setTimelineLoading] = useState(false)
  const [timelineSources, setTimelineSources] = useState<string[]>(['claude_code'])

  // UI state
  const [searchTerm, setSearchTerm] = useState('')
  const [showFilters, setShowFilters] = useState(false)

  // Expand state for child items
  const [expandedItems, setExpandedItems] = useState<Set<string>>(new Set())
  const [childrenData, setChildrenData] = useState<Record<string, WorkItem[]>>({})
  const [loadingChildren, setLoadingChildren] = useState<Set<string>>(new Set())

  // Aggregate state
  const [aggregating, setAggregating] = useState(false)
  const [aggregateResult, setAggregateResult] = useState<string | null>(null)

  // Fetch functions
  const fetchWorkItems = useCallback(async () => {
    setLoading(true)
    try {
      const response = await workItems.list({ ...filters, page })
      setItems(response.items)
      setTotalPages(response.pages)
      setTotal(response.total)
    } catch (err) {
      console.error('Failed to fetch work items:', err)
    } finally {
      setLoading(false)
    }
  }, [filters, page])

  const fetchStats = useCallback(async () => {
    try {
      const response = await workItems.getStats()
      setStats(response)
    } catch (err) {
      console.error('Failed to fetch stats:', err)
    }
  }, [])

  const fetchGroupedData = useCallback(async () => {
    setLoading(true)
    try {
      const response = await workItems.getGrouped({
        start_date: filters.start_date,
        end_date: filters.end_date,
      })
      setGroupedData(response)
    } catch (err) {
      console.error('Failed to fetch grouped data:', err)
    } finally {
      setLoading(false)
    }
  }, [filters.start_date, filters.end_date])

  const fetchTimelineData = useCallback(async () => {
    setTimelineLoading(true)
    try {
      // Pass sources filter (undefined if all sources selected to use backend defaults)
      const sourcesToPass = timelineSources.length === 1 && timelineSources[0] === 'claude_code' ? undefined : timelineSources
      const response = await workItems.getTimeline(timelineDate, sourcesToPass)
      const sessions: TimelineSession[] = response.sessions.map(s => ({
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
      setTimelineSessions(sessions)
    } catch (err) {
      console.error('Failed to fetch timeline:', err)
      setTimelineSessions([])
    } finally {
      setTimelineLoading(false)
    }
  }, [timelineDate, timelineSources])

  // Effects
  useEffect(() => {
    if (!isAuthenticated || !token) return

    if (viewMode === 'list') {
      fetchWorkItems()
    } else if (viewMode === 'project' || viewMode === 'task') {
      fetchGroupedData()
    }
    fetchStats()
  }, [page, filters, viewMode, isAuthenticated, token, fetchWorkItems, fetchGroupedData, fetchStats])

  useEffect(() => {
    if (!isAuthenticated || !token) return

    if (viewMode === 'timeline') {
      fetchTimelineData()
    }
  }, [viewMode, timelineDate, timelineSources, isAuthenticated, token, fetchTimelineData])

  // Handlers
  const handleSearch = useCallback((e: React.FormEvent) => {
    e.preventDefault()
    setPage(1)
    setFilters(prev => ({ ...prev, search: searchTerm || undefined }))
  }, [searchTerm])

  const handleAggregate = useCallback(async () => {
    setAggregating(true)
    setAggregateResult(null)
    try {
      const result = await workItems.aggregate({ source: filters.source })
      setAggregateResult(
        `已彙整 ${result.original_count} 個項目為 ${result.aggregated_count} 組，包含 ${result.deleted_count} 個子項目`
      )
      setPage(1)
      setExpandedItems(new Set())
      setChildrenData({})
      fetchWorkItems()
      fetchStats()
    } catch (err) {
      console.error('Failed to aggregate work items:', err)
      setAggregateResult('彙整失敗，請稀後再試')
    } finally {
      setAggregating(false)
    }
  }, [filters.source, fetchWorkItems, fetchStats])

  const toggleExpand = useCallback(async (itemId: string) => {
    const newExpanded = new Set(expandedItems)

    if (newExpanded.has(itemId)) {
      newExpanded.delete(itemId)
      setExpandedItems(newExpanded)
    } else {
      newExpanded.add(itemId)
      setExpandedItems(newExpanded)

      if (!childrenData[itemId]) {
        setLoadingChildren(prev => new Set(prev).add(itemId))
        try {
          const response = await workItems.list({ parent_id: itemId, per_page: 100 })
          setChildrenData(prev => ({ ...prev, [itemId]: response.items }))
        } catch (err) {
          console.error('Failed to fetch children:', err)
        } finally {
          setLoadingChildren(prev => {
            const newSet = new Set(prev)
            newSet.delete(itemId)
            return newSet
          })
        }
      }
    }
  }, [expandedItems, childrenData])

  const clearFilters = useCallback(() => {
    setPage(1)
    setFilters({ per_page: 20 })
    setSearchTerm('')
  }, [])

  const clearAggregateResult = useCallback(() => {
    setAggregateResult(null)
  }, [])

  // Computed data
  const projectGroups: ProjectGroup[] = useMemo(() => {
    if (!groupedData?.by_project) return []
    return groupedData.by_project.map(p => ({
      project_name: p.project_name,
      total_hours: p.total_hours,
      issues: p.issues.map(i => ({
        jira_key: i.jira_key || undefined,
        jira_title: i.jira_title || undefined,
        total_hours: i.total_hours,
        logs: i.logs.map(l => ({
          id: l.id,
          title: l.title,
          description: l.description,
          hours: l.hours,
          date: l.date,
          source: l.source,
          synced_to_tempo: l.synced_to_tempo,
        })),
      })),
    }))
  }, [groupedData])

  const taskGroups = useMemo(() => {
    if (!groupedData?.by_project) return []
    const taskMap = new Map<string, ProjectGroup['issues'][0]>()

    groupedData.by_project.forEach(project => {
      project.issues.forEach(issue => {
        const key = issue.jira_key || 'unmapped'
        const existing = taskMap.get(key)
        if (existing) {
          existing.total_hours += issue.total_hours
          existing.logs.push(...issue.logs)
        } else {
          taskMap.set(key, {
            jira_key: issue.jira_key || undefined,
            jira_title: issue.jira_title || undefined,
            total_hours: issue.total_hours,
            logs: [...issue.logs],
          })
        }
      })
    })

    return Array.from(taskMap.values()).sort((a, b) => b.total_hours - a.total_hours)
  }, [groupedData])

  return {
    // Data
    items,
    stats,
    loading,
    page,
    totalPages,
    total,
    filters,
    viewMode,
    groupedData,
    timelineDate,
    timelineSessions,
    timelineLoading,
    timelineSources,
    searchTerm,
    showFilters,
    expandedItems,
    childrenData,
    loadingChildren,
    aggregating,
    aggregateResult,
    projectGroups,
    taskGroups,
    // Setters
    setPage,
    setFilters,
    setViewMode,
    setTimelineDate,
    setTimelineSources,
    setSearchTerm,
    setShowFilters,
    // Actions
    fetchWorkItems,
    fetchStats,
    handleSearch,
    handleAggregate,
    toggleExpand,
    clearFilters,
    clearAggregateResult,
  }
}

// =============================================================================
// CRUD Operations Hook
// =============================================================================

export function useWorkItemCrud(
  fetchWorkItems: () => Promise<void>,
  fetchStats: () => Promise<void>
) {
  const [selectedItem, setSelectedItem] = useState<WorkItem | null>(null)
  const [showCreateModal, setShowCreateModal] = useState(false)
  const [showEditModal, setShowEditModal] = useState(false)
  const [showJiraModal, setShowJiraModal] = useState(false)
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false)
  const [itemToDelete, setItemToDelete] = useState<WorkItem | null>(null)

  // Form state
  const [formData, setFormData] = useState<WorkItemFormData>({
    title: '',
    description: '',
    hours: 0,
    date: new Date().toISOString().split('T')[0],
    jira_issue_key: '',
    category: '',
    project_name: '',
  })

  // Jira mapping state
  const [jiraKey, setJiraKey] = useState('')
  const [jiraTitle, setJiraTitle] = useState('')

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
      fetchWorkItems()
      fetchStats()
    } catch (err) {
      console.error('Failed to create work item:', err)
    }
  }, [formData, resetForm, fetchWorkItems, fetchStats])

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
      fetchWorkItems()
    } catch (err) {
      console.error('Failed to update work item:', err)
    }
  }, [selectedItem, formData, resetForm, fetchWorkItems])

  const handleDelete = useCallback(async () => {
    if (!itemToDelete) return
    try {
      await workItems.remove(itemToDelete.id)
      setShowDeleteConfirm(false)
      setItemToDelete(null)
      fetchWorkItems()
      fetchStats()
    } catch (err) {
      console.error('Failed to delete work item:', err)
    }
  }, [itemToDelete, fetchWorkItems, fetchStats])

  const handleMapJira = useCallback(async (e: React.FormEvent) => {
    e.preventDefault()
    if (!selectedItem) return
    try {
      await workItems.mapToJira(selectedItem.id, jiraKey, jiraTitle || undefined)
      setShowJiraModal(false)
      setSelectedItem(null)
      setJiraKey('')
      setJiraTitle('')
      fetchWorkItems()
      fetchStats()
    } catch (err) {
      console.error('Failed to map Jira issue:', err)
    }
  }, [selectedItem, jiraKey, jiraTitle, fetchWorkItems, fetchStats])

  const openEditModal = useCallback((item: WorkItem) => {
    setSelectedItem(item)
    setFormData({
      title: item.title,
      description: item.description || '',
      hours: item.hours,
      date: item.date,
      jira_issue_key: item.jira_issue_key || '',
      category: item.category || '',
      project_name: deriveProjectName(item),
    })
    setShowEditModal(true)
  }, [])

  const duplicateItem = useCallback((item: WorkItem) => {
    setFormData({
      title: item.title,
      description: item.description || '',
      hours: item.hours,
      date: new Date().toISOString().split('T')[0],
      jira_issue_key: item.jira_issue_key || '',
      category: item.category || '',
      project_name: deriveProjectName(item),
    })
    setShowCreateModal(true)
  }, [])

  const handleQuickPick = useCallback((item: QuickPickItem) => {
    setFormData({
      title: item.title,
      description: item.description,
      hours: item.hours,
      date: new Date().toISOString().split('T')[0],
      jira_issue_key: item.jira_issue_key,
      category: '',
      project_name: item.project_name,
    })
  }, [])

  const openJiraModal = useCallback((item: WorkItem) => {
    setSelectedItem(item)
    setJiraKey(item.jira_issue_key || '')
    setJiraTitle(item.jira_issue_title || '')
    setShowJiraModal(true)
  }, [])

  const confirmDelete = useCallback((item: WorkItem) => {
    setItemToDelete(item)
    setShowDeleteConfirm(true)
  }, [])

  const closeCreateModal = useCallback(() => {
    setShowCreateModal(false)
    resetForm()
  }, [resetForm])

  const closeEditModal = useCallback(() => {
    setShowEditModal(false)
    setSelectedItem(null)
    resetForm()
  }, [resetForm])

  const closeJiraModal = useCallback(() => {
    setShowJiraModal(false)
    setSelectedItem(null)
    setJiraKey('')
    setJiraTitle('')
  }, [])

  const closeDeleteConfirm = useCallback(() => {
    setShowDeleteConfirm(false)
    setItemToDelete(null)
  }, [])

  return {
    // State
    selectedItem,
    showCreateModal,
    showEditModal,
    showJiraModal,
    showDeleteConfirm,
    itemToDelete,
    formData,
    jiraKey,
    jiraTitle,
    // Setters
    setShowCreateModal,
    setFormData,
    setJiraKey,
    setJiraTitle,
    // Actions
    handleCreate,
    handleUpdate,
    handleDelete,
    handleMapJira,
    openEditModal,
    duplicateItem,
    handleQuickPick,
    openJiraModal,
    confirmDelete,
    closeCreateModal,
    closeEditModal,
    closeJiraModal,
    closeDeleteConfirm,
  }
}

// =============================================================================
// Constants
// =============================================================================

export const SOURCE_LABELS: Record<string, string> = {
  claude_code: 'Claude Code',
  gitlab: 'GitLab',
  manual: '手動',
}
