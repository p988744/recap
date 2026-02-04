import { useMemo } from 'react'
import { Plus } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { WorklogDay, WorklogDayProject, HourlyBreakdownItem, ManualWorkItem } from '@/types/worklog'
import type { WorklogSyncRecord, TempoSyncTarget } from '@/types'
import { ProjectCard } from '@/pages/Worklog/components/ProjectCard'

interface DayDetailsProps {
  day: WorklogDay
  expandedProject: { date: string; projectPath: string } | null
  hourlyData: HourlyBreakdownItem[]
  hourlyLoading: boolean
  onToggleHourly: (date: string, projectPath: string) => void
  onAddManualItem: (date: string) => void
  onEditManualItem: (id: string) => void
  onDeleteManualItem: (id: string) => void
  getSyncRecord?: (projectPath: string, date: string) => WorklogSyncRecord | undefined
  onSyncProject?: (target: TempoSyncTarget) => void
  getMappedIssueKey?: (path: string) => string
  onIssueKeyChange?: (path: string, key: string) => void
}

interface ManualProjectGroup {
  projectPath: string
  projectName: string
  items: ManualWorkItem[]
  totalHours: number
}

export function DayDetails({
  day,
  expandedProject,
  hourlyData,
  hourlyLoading,
  onToggleHourly,
  onAddManualItem,
  onEditManualItem,
  onDeleteManualItem,
  getSyncRecord,
  onSyncProject,
  getMappedIssueKey,
  onIssueKeyChange,
}: DayDetailsProps) {
  // Group manual items by project
  const manualProjects = useMemo(() => {
    const groups = new Map<string, ManualProjectGroup>()

    for (const item of day.manual_items) {
      // Use project_path as key, fallback to "未分類" if no project
      const projectPath = item.project_path ?? 'manual:uncategorized'
      const projectName = item.project_name ?? '未分類'

      if (!groups.has(projectPath)) {
        groups.set(projectPath, {
          projectPath,
          projectName,
          items: [],
          totalHours: 0,
        })
      }

      const group = groups.get(projectPath)!
      group.items.push(item)
      group.totalHours += item.hours
    }

    return Array.from(groups.values())
  }, [day.manual_items])

  // Create pseudo WorklogDayProject for manual projects (for ProjectCard compatibility)
  const createManualProject = (group: ManualProjectGroup): WorklogDayProject => ({
    project_path: group.projectPath,
    project_name: group.projectName,
    daily_summary: undefined,
    total_commits: 0,
    total_files: 0,
    total_hours: group.totalHours,
    has_hourly_data: true, // Enable expand for manual projects
  })

  return (
    <div className="border-t border-border pt-4 space-y-3">
      {/* Automatic project cards */}
      {day.projects.map((project) => {
        const isExpanded =
          expandedProject?.date === day.date &&
          expandedProject?.projectPath === project.project_path
        return (
          <ProjectCard
            key={project.project_path}
            project={project}
            date={day.date}
            isExpanded={isExpanded}
            hourlyData={isExpanded ? hourlyData : []}
            hourlyLoading={isExpanded ? hourlyLoading : false}
            onToggleHourly={() => onToggleHourly(day.date, project.project_path)}
            syncRecord={getSyncRecord?.(project.project_path, day.date)}
            onSyncToTempo={
              onSyncProject
                ? () =>
                    onSyncProject({
                      projectPath: project.project_path,
                      projectName: project.project_name,
                      date: day.date,
                      weekday: day.weekday,
                      hours: project.total_hours,
                      description: project.daily_summary ?? '',
                    })
                : undefined
            }
            mappedIssueKey={getMappedIssueKey?.(project.project_path)}
            onIssueKeyChange={onIssueKeyChange ? (key) => onIssueKeyChange(project.project_path, key) : undefined}
          />
        )
      })}

      {/* Manual project cards (grouped by project) */}
      {manualProjects.map((group) => {
        const pseudoProject = createManualProject(group)
        const isExpanded =
          expandedProject?.date === day.date &&
          expandedProject?.projectPath === group.projectPath
        return (
          <ProjectCard
            key={group.projectPath}
            project={pseudoProject}
            date={day.date}
            isExpanded={isExpanded}
            hourlyData={[]}
            hourlyLoading={false}
            onToggleHourly={() => onToggleHourly(day.date, group.projectPath)}
            // Manual project specific props
            manualItems={group.items}
            onEditManualItem={onEditManualItem}
            onDeleteManualItem={onDeleteManualItem}
            onSyncManualItem={onSyncProject ? (target) => onSyncProject({ ...target, weekday: day.weekday }) : undefined}
            getManualItemSyncRecord={getSyncRecord}
            getManualItemIssueKey={getMappedIssueKey}
            onManualItemIssueKeyChange={onIssueKeyChange}
          />
        )
      })}

      {/* Add button */}
      <Button
        variant="ghost"
        size="sm"
        className="w-full h-8 text-xs text-muted-foreground hover:text-foreground border border-dashed border-border"
        onClick={() => onAddManualItem(day.date)}
      >
        <Plus className="w-3.5 h-3.5 mr-1.5" strokeWidth={1.5} />
        新增項目
      </Button>
    </div>
  )
}
