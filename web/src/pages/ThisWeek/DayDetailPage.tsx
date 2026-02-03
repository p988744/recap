import { useState, useEffect, useCallback, useMemo } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { ArrowLeft, Clock, FolderKanban, GitCommit, Plus, Upload } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { useAuth } from '@/lib/auth'
import { useDayDetail } from './hooks/useDayDetail'
import { ProjectCard } from '@/pages/Worklog/components/ProjectCard'
import { ManualItemCard } from '@/pages/Worklog/components/ManualItemCard'
import { DayGanttChart } from './components'
import {
  CreateModal,
  EditModal,
  DeleteModal,
} from '../WorkItems/components/Modals'
import { TempoBatchSyncModal } from '../Worklog/components'
import { useTempoSync } from '../Worklog/hooks'
import { workItems, config as configService } from '@/services'
import type { WorkItem, BatchSyncRow } from '@/types'

// Get weekday label in Chinese based on actual day of week (0=Sunday, 1=Monday, ...)
function getWeekdayLabel(dayOfWeek: number): string {
  const labels = ['週日', '週一', '週二', '週三', '週四', '週五', '週六']
  return labels[dayOfWeek] || ''
}

// Format date for display
function formatDateDisplay(dateStr: string): string {
  const d = new Date(dateStr + 'T00:00:00')
  const weekday = getWeekdayLabel(d.getDay())
  return `${weekday} ${d.getFullYear()}/${String(d.getMonth() + 1).padStart(2, '0')}/${String(d.getDate()).padStart(2, '0')}`
}

// Format date for export label (MM/DD)
function formatExportDate(dateStr: string): string {
  const d = new Date(dateStr + 'T00:00:00')
  return `${String(d.getMonth() + 1).padStart(2, '0')}/${String(d.getDate()).padStart(2, '0')}`
}

interface WorkItemFormData {
  title: string
  description: string
  hours: number
  date: string
  jira_issue_key: string
  category: string
  project_name: string
}

