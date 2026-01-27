import { ChevronDown, ChevronRight, GitCommit, FileCode } from 'lucide-react'
import type { WorklogDayProject, HourlyBreakdownItem } from '@/types/worklog'
import { HourlyBreakdown } from './HourlyBreakdown'

interface ProjectCardProps {
  project: WorklogDayProject
  date: string
  isExpanded: boolean
  hourlyData: HourlyBreakdownItem[]
  hourlyLoading: boolean
  onToggleHourly: () => void
}

export function ProjectCard({
  project,
  isExpanded,
  hourlyData,
  hourlyLoading,
  onToggleHourly,
}: ProjectCardProps) {
  const hasHourly = project.has_hourly_data

  return (
    <div className="border border-border rounded-lg bg-white/60">
      {/* Card header */}
      <button
        className="w-full px-4 py-3 flex items-start gap-3 text-left hover:bg-muted/30 transition-colors rounded-lg"
        onClick={hasHourly ? onToggleHourly : undefined}
        disabled={!hasHourly}
      >
        {/* Expand icon */}
        <div className="mt-0.5 text-muted-foreground">
          {hasHourly ? (
            isExpanded ? (
              <ChevronDown className="w-4 h-4" strokeWidth={1.5} />
            ) : (
              <ChevronRight className="w-4 h-4" strokeWidth={1.5} />
            )
          ) : (
            <div className="w-4 h-4" />
          )}
        </div>

        {/* Content */}
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1">
            <span className="text-sm font-medium text-foreground truncate">
              {project.project_name}
            </span>
          </div>

          {/* Summary */}
          {project.daily_summary && (
            <p className="text-sm text-muted-foreground leading-relaxed line-clamp-2">
              {project.daily_summary}
            </p>
          )}

          {/* Stats row */}
          <div className="flex items-center gap-4 mt-2">
            {project.total_commits > 0 && (
              <span className="flex items-center gap-1 text-xs text-muted-foreground">
                <GitCommit className="w-3 h-3" strokeWidth={1.5} />
                {project.total_commits} commits
              </span>
            )}
            {project.total_files > 0 && (
              <span className="flex items-center gap-1 text-xs text-muted-foreground">
                <FileCode className="w-3 h-3" strokeWidth={1.5} />
                {project.total_files} files
              </span>
            )}
          </div>
        </div>
      </button>

      {/* Hourly breakdown (expanded) */}
      {isExpanded && (
        <div className="border-t border-border">
          <HourlyBreakdown items={hourlyData} loading={hourlyLoading} />
        </div>
      )}
    </div>
  )
}
