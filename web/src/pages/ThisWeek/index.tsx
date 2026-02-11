import { useMemo } from 'react'
import { Plus, Upload } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { useAuth } from '@/lib/auth'
import { useThisWeek } from './hooks'
import { WeekHeader, WeekOverview, TodayWorkSection, WeekTimelineSection } from './components'
import {
  CreateModal,
  EditModal,
  DeleteModal,
} from '../WorkItems/components/Modals'
import { HttpExportModal } from '../WorkItems/components/HttpExportModal'
import {
  TempoSyncModal,
  TempoBatchSyncModal,
  TempoWeekSyncModal,
} from '../Worklog/components'
import { useTempoSync } from '../Worklog/hooks'
import { useHttpExport } from '../WorkItems/hooks/useHttpExport'
import type { BatchSyncRow, WorkItem } from '@/types'

export function ThisWeekPage() {
  const { isAuthenticated } = useAuth()
  const tw = useThisWeek(isAuthenticated)

  // HTTP Export
  const httpExp = useHttpExport(isAuthenticated)

  const exportItems: WorkItem[] = useMemo(() => {
    const items: WorkItem[] = []
    for (const day of tw.days) {
      for (const p of day.projects) {
        items.push({
          id: `${day.date}:${p.project_path}`,
          title: p.project_name,
          description: p.daily_summary ?? '',
          hours: p.total_hours,
          date: day.date,
          source: 'auto',
          jira_issue_key: '',
          category: '',
          user_id: '',
          created_at: '',
          updated_at: '',
          synced_to_tempo: false,
        } as WorkItem)
      }
      for (const m of day.manual_items) {
        items.push({
          id: m.id,
          title: m.title,
          description: m.description ?? '',
          hours: m.hours,
          date: m.date,
          source: 'manual',
          jira_issue_key: m.jira_issue_key ?? '',
          category: '',
          user_id: '',
          created_at: '',
          updated_at: '',
          synced_to_tempo: false,
        } as WorkItem)
      }
    }
    return items
  }, [tw.days])

  const ts = useTempoSync(
    isAuthenticated,
    tw.startDate,
    tw.endDate,
    tw.days,
    tw.fetchData,
  )

  // Build batch sync rows for the selected day
  const batchRows: BatchSyncRow[] = useMemo(() => {
    if (!ts.batchSyncDate) return []
    const day = tw.days.find((d) => d.date === ts.batchSyncDate)
    if (!day) return []

    const rows: BatchSyncRow[] = []
    for (const p of day.projects) {
      const existing = ts.getSyncRecord(p.project_path, day.date)
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
      const existing = ts.getSyncRecord(`manual:${m.id}`, day.date)
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
  }, [ts.batchSyncDate, tw.days, ts.getSyncRecord, ts.getMappedIssueKey])

  // Build week sync rows
  const weekRows: BatchSyncRow[] = useMemo(() => {
    const rows: BatchSyncRow[] = []
    for (const day of tw.days) {
      for (const p of day.projects) {
        const existing = ts.getSyncRecord(p.project_path, day.date)
        if (existing) continue
        rows.push({
          projectPath: p.project_path,
          projectName: p.project_name,
          issueKey: ts.getMappedIssueKey(p.project_path),
          hours: p.total_hours,
          description: p.daily_summary ?? '',
          isManual: false,
          date: day.date,
        })
      }
      for (const m of day.manual_items) {
        const existing = ts.getSyncRecord(`manual:${m.id}`, day.date)
        if (existing) continue
        rows.push({
          projectPath: `manual:${m.id}`,
          projectName: m.title,
          issueKey: ts.getMappedIssueKey(`manual:${m.id}`),
          hours: m.hours,
          description: m.description ?? m.title,
          isManual: true,
          manualItemId: m.id,
          date: day.date,
        })
      }
    }
    return rows
  }, [tw.days, ts.getSyncRecord, ts.getMappedIssueKey])

  // Loading state
  if (tw.loading) {
    return (
      <div className="space-y-10">
        <div className="sticky top-0 z-10 -mx-12 px-12 py-4 bg-background/95 backdrop-blur-sm">
          <WeekHeader
            weekNumber={tw.weekNumber}
            startDate={tw.startDate}
            endDate={tw.endDate}
            isCurrentWeek={tw.isCurrentWeek}
            onPrev={tw.goToPreviousWeek}
            onNext={tw.goToNextWeek}
            onToday={tw.goToThisWeek}
          />
        </div>
        <div className="flex items-center justify-center h-48">
          <div className="w-6 h-6 border border-border border-t-foreground/60 rounded-full animate-spin" />
        </div>
      </div>
    )
  }

  return (
    <div className="space-y-10">
      {/* Sticky Header with actions */}
      <div className="sticky top-0 z-10 -mx-12 px-12 py-4 bg-background/95 backdrop-blur-sm border-b border-transparent [&:not(:first-child)]:border-border">
        <div className="flex items-center justify-between">
          <WeekHeader
            weekNumber={tw.weekNumber}
            startDate={tw.startDate}
            endDate={tw.endDate}
            isCurrentWeek={tw.isCurrentWeek}
            onPrev={tw.goToPreviousWeek}
            onNext={tw.goToNextWeek}
            onToday={tw.goToThisWeek}
          />
          <div className="flex items-center gap-1.5">
            {(tw.jiraConfigured || (httpExp.hasConfigs && exportItems.length > 0)) && (
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button variant="ghost" size="icon" title="匯出">
                    <Upload className="w-4 h-4" strokeWidth={1.5} />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end">
                  {tw.jiraConfigured && (
                    <>
                      <DropdownMenuLabel>Tempo</DropdownMenuLabel>
                      <DropdownMenuItem onClick={ts.openWeekSyncModal}>
                        匯出本週到 Tempo
                      </DropdownMenuItem>
                    </>
                  )}
                  {tw.jiraConfigured && httpExp.hasConfigs && exportItems.length > 0 && (
                    <DropdownMenuSeparator />
                  )}
                  {httpExp.hasConfigs && exportItems.length > 0 && (
                    <>
                      <DropdownMenuLabel>HTTP Export</DropdownMenuLabel>
                      {httpExp.configs.map((c) => (
                        <DropdownMenuItem
                          key={c.id}
                          onClick={() => {
                            httpExp.setSelectedConfigId(c.id)
                            httpExp.openExport(exportItems)
                          }}
                        >
                          {c.name}
                        </DropdownMenuItem>
                      ))}
                    </>
                  )}
                </DropdownMenuContent>
              </DropdownMenu>
            )}
            <Button variant="ghost" size="icon" onClick={() => tw.openCreateModal()} title="新增項目">
              <Plus className="w-4 h-4" strokeWidth={1.5} />
            </Button>
          </div>
        </div>
      </div>

      {/* Week Overview */}
      <WeekOverview
        days={tw.days}
        startDate={tw.startDate}
        endDate={tw.endDate}
      />

      {/* Today's Work - only show when viewing current week */}
      {tw.isCurrentWeek && (
        <TodayWorkSection
          day={tw.days.find(d => d.date === tw.today) ?? null}
          expandedProject={tw.expandedProject}
          hourlyData={tw.hourlyData}
          hourlyLoading={tw.hourlyLoading}
          onToggleHourly={tw.toggleHourlyBreakdown}
          onAddManualItem={tw.openCreateModal}
          onEditManualItem={tw.openEditManualItem}
          onDeleteManualItem={tw.confirmDeleteManualItem}
          getSyncRecord={tw.jiraConfigured ? ts.getSyncRecord : undefined}
          onSyncProject={tw.jiraConfigured ? ts.openSyncModal : undefined}
          getMappedIssueKey={tw.jiraConfigured ? ts.getMappedIssueKey : undefined}
          onIssueKeyChange={tw.jiraConfigured ? ts.updateIssueKey : undefined}
        />
      )}

      {/* Week Timeline - summary cards that navigate to day details */}
      <WeekTimelineSection
        days={tw.days}
        today={tw.isCurrentWeek ? tw.today : undefined}
      />

      {/* CRUD Modals */}
      <CreateModal
        open={tw.showCreateModal}
        onOpenChange={(open) => { if (!open) tw.closeCreateModal() }}
        formData={tw.formData}
        setFormData={tw.setFormData}
        onSubmit={tw.handleCreate}
        onCancel={tw.closeCreateModal}
      />

      <EditModal
        open={tw.showEditModal}
        onOpenChange={(open) => { if (!open) tw.closeEditModal() }}
        formData={tw.formData}
        setFormData={tw.setFormData}
        onSubmit={tw.handleUpdate}
        onCancel={tw.closeEditModal}
      />

      <DeleteModal
        open={tw.showDeleteConfirm}
        onOpenChange={(open) => { if (!open) tw.closeDeleteConfirm() }}
        itemToDelete={tw.itemToDelete}
        onConfirm={tw.handleDelete}
        onCancel={tw.closeDeleteConfirm}
      />

      {/* HTTP Export Modal */}
      <HttpExportModal
        open={httpExp.showModal}
        onOpenChange={(open) => { if (!open) httpExp.closeModal() }}
        configs={httpExp.configs}
        selectedConfigId={httpExp.selectedConfigId}
        onConfigChange={httpExp.setSelectedConfigId}
        items={httpExp.itemsToExport}
        result={httpExp.result}
        exporting={httpExp.exporting}
        exportedIds={httpExp.exportedIds}
        onExport={httpExp.executeExport}
        onClose={httpExp.closeModal}
      />

      {/* Tempo Sync Modals (only when Jira configured) */}
      {tw.jiraConfigured && (
        <>
          <TempoSyncModal
            target={ts.syncTarget}
            defaultIssueKey={ts.syncTarget ? ts.getMappedIssueKey(ts.syncTarget.projectPath) : ''}
            syncing={ts.syncing}
            syncResult={ts.syncResult}
            onSync={ts.executeSingleSync}
            onClose={ts.closeSyncModal}
          />

          <TempoBatchSyncModal
            open={!!ts.batchSyncDate}
            date={ts.batchSyncDate ?? ''}
            weekday={ts.batchSyncWeekday}
            initialRows={batchRows}
            syncing={ts.syncing}
            syncResult={ts.syncResult}
            onSync={ts.executeBatchSync}
            onClose={ts.closeBatchSyncModal}
          />

          <TempoWeekSyncModal
            open={ts.weekSyncOpen}
            startDate={tw.startDate}
            endDate={tw.endDate}
            initialRows={weekRows}
            syncing={ts.syncing}
            syncResult={ts.syncResult}
            onSync={ts.executeWeekSync}
            onClose={ts.closeWeekSyncModal}
          />
        </>
      )}
    </div>
  )
}

// Re-export detail pages
export { DayDetailPage } from './DayDetailPage'
export { ProjectDayDetailPage } from './ProjectDayDetailPage'