export function DayDetailPage() {
  const { date } = useParams<{ date: string }>()
  const navigate = useNavigate()
  const { isAuthenticated } = useAuth()
  const {
    day,
    loading,
    totalHours,
    totalCommits,
    projectCount,
    expandedProject,
    hourlyData,
    hourlyLoading,
    toggleHourlyBreakdown,
    fetchData,
  } = useDayDetail(date ?? '', isAuthenticated)

  // Jira config state
  const [jiraConfigured, setJiraConfigured] = useState(false)

  // CRUD state
  const [showCreateModal, setShowCreateModal] = useState(false)
  const [showEditModal, setShowEditModal] = useState(false)
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false)
  const [selectedItem, setSelectedItem] = useState<WorkItem | null>(null)
  const [itemToDelete, setItemToDelete] = useState<WorkItem | null>(null)
  const [formData, setFormData] = useState<WorkItemFormData>({
    title: '',
    description: '',
    hours: 0,
    date: date ?? new Date().toISOString().split('T')[0],
    jira_issue_key: '',
    category: '',
    project_name: '',
  })

  // Export modal state
  const [showExportModal, setShowExportModal] = useState(false)

  // Tempo sync hook (for sync records lookup)
  const ts = useTempoSync(
    isAuthenticated,
    date ?? '',
    date ?? '',
    day ? [day] : [],
    fetchData,
  )

  // Fetch Jira config
  useEffect(() => {
    if (!isAuthenticated) return
    configService.getConfig()
      .then((c) => setJiraConfigured(c.jira_configured))
      .catch(() => {})
  }, [isAuthenticated])

  // Build batch sync rows for export
  const batchRows: BatchSyncRow[] = useMemo(() => {
    if (!day || !date) return []
    const rows: BatchSyncRow[] = []

    for (const p of day.projects) {
      const existing = ts.getSyncRecord(p.project_path, date)
      if (existing) continue
      rows.push({
        projectPath: p.project_path,
        projectName: p.project_name,
        issueKey: ts.getMappedIssueKey(p.project_path),
        hours: p.total_hours,
        description: p.daily_summary ?? '',
        isManual: false,
      })
    }

    for (const m of day.manual_items) {
      const existing = ts.getSyncRecord(`manual:${m.id}`, date)
      if (existing) continue
      rows.push({
        projectPath: `manual:${m.id}`,
        projectName: m.title,
        issueKey: ts.getMappedIssueKey(`manual:${m.id}`),
        hours: m.hours,
        description: m.description ?? m.title,
        isManual: true,
        manualItemId: m.id,
      })
    }

    return rows
  }, [day, date, ts])

  // CRUD handlers
  const resetForm = useCallback(() => {
    setFormData({
      title: '',
      description: '',
      hours: 0,
      date: date ?? new Date().toISOString().split('T')[0],
      jira_issue_key: '',
      category: '',
      project_name: '',
    })
  }, [date])

  const openCreateModal = useCallback(() => {
    resetForm()
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

  const confirmDeleteManualItem = useCallback(async (id: string) => {
    try {
      const item = await workItems.get(id)
      setItemToDelete(item)
      setShowDeleteConfirm(true)
    } catch (err) {
      console.error('Failed to get work item:', err)
    }
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

  if (!date) {
    return (
      <div className="p-8 text-center">
        <p className="text-muted-foreground">Invalid date</p>
      </div>
    )
  }

  if (loading) {
    return (
      <div className="space-y-8">
        {/* Back button */}
        <Button variant="ghost" size="sm" onClick={() => navigate('/')}>
          <ArrowLeft className="w-4 h-4 mr-2" strokeWidth={1.5} />
          返回本週工作
        </Button>

        <div className="flex items-center justify-center h-48">
          <div className="w-6 h-6 border border-border border-t-foreground/60 rounded-full animate-spin" />
        </div>
      </div>
    )
  }

  const hasProjects = day && day.projects.length > 0
  const hasManualItems = day && day.manual_items.length > 0
  const isEmpty = !hasProjects && !hasManualItems

  return (
    <div className="space-y-8 animate-fade-up">
      {/* Sticky Header */}
      <div className="sticky top-0 z-10 -mx-12 px-12 py-4 bg-background/95 backdrop-blur-sm border-b border-transparent">
        <div className="flex items-center justify-between">
          {/* Left side: Back button + Title */}
          <div className="flex items-center gap-4">
            <Button variant="ghost" size="sm" onClick={() => navigate('/')}>
              <ArrowLeft className="w-4 h-4 mr-2" strokeWidth={1.5} />
              返回本週工作
            </Button>
            <div>
              <h1 className="text-2xl font-semibold text-foreground">
                {formatDateDisplay(date)}
              </h1>
              {!isEmpty && (
                <div className="flex items-center gap-4 text-sm text-muted-foreground mt-1">
                  <span className="flex items-center gap-1">
                    <Clock className="w-3.5 h-3.5" strokeWidth={1.5} />
                    總工時: {totalHours.toFixed(1)}h
                  </span>
                  <span className="flex items-center gap-1">
                    <FolderKanban className="w-3.5 h-3.5" strokeWidth={1.5} />
                    專案數: {projectCount}
                  </span>
                  <span className="flex items-center gap-1">
                    <GitCommit className="w-3.5 h-3.5" strokeWidth={1.5} />
                    Commits: {totalCommits}
                  </span>
                </div>
              )}
            </div>
          </div>

          {/* Right side: Actions */}
          <div className="flex items-center gap-1.5">
            {jiraConfigured && (
              <Button
                variant="ghost"
                size="icon"
                onClick={() => setShowExportModal(true)}
                title={`Export Day: ${formatExportDate(date)}`}
              >
                <Upload className="w-4 h-4" strokeWidth={1.5} />
              </Button>
            )}
            <Button
              variant="ghost"
              size="icon"
              onClick={openCreateModal}
              title="新增項目"
            >
              <Plus className="w-4 h-4" strokeWidth={1.5} />
            </Button>
          </div>
        </div>
      </div>

      {/* Content */}
      {isEmpty ? (
        <Card>
          <CardContent className="py-16 text-center">
            <p className="text-muted-foreground mb-4">當日無工作紀錄</p>
            <Button variant="outline" onClick={openCreateModal}>
              <Plus className="w-4 h-4 mr-2" strokeWidth={1.5} />
              新增項目
            </Button>
          </CardContent>
        </Card>
      ) : (
        <div className="space-y-6">
          {/* Gantt Chart - hourly timeline */}
          {(hasProjects || hasManualItems) && (
            <Card>
              <CardContent className="py-4">
                <DayGanttChart date={date} projects={day.projects} manualItems={day.manual_items} />
              </CardContent>
            </Card>
          )}

          {/* Projects */}
          {hasProjects && (
            <section className="space-y-3">
              <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
                專案工作
              </h2>
              {day.projects.map((project) => {
                const isExpanded = expandedProject === project.project_path
                return (
                  <ProjectCard
                    key={project.project_path}
                    project={project}
                    date={date}
                    isExpanded={isExpanded}
                    hourlyData={isExpanded ? hourlyData : []}
                    hourlyLoading={isExpanded ? hourlyLoading : false}
                    onToggleHourly={() => toggleHourlyBreakdown(project.project_path)}
                    syncRecord={jiraConfigured ? ts.getSyncRecord(project.project_path, date) : undefined}
                    onSyncToTempo={jiraConfigured ? () => ts.openSyncModal({
                      projectPath: project.project_path,
                      projectName: project.project_name,
                      date: date,
                      weekday: getWeekdayLabel(new Date(date + 'T00:00:00').getDay()),
                      hours: project.total_hours,
                      description: project.daily_summary ?? '',
                    }) : undefined}
                    mappedIssueKey={jiraConfigured ? ts.getMappedIssueKey(project.project_path) : undefined}
                    onIssueKeyChange={jiraConfigured ? (key) => ts.updateIssueKey(project.project_path, key) : undefined}
                  />
                )
              })}
            </section>
          )}

          {/* Manual items */}
          {hasManualItems && (
            <section className="space-y-3">
              <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
                手動項目
              </h2>
              {day.manual_items.map((item) => (
                <ManualItemCard
                  key={item.id}
                  item={item}
                  onEdit={() => openEditManualItem(item.id)}
                  onDelete={() => confirmDeleteManualItem(item.id)}
                />
              ))}
            </section>
          )}
        </div>
      )}

      {/* CRUD Modals */}
      <CreateModal
        open={showCreateModal}
        onOpenChange={(open) => { if (!open) closeCreateModal() }}
        formData={formData}
        setFormData={setFormData}
        onSubmit={handleCreate}
        onCancel={closeCreateModal}
      />

      <EditModal
        open={showEditModal}
        onOpenChange={(open) => { if (!open) closeEditModal() }}
        formData={formData}
        setFormData={setFormData}
        onSubmit={handleUpdate}
        onCancel={closeEditModal}
      />

      <DeleteModal
        open={showDeleteConfirm}
        onOpenChange={(open) => { if (!open) closeDeleteConfirm() }}
        itemToDelete={itemToDelete}
        onConfirm={handleDelete}
        onCancel={closeDeleteConfirm}
      />

      {/* Tempo Export Modal */}
      {jiraConfigured && (
        <TempoBatchSyncModal
          open={showExportModal}
          date={date}
          weekday={getWeekdayLabel(new Date(date + 'T00:00:00').getDay())}
          initialRows={batchRows}
          syncing={ts.syncing}
          syncResult={ts.syncResult}
          onSync={ts.executeBatchSync}
          onClose={() => setShowExportModal(false)}
        />
      )}
    </div>
  )
}
