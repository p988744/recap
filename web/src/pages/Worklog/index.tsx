import { useMemo } from 'react'
import { Plus, Upload } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useAuth } from '@/lib/auth'
import { useWorklog, useTempoSync } from './hooks'
import { DateRangeBar, DaySection, TempoSyncModal, TempoBatchSyncModal, TempoWeekSyncModal } from './components'
import {
  CreateModal,
  EditModal,
  DeleteModal,
} from '../WorkItems/components/Modals'
import type { BatchSyncRow } from '@/types'

export function WorklogPage() {
  const { isAuthenticated } = useAuth()

  const wl = useWorklog(isAuthenticated)

  const ts = useTempoSync(
    isAuthenticated,
    wl.startDate,
    wl.endDate,
    wl.days,
    wl.fetchOverview,
  )

  // Build batch sync rows for the selected day
  const batchRows: BatchSyncRow[] = useMemo(() => {
    if (!ts.batchSyncDate) return []
    const day = wl.days.find((d) => d.date === ts.batchSyncDate)
    if (!day) return []

    const rows: BatchSyncRow[] = []
    for (const p of day.projects) {
      // Skip already synced projects
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
  }, [ts.batchSyncDate, wl.days, ts.getSyncRecord, ts.getMappedIssueKey])

  // Build week sync rows (all days, all projects + manual items, skip synced)
  const weekRows: BatchSyncRow[] = useMemo(() => {
    const rows: BatchSyncRow[] = []
    for (const day of wl.days) {
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
  }, [wl.days, ts.getSyncRecord, ts.getMappedIssueKey])

  // Loading state
  if (wl.loading) {
    return (
      <div className="space-y-12">
        <header className="animate-fade-up opacity-0 delay-1">
          <div className="flex items-start justify-between mb-6">
            <div>
              <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
                紀錄
              </p>
              <h1 className="font-display text-4xl text-foreground tracking-tight">工作日誌</h1>
            </div>
          </div>
          <DateRangeBar
            startDate={wl.startDate}
            endDate={wl.endDate}
            isCurrentWeek={wl.isCurrentWeek}
            onPrev={wl.goToPreviousWeek}
            onNext={wl.goToNextWeek}
            onToday={wl.goToThisWeek}
          />
        </header>
        <div className="flex items-center justify-center h-48">
          <div className="w-6 h-6 border border-border border-t-foreground/60 rounded-full animate-spin" />
        </div>
      </div>
    )
  }

  return (
    <div className="space-y-12">
      {/* Header */}
      <header className="animate-fade-up opacity-0 delay-1">
        <div className="flex items-start justify-between mb-6">
          <div>
            <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
              紀錄
            </p>
            <h1 className="font-display text-4xl text-foreground tracking-tight">工作日誌</h1>
          </div>
          <div className="flex items-center gap-2">
            {wl.jiraConfigured && (
              <Button variant="outline" onClick={ts.openWeekSyncModal}>
                <Upload className="w-4 h-4 mr-2" strokeWidth={1.5} />
                Export Week
              </Button>
            )}
            <Button onClick={() => wl.openCreateModal()}>
              <Plus className="w-4 h-4 mr-2" strokeWidth={1.5} />
              新增項目
            </Button>
          </div>
        </div>
        <DateRangeBar
          startDate={wl.startDate}
          endDate={wl.endDate}
          isCurrentWeek={wl.isCurrentWeek}
          onPrev={wl.goToPreviousWeek}
          onNext={wl.goToNextWeek}
          onToday={wl.goToThisWeek}
        />
      </header>

      {/* Day sections */}
      <section className="space-y-8 animate-fade-up opacity-0 delay-2">
        {wl.days.length === 0 ? (
          <div className="py-16 text-center">
            <p className="text-sm text-muted-foreground mb-4">本週尚無工作紀錄</p>
            <Button variant="outline" onClick={() => wl.openCreateModal()}>
              <Plus className="w-4 h-4 mr-2" strokeWidth={1.5} />
              新增手動工作項目
            </Button>
          </div>
        ) : (
          wl.days.map((day) => (
            <DaySection
              key={day.date}
              day={day}
              expandedProject={wl.expandedProject}
              hourlyData={wl.hourlyData}
              hourlyLoading={wl.hourlyLoading}
              onToggleHourly={wl.toggleHourlyBreakdown}
              onAddManualItem={wl.openCreateModal}
              onEditManualItem={wl.openEditManualItem}
              onDeleteManualItem={wl.confirmDeleteManualItem}
              getSyncRecord={wl.jiraConfigured ? ts.getSyncRecord : undefined}
              onSyncProject={wl.jiraConfigured ? ts.openSyncModal : undefined}
              onSyncDay={wl.jiraConfigured ? ts.openBatchSyncModal : undefined}
              getMappedIssueKey={wl.jiraConfigured ? ts.getMappedIssueKey : undefined}
              onIssueKeyChange={wl.jiraConfigured ? ts.updateIssueKey : undefined}
            />
          ))
        )}
      </section>

      {/* CRUD Modals — reuse from WorkItems */}
      <CreateModal
        open={wl.showCreateModal}
        onOpenChange={(open) => { if (!open) wl.closeCreateModal() }}
        formData={wl.formData}
        setFormData={wl.setFormData}
        onSubmit={wl.handleCreate}
        onCancel={wl.closeCreateModal}
      />

      <EditModal
        open={wl.showEditModal}
        onOpenChange={(open) => { if (!open) wl.closeEditModal() }}
        formData={wl.formData}
        setFormData={wl.setFormData}
        onSubmit={wl.handleUpdate}
        onCancel={wl.closeEditModal}
      />

      <DeleteModal
        open={wl.showDeleteConfirm}
        onOpenChange={(open) => { if (!open) wl.closeDeleteConfirm() }}
        itemToDelete={wl.itemToDelete}
        onConfirm={wl.handleDelete}
        onCancel={wl.closeDeleteConfirm}
      />

      {/* Tempo Sync Modals (only when Jira configured) */}
      {wl.jiraConfigured && (
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
            startDate={wl.startDate}
            endDate={wl.endDate}
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
