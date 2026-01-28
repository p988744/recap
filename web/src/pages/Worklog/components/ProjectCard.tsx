import { ChevronDown, ChevronRight, GitCommit, FileCode, Upload, RefreshCw, Check } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { WorklogDayProject, HourlyBreakdownItem } from '@/types/worklog'
import type { WorklogSyncRecord } from '@/types'
import { MarkdownSummary } from '@/components/MarkdownSummary'
import { HourlyBreakdown } from './HourlyBreakdown'

interface ProjectCardProps {
  project: WorklogDayProject
  date: string
  isExpanded: boolean
  hourlyData: HourlyBreakdownItem[]
  hourlyLoading: boolean
  onToggleHourly: () => void
  syncRecord?: WorklogSyncRecord
  onSyncToTempo?: () => void
}

export function ProjectCard({
  project,
  isExpanded,
  hourlyData,
  hourlyLoading,
  onToggleHourly,
  syncRecord,
  onSyncToTempo,
}: ProjectCardProps) {
  const hasHourly = project.has_hourly_data
  const isSynced = !!syncRecord

  return (
    <div className="border border-border rounded-lg bg-white/60">
      {/* Card header */}
      <div className="flex items-start">
        <button
          className="flex-1 px-4 py-3 flex items-start gap-3 text-left hover:bg-muted/30 transition-colors rounded-l-lg"
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
              <MarkdownSummary content={project.daily_summary} />
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

            {/* Sync status row */}
            {isSynced && (
              <div className="flex items-center gap-1.5 mt-2 text-xs text-green-700">
                <Check className="w-3 h-3" strokeWidth={2} />
                <span>
                  Synced to {syncRecord.jira_issue_key} · {syncRecord.hours}h · {syncRecord.synced_at.slice(5, 16).replace('T', ' ')}
                </span>
              </div>
            )}
          </div>
        </button>

        {/* Sync button */}
        {onSyncToTempo && (
          <div className="px-2 py-3">
            <Button
              variant="ghost"
              size="sm"
              className="h-7 text-xs text-muted-foreground hover:text-foreground"
              onClick={(e) => { e.stopPropagation(); onSyncToTempo() }}
            >
              {isSynced ? (
                <><RefreshCw className="w-3 h-3 mr-1" strokeWidth={1.5} />Re-sync</>
              ) : (
                <><Upload className="w-3 h-3 mr-1" strokeWidth={1.5} />Sync</>
              )}
            </Button>
          </div>
        )}
      </div>

      {/* Hourly breakdown (expanded) */}
      {isExpanded && (
        <div className="border-t border-border">
          <HourlyBreakdown items={hourlyData} loading={hourlyLoading} />
        </div>
      )}
    </div>
  )
}
