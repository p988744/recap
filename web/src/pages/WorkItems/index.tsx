import { useState, useEffect } from 'react'
import { Plus, Upload } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { TooltipProvider } from '@/components/ui/tooltip'
import { useAuth } from '@/lib/auth'
import { ViewModeSwitcher } from '@/components/ViewModeSwitcher'
import { useWorkItems, useWorkItemCrud, useRecentManualItems } from './hooks'
import { useHttpExport } from './hooks/useHttpExport'
import {
  StatsCards,
  SearchAndFilters,
  ListView,
  ProjectView,
  TaskView,
  TimelineView,
  ProjectDetailPanel,
  HttpExportModal,
  CreateModal,
  EditModal,
  JiraModal,
  DeleteModal,
} from './components'

export function WorkItemsPage() {
  const { token, isAuthenticated } = useAuth()

  // Main work items state and actions
  const workItemsState = useWorkItems(isAuthenticated, token)

  // CRUD operations
  const crud = useWorkItemCrud(workItemsState.fetchWorkItems, workItemsState.fetchStats)

  // Recent manual items for quick pick
  const { recentItems, refreshRecent } = useRecentManualItems()

  useEffect(() => {
    if (isAuthenticated) refreshRecent()
  }, [isAuthenticated, refreshRecent])

  // HTTP Export
  const httpExp = useHttpExport(isAuthenticated)

  // Project detail panel
  const [detailProjectName, setDetailProjectName] = useState<string | null>(null)

  // Loading state
  if (workItemsState.loading || (workItemsState.viewMode === 'timeline' && workItemsState.timelineLoading)) {
    return (
      <TooltipProvider>
        <div className="space-y-12">
          {/* Header */}
          <header className="animate-fade-up opacity-0 delay-1">
            <div className="flex items-start justify-between mb-6">
              <div>
                <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
                  管理
                </p>
                <h1 className="font-display text-4xl text-foreground tracking-tight">工作項目</h1>
              </div>
              <Button onClick={() => crud.setShowCreateModal(true)}>
                <Plus className="w-4 h-4 mr-2" strokeWidth={1.5} />
                新增項目
              </Button>
            </div>
            <ViewModeSwitcher value={workItemsState.viewMode} onChange={workItemsState.setViewMode} />
          </header>

          {/* Loading spinner */}
          <div className="flex items-center justify-center h-48">
            <div className="w-6 h-6 border border-border border-t-foreground/60 rounded-full animate-spin" />
          </div>
        </div>
      </TooltipProvider>
    )
  }

  return (
    <TooltipProvider>
      <div className="space-y-12">
        {/* Header */}
        <header className="animate-fade-up opacity-0 delay-1">
          <div className="flex items-start justify-between mb-6">
            <div>
              <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
                管理
              </p>
              <h1 className="font-display text-4xl text-foreground tracking-tight">工作項目</h1>
            </div>
            <div className="flex items-center gap-2">
              {httpExp.hasConfigs && (
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button variant="outline">
                      <Upload className="w-4 h-4 mr-2" strokeWidth={1.5} />
                      匯出
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
                    {httpExp.configs.map((c) => (
                      <DropdownMenuItem
                        key={c.id}
                        onClick={() => {
                          httpExp.setSelectedConfigId(c.id)
                          httpExp.openExport(workItemsState.items)
                        }}
                      >
                        {c.name}
                      </DropdownMenuItem>
                    ))}
                  </DropdownMenuContent>
                </DropdownMenu>
              )}
              <Button onClick={() => crud.setShowCreateModal(true)}>
                <Plus className="w-4 h-4 mr-2" strokeWidth={1.5} />
                新增項目
              </Button>
            </div>
          </div>
          <ViewModeSwitcher value={workItemsState.viewMode} onChange={workItemsState.setViewMode} />
        </header>

        {/* Stats Cards */}
        {workItemsState.stats && <StatsCards stats={workItemsState.stats} />}

        {/* Search and Filters */}
        <SearchAndFilters
          searchTerm={workItemsState.searchTerm}
          setSearchTerm={workItemsState.setSearchTerm}
          showFilters={workItemsState.showFilters}
          setShowFilters={workItemsState.setShowFilters}
          filters={workItemsState.filters}
          setFilters={workItemsState.setFilters}
          setPage={workItemsState.setPage}
          onSearch={workItemsState.handleSearch}
          onClearFilters={workItemsState.clearFilters}
        />

        {/* Content by View Mode */}
        <section className="animate-fade-up opacity-0 delay-4">
          {workItemsState.viewMode === 'list' && (
            <ListView
              items={workItemsState.items}
              total={workItemsState.total}
              page={workItemsState.page}
              totalPages={workItemsState.totalPages}
              expandedItems={workItemsState.expandedItems}
              childrenData={workItemsState.childrenData}
              loadingChildren={workItemsState.loadingChildren}
              aggregating={workItemsState.aggregating}
              aggregateResult={workItemsState.aggregateResult}
              onAggregate={workItemsState.handleAggregate}
              onClearAggregateResult={workItemsState.clearAggregateResult}
              onToggleExpand={workItemsState.toggleExpand}
              onEdit={crud.openEditModal}
              onDuplicate={crud.duplicateItem}
              onDelete={crud.confirmDelete}
              onJiraMap={crud.openJiraModal}
              onCreateNew={() => crud.setShowCreateModal(true)}
              setPage={workItemsState.setPage}
            />
          )}

          {workItemsState.viewMode === 'project' && (
            <ProjectView
              projectGroups={workItemsState.projectGroups}
              items={workItemsState.items}
              onItemClick={crud.openEditModal}
              onProjectDetail={setDetailProjectName}
            />
          )}

          {workItemsState.viewMode === 'task' && (
            <TaskView
              taskGroups={workItemsState.taskGroups}
              onItemClick={crud.openEditModal}
            />
          )}

          {workItemsState.viewMode === 'timeline' && (
            <TimelineView
              sessions={workItemsState.timelineSessions}
              date={workItemsState.timelineDate}
              onDateChange={workItemsState.setTimelineDate}
              sources={workItemsState.timelineSources}
              onSourcesChange={workItemsState.setTimelineSources}
            />
          )}
        </section>

        {/* Modals */}
        <CreateModal
          open={crud.showCreateModal}
          onOpenChange={crud.setShowCreateModal}
          formData={crud.formData}
          setFormData={crud.setFormData}
          onSubmit={async (e) => {
            await crud.handleCreate(e)
            refreshRecent()
          }}
          onCancel={crud.closeCreateModal}
          recentItems={recentItems}
          onQuickPick={crud.handleQuickPick}
        />

        <EditModal
          open={crud.showEditModal}
          onOpenChange={(open) => {
            if (!open) crud.closeEditModal()
          }}
          formData={crud.formData}
          setFormData={crud.setFormData}
          onSubmit={crud.handleUpdate}
          onCancel={crud.closeEditModal}
        />

        <JiraModal
          open={crud.showJiraModal}
          onOpenChange={(open) => {
            if (!open) crud.closeJiraModal()
          }}
          selectedItem={crud.selectedItem}
          jiraKey={crud.jiraKey}
          setJiraKey={crud.setJiraKey}
          jiraTitle={crud.jiraTitle}
          setJiraTitle={crud.setJiraTitle}
          onSubmit={crud.handleMapJira}
          onCancel={crud.closeJiraModal}
        />

        <DeleteModal
          open={crud.showDeleteConfirm}
          onOpenChange={(open) => {
            if (!open) crud.closeDeleteConfirm()
          }}
          itemToDelete={crud.itemToDelete}
          onConfirm={crud.handleDelete}
          onCancel={crud.closeDeleteConfirm}
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

        {/* Project Detail Side Panel */}
        <ProjectDetailPanel
          projectName={detailProjectName}
          onClose={() => setDetailProjectName(null)}
        />
      </div>
    </TooltipProvider>
  )
}
