import { useCallback, useEffect, useState } from 'react'
import { Plus } from 'lucide-react'
import { projects as projectsService, worklog } from '@/services'
import type { ProjectInfo } from '@/types'
import type { CompactionResult } from '@/services/worklog'
import type { BackgroundSyncStatus, SyncProgress } from '@/services/background-sync'
import { ProjectList } from './ProjectList'
import { ProjectSourcePanel } from './ProjectSourcePanel'
import { ClaudePathSetting } from './ClaudePathSetting'
import { AntigravityPathSetting } from './AntigravityPathSetting'
import { AddProjectDialog } from './AddProjectDialog'
import { DataSyncStatus, DataCompactionStatus } from './DataSyncStatus'

type PhaseState = 'idle' | 'syncing' | 'done'
type CompactionPhase = 'idle' | 'hourly' | 'timeline' | 'done'

interface ProjectsSectionProps {
  syncStatus?: BackgroundSyncStatus | null
  syncEnabled?: boolean
  autoGenerateSummaries?: boolean
  dataSyncState?: PhaseState
  summaryState?: PhaseState
  syncProgress?: SyncProgress | null
  onTriggerSync?: () => void
  onRefreshStatus?: () => Promise<void>
}

export function ProjectsSection({
  syncStatus = null,
  syncEnabled = false,
  autoGenerateSummaries = true,
  dataSyncState = 'idle',
  summaryState = 'idle',
  syncProgress = null,
  onTriggerSync,
  onRefreshStatus,
}: ProjectsSectionProps) {
  const [projectList, setProjectList] = useState<ProjectInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [selectedProject, setSelectedProject] = useState<string | null>(null)
  const [showAddDialog, setShowAddDialog] = useState(false)
  const [compactionPhase, setCompactionPhase] = useState<CompactionPhase>('idle')
  const [compactionResult, setCompactionResult] = useState<CompactionResult | null>(null)

  const fetchProjects = useCallback(async () => {
    try {
      const data = await projectsService.listProjects()
      setProjectList(data)
    } catch (err) {
      console.error('Failed to fetch projects:', err)
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    fetchProjects()
  }, [fetchProjects])

  const handleToggleVisibility = useCallback(async (projectName: string, hidden: boolean) => {
    try {
      await projectsService.setProjectVisibility(projectName, hidden)
      setProjectList((prev) =>
        prev.map((p) =>
          p.project_name === projectName ? { ...p, hidden } : p
        )
      )
    } catch (err) {
      console.error('Failed to toggle visibility:', err)
    }
  }, [])

  const handleAddProject = useCallback(async () => {
    setShowAddDialog(false)
    await fetchProjects()
  }, [fetchProjects])

  const handleRemoveProject = useCallback(async (projectName: string) => {
    try {
      await projectsService.removeManualProject(projectName)
      await fetchProjects()
    } catch (err) {
      console.error('Failed to remove project:', err)
    }
  }, [fetchProjects])

  const handleTriggerCompaction = useCallback(async () => {
    if (compactionPhase !== 'idle') return

    try {
      setCompactionPhase('hourly')
      setCompactionResult(null)
      const result = await worklog.triggerCompaction()
      setCompactionResult(result)
      setCompactionPhase('timeline')
      // Brief delay to show timeline phase
      await new Promise(resolve => setTimeout(resolve, 500))
      setCompactionPhase('done')
      console.log('Compaction complete:', result)
      // Refresh backend status to update last_compaction_at
      if (onRefreshStatus) {
        await onRefreshStatus()
      }
      // Reset phase after showing done state, but keep result for display
      setTimeout(() => setCompactionPhase('idle'), 2000)
    } catch (err) {
      console.error('Failed to trigger compaction:', err)
      setCompactionPhase('idle')
      setCompactionResult(null)
    }
  }, [compactionPhase, onRefreshStatus])

  if (loading) {
    return (
      <section className="animate-fade-up opacity-0 delay-1">
        <h2 className="font-display text-2xl text-foreground mb-6">專案</h2>
        <div className="flex items-center justify-center h-48">
          <div className="w-5 h-5 border border-border border-t-charcoal/60 rounded-full animate-spin" />
        </div>
      </section>
    )
  }

  return (
    <section className="animate-fade-up opacity-0 delay-1">
      <h2 className="font-display text-2xl text-foreground mb-1">專案</h2>
      <p className="text-xs text-muted-foreground mb-6">
        管理專案設定。隱藏的專案不會出現在儀表板、報告和統計中。
      </p>

      {/* Data sync status */}
      {onTriggerSync && (
        <div className="space-y-3">
          <DataSyncStatus
            status={syncStatus}
            enabled={syncEnabled}
            dataSyncState={dataSyncState}
            summaryState={summaryState}
            syncProgress={syncProgress}
            onTriggerSync={onTriggerSync}
          />
          <DataCompactionStatus
            status={syncStatus}
            enabled={syncEnabled}
            autoGenerateSummaries={autoGenerateSummaries}
            compactionPhase={compactionPhase}
            compactionResult={compactionResult}
            onTriggerCompaction={handleTriggerCompaction}
          />
        </div>
      )}

      {/* Data source path settings */}
      <div className="mt-4 space-y-3">
        <ClaudePathSetting />
        <AntigravityPathSetting />
      </div>

      {/* Project list */}
      <div className="mt-6">
        <div className="flex items-center justify-between mb-3">
          <h3 className="text-sm font-medium text-foreground">專案列表</h3>
          <button
            onClick={() => setShowAddDialog(true)}
            className="flex items-center gap-1.5 px-2.5 py-1.5 text-xs text-muted-foreground hover:text-foreground rounded-md hover:bg-foreground/5 transition-colors"
          >
            <Plus className="w-3.5 h-3.5" />
            新增專案
          </button>
        </div>
        <ProjectList
          projects={projectList}
          onSelect={setSelectedProject}
          onToggleVisibility={handleToggleVisibility}
          onRemove={handleRemoveProject}
        />
      </div>

      <ProjectSourcePanel
        projectName={selectedProject}
        onClose={() => setSelectedProject(null)}
      />

      <AddProjectDialog
        open={showAddDialog}
        onClose={() => setShowAddDialog(false)}
        onAdded={handleAddProject}
      />
    </section>
  )
}
