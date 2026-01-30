import { Plus } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { WorklogDay, HourlyBreakdownItem } from '@/types/worklog'
import type { WorklogSyncRecord, TempoSyncTarget } from '@/types'
import { ProjectCard } from '@/pages/Worklog/components/ProjectCard'
import { ManualItemCard } from '@/pages/Worklog/components/ManualItemCard'

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
  return (
    <div className="border-t border-border pt-4 space-y-3">
      {/* Project cards */}
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

      {/* Manual items */}
      {day.manual_items.map((item) => (
        <ManualItemCard
          key={item.id}
          item={item}
          onEdit={() => onEditManualItem(item.id)}
          onDelete={() => onDeleteManualItem(item.id)}
          syncRecord={getSyncRecord?.(`manual:${item.id}`, day.date)}
          onSyncToTempo={
            onSyncProject
              ? () =>
                  onSyncProject({
                    projectPath: `manual:${item.id}`,
                    projectName: item.title,
                    date: day.date,
                    weekday: day.weekday,
                    hours: item.hours,
                    description: item.description ?? item.title,
                  })
              : undefined
          }
          mappedIssueKey={getMappedIssueKey?.(`manual:${item.id}`)}
          onIssueKeyChange={onIssueKeyChange ? (key) => onIssueKeyChange(`manual:${item.id}`, key) : undefined}
        />
      ))}

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
