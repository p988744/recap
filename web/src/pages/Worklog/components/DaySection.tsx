import { Plus } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { WorklogDay } from '@/types/worklog'
import type { HourlyBreakdownItem } from '@/types/worklog'
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
        <Button
          variant="ghost"
          size="sm"
          className="h-7 text-xs text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity"
          onClick={() => onAddManualItem(day.date)}
        >
          <Plus className="w-3 h-3 mr-1" strokeWidth={1.5} />
          新增
        </Button>
      </div>

      {/* Content */}
      {isEmpty ? (
        <div className="py-6 text-center">
          <p className="text-sm text-muted-foreground">尚無工作紀錄</p>
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
            />
          ))}
        </div>
      )}

      {/* Divider */}
      <div className="mt-8 h-px bg-border" />
    </section>
  )
}
