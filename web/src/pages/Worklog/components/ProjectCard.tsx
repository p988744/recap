import { useState } from 'react'
import { ChevronDown, ChevronRight, GitCommit, FileCode, Upload, RefreshCw, Link } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { WorklogDayProject, HourlyBreakdownItem } from '@/types/worklog'
import type { WorklogSyncRecord } from '@/types'
import { MarkdownSummary } from '@/components/MarkdownSummary'
import { HourlyBreakdown } from './HourlyBreakdown'
import { JiraBadge } from './JiraBadge'
import { IssueKeyCombobox } from './IssueKeyCombobox'

interface ProjectCardProps {
  project: WorklogDayProject
  date: string
  isExpanded: boolean
  hourlyData: HourlyBreakdownItem[]
  hourlyLoading: boolean
  onToggleHourly: () => void
  syncRecord?: WorklogSyncRecord
  onSyncToTempo?: () => void
  mappedIssueKey?: string
  onIssueKeyChange?: (issueKey: string) => void
}

export function ProjectCard({
  project,
  isExpanded,
  hourlyData,
  hourlyLoading,
  onToggleHourly,
  syncRecord,
  onSyncToTempo,
  mappedIssueKey,
  onIssueKeyChange,
}: ProjectCardProps) {
  const hasHourly = project.has_hourly_data
  const isSynced = !!syncRecord
  const [editing, setEditing] = useState(false)
  const [editValue, setEditValue] = useState('')
  const displayKey = syncRecord?.jira_issue_key ?? mappedIssueKey ?? ''

  return (
    <div className="border border-border rounded-lg bg-white/60">
      {/* Card header */}
      <div className="flex items-start">
        <div
          className={`flex-1 px-4 py-3 flex items-start gap-3 text-left transition-colors rounded-l-lg ${hasHourly ? 'hover:bg-muted/30 cursor-pointer' : ''}`}
          onClick={hasHourly ? onToggleHourly : undefined}
          role={hasHourly ? 'button' : undefined}
          tabIndex={hasHourly ? 0 : undefined}
          onKeyDown={hasHourly ? (e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onToggleHourly() } } : undefined}
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

            {/* Jira issue key: badge + inline edit */}
            {onIssueKeyChange && (
              <div className="mt-2">
                {editing ? (
                  <div className="max-w-[220px]" onClick={(e) => e.stopPropagation()}>
                    <IssueKeyCombobox
                      value={editValue}
                      onChange={setEditValue}
                      onBlur={() => {
                        const trimmed = editValue.trim()
                        if (trimmed && trimmed !== displayKey) {
                          onIssueKeyChange(trimmed)
                        }
                        setEditing(false)
                      }}
                      compact
                      placeholder="e.g. PROJ-123"
                      className="h-7"
                    />
                  </div>
                ) : displayKey ? (
                  <button
                    type="button"
                    onClick={(e) => { e.stopPropagation(); setEditValue(displayKey); setEditing(true) }}
                    className="cursor-pointer hover:opacity-80 transition-opacity"
                  >
                    <JiraBadge issueKey={displayKey} />
                  </button>
                ) : (
                  <button
                    type="button"
                    onClick={(e) => { e.stopPropagation(); setEditValue(''); setEditing(true) }}
                    className="flex items-center gap-1 text-[11px] text-muted-foreground hover:text-foreground transition-colors"
                  >
                    <Link className="w-3 h-3" strokeWidth={1.5} />
                    Link Jira
                  </button>
                )}
              </div>
            )}
          </div>
        </div>

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
                <><RefreshCw className="w-3 h-3 mr-1" strokeWidth={1.5} />Re-export</>
              ) : (
                <><Upload className="w-3 h-3 mr-1" strokeWidth={1.5} />Export</>
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
