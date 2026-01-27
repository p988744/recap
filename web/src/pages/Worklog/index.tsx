import { Plus } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useAuth } from '@/lib/auth'
import { useWorklog } from './hooks'
import { DateRangeBar, DaySection } from './components'
import {
  CreateModal,
  EditModal,
  DeleteModal,
} from '../WorkItems/components/Modals'

export function WorklogPage() {
  const { isAuthenticated } = useAuth()

  const wl = useWorklog(isAuthenticated)

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
          <Button onClick={() => wl.openCreateModal()}>
            <Plus className="w-4 h-4 mr-2" strokeWidth={1.5} />
            新增項目
          </Button>
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
            />
          ))
        )}
      </section>

      {/* Modals — reuse from WorkItems */}
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
    </div>
  )
}
