import { Plus, Upload } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { WorklogDay } from '@/types/worklog'
import type { HourlyBreakdownItem } from '@/types/worklog'
import type { WorklogSyncRecord, TempoSyncTarget } from '@/types'
import { ProjectCard } from './ProjectCard'
import { ManualItemCard } from './ManualItemCard'

interface DaySectionProps {
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
  onSyncDay?: (date: string, weekday: string) => void
  getMappedIssueKey?: (path: string) => string
  onIssueKeyChange?: (path: string, key: string) => void
}

export function DaySection({
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
  onSyncDay,
  getMappedIssueKey,
  onIssueKeyChange,
}: DaySectionProps) {
  const isEmpty = day.projects.length === 0 && day.manual_items.length === 0

  return (
    <section className="group">
      {/* Day header */}
      <div className="flex items-baseline justify-between mb-4">
        <div className="flex items-baseline gap-3">
          <h2 className="font-display text-lg text-foreground tracking-tight">
            {day.date.slice(5).replace('-', '/')}
          </h2>
          <span className="text-xs text-muted-foreground">{day.weekday}</span>
        </div>
        <div className="flex items-center gap-1">
          {onSyncDay && !isEmpty && (
            <Button
              variant="ghost"
              size="sm"
              className="h-7 text-xs text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity"
              onClick={() => onSyncDay(day.date, day.weekday)}
            >
              <Upload className="w-3 h-3 mr-1" strokeWidth={1.5} />
              Export Day
            </Button>
          )}
          <Button
            variant="ghost"
            size="sm"
            className="h-7 text-xs text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity"
            onClick={() => onAddManualItem(day.date)}
          >
            <Plus className="w-3 h-3 mr-1" strokeWidth={1.5} />
            Add
          </Button>
        </div>
      </div>

      {/* Content */}
      {isEmpty ? (
        <div className="py-6 text-center">
          <p className="text-sm text-muted-foreground">No records</p>
        </div>
      ) : (
        <div className="space-y-3">
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
        </div>
      )}

      {/* Divider */}
      <div className="mt-8 h-px bg-border" />
    </section>
  )
}
