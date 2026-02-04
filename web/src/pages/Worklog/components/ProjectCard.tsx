import { useState } from 'react'
import { ChevronDown, ChevronRight, GitCommit, FileCode, Upload, RefreshCw, Link, FileText, Pencil, Trash2 } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { WorklogDayProject, HourlyBreakdownItem, ManualWorkItem } from '@/types/worklog'
import type { WorklogSyncRecord, TempoSyncTarget } from '@/types'
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
  // Manual project support
  manualItems?: ManualWorkItem[]
  onEditManualItem?: (id: string) => void
  onDeleteManualItem?: (id: string) => void
  onSyncManualItem?: (target: TempoSyncTarget) => void
  getManualItemSyncRecord?: (itemId: string, date: string) => WorklogSyncRecord | undefined
  getManualItemIssueKey?: (itemId: string) => string
  onManualItemIssueKeyChange?: (itemId: string, key: string) => void
}

export function ProjectCard({
  project,
  date,
  isExpanded,
  hourlyData,
  hourlyLoading,
  onToggleHourly,
  syncRecord,
  onSyncToTempo,
  mappedIssueKey,
  onIssueKeyChange,
  manualItems,
  onEditManualItem,
  onDeleteManualItem,
  onSyncManualItem,
  getManualItemSyncRecord,
  getManualItemIssueKey,
  onManualItemIssueKeyChange,
}: ProjectCardProps) {
  const isManualProject = !!manualItems && manualItems.length > 0
  const hasExpandable = project.has_hourly_data || isManualProject
  const isSynced = !!syncRecord
  const [editing, setEditing] = useState(false)
  const [editValue, setEditValue] = useState('')
  const displayKey = syncRecord?.jira_issue_key ?? mappedIssueKey ?? ''

  return (
    <div className="border border-border rounded-lg bg-white/60">
      {/* Card header */}
      <div className="flex items-start">
        <div
          className={`flex-1 px-4 py-3 flex items-start gap-3 text-left transition-colors rounded-l-lg ${hasExpandable ? 'hover:bg-muted/30 cursor-pointer' : ''}`}
          onClick={hasExpandable ? onToggleHourly : undefined}
          role={hasExpandable ? 'button' : undefined}
          tabIndex={hasExpandable ? 0 : undefined}
          onKeyDown={hasExpandable ? (e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onToggleHourly() } } : undefined}
        >
          {/* Expand icon */}
          <div className="mt-0.5 text-muted-foreground">
            {hasExpandable ? (
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
              {isManualProject && (
                <span className="text-xs text-muted-foreground">
                  {project.total_hours.toFixed(1)}h
                </span>
              )}
            </div>

            {/* Summary for automatic projects */}
            {!isManualProject && project.daily_summary && (
              <MarkdownSummary content={project.daily_summary} />
            )}

            {/* Items preview for manual projects (when collapsed) */}
            {isManualProject && !isExpanded && manualItems && (
              <div className="text-sm text-muted-foreground">
                {manualItems.length === 1 ? (
                  <span>{manualItems[0].title}</span>
                ) : (
                  <span>{manualItems.length} 個項目</span>
                )}
              </div>
            )}

            {/* Stats row for automatic projects */}
            {!isManualProject && (
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
            )}

            {/* Jira issue key for automatic projects */}
            {!isManualProject && onIssueKeyChange && (
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

        {/* Sync button for automatic projects */}
        {!isManualProject && onSyncToTempo && (
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

      {/* Expanded content */}
      {isExpanded && (
        <div className="border-t border-border">
          {isManualProject && manualItems ? (
            // Manual items list
            <div>
              {manualItems.map((item) => (
                <ManualItemRow
                  key={item.id}
                  item={item}
                  date={date}
                  onEdit={onEditManualItem ? () => onEditManualItem(item.id) : undefined}
                  onDelete={onDeleteManualItem ? () => onDeleteManualItem(item.id) : undefined}
                  syncRecord={getManualItemSyncRecord?.(`manual:${item.id}`, item.date)}
                  onSyncToTempo={onSyncManualItem ? () => onSyncManualItem({
                    projectPath: `manual:${item.id}`,
                    projectName: item.title,
                    date: item.date,
                    weekday: '', // Will be filled by caller
                    hours: item.hours,
                    description: item.description ?? item.title,
                  }) : undefined}
                  mappedIssueKey={getManualItemIssueKey?.(`manual:${item.id}`)}
                  onIssueKeyChange={onManualItemIssueKeyChange ? (key) => onManualItemIssueKeyChange(`manual:${item.id}`, key) : undefined}
                />
              ))}
            </div>
          ) : (
            // Hourly breakdown for automatic projects
            <HourlyBreakdown items={hourlyData} loading={hourlyLoading} />
          )}
        </div>
      )}
    </div>
  )
}

interface ManualItemRowProps {
  item: ManualWorkItem
  date: string
  onEdit?: () => void
  onDelete?: () => void
  syncRecord?: WorklogSyncRecord
  onSyncToTempo?: () => void
  mappedIssueKey?: string
  onIssueKeyChange?: (issueKey: string) => void
}

function ManualItemRow({
  item,
  onEdit,
  onDelete,
  syncRecord,
  onSyncToTempo,
  mappedIssueKey,
  onIssueKeyChange,
}: ManualItemRowProps) {
  const isSynced = !!syncRecord
  const [editing, setEditing] = useState(false)
  const [editValue, setEditValue] = useState('')
  const displayKey = syncRecord?.jira_issue_key ?? mappedIssueKey ?? item.jira_issue_key ?? ''

  return (
    <div className="group/item flex items-start px-4 py-3 hover:bg-muted/20 transition-colors">
      {/* Indent + icon */}
      <div className="w-4 mr-3" /> {/* Spacer for alignment with chevron */}
      <div className="mt-0.5 mr-3 text-muted-foreground">
        <FileText className="w-4 h-4" strokeWidth={1.5} />
      </div>

      {/* Content */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-0.5">
          <span className="text-sm font-medium text-foreground">{item.title}</span>
          <span className="text-xs text-muted-foreground">{item.hours}h</span>
        </div>

        {item.description && (
          <p className="text-sm text-muted-foreground line-clamp-2">{item.description}</p>
        )}

        {/* Jira issue key */}
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
                onClick={(e) => {
                  e.stopPropagation()
                  setEditValue(displayKey)
                  setEditing(true)
                }}
                className="cursor-pointer hover:opacity-80 transition-opacity"
              >
                <JiraBadge issueKey={displayKey} />
              </button>
            ) : (
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation()
                  setEditValue('')
                  setEditing(true)
                }}
                className="flex items-center gap-1 text-[11px] text-muted-foreground hover:text-foreground transition-colors"
              >
                <Link className="w-3 h-3" strokeWidth={1.5} />
                Link Jira
              </button>
            )}
          </div>
        )}
      </div>

      {/* Actions */}
      <div className="flex items-center gap-1 shrink-0">
        {(onEdit || onDelete) && (
          <div className="flex items-center gap-1 opacity-0 group-hover/item:opacity-100 transition-opacity">
            {onEdit && (
              <Button
                variant="ghost"
                size="icon"
                className="h-7 w-7"
                onClick={(e) => {
                  e.stopPropagation()
                  onEdit()
                }}
                title="Edit"
              >
                <Pencil className="w-3 h-3" strokeWidth={1.5} />
              </Button>
            )}
            {onDelete && (
              <Button
                variant="ghost"
                size="icon"
                className="h-7 w-7 text-destructive"
                onClick={(e) => {
                  e.stopPropagation()
                  onDelete()
                }}
                title="Delete"
              >
                <Trash2 className="w-3 h-3" strokeWidth={1.5} />
              </Button>
            )}
          </div>
        )}
        {onSyncToTempo && (
          <Button
            variant="ghost"
            size="sm"
            className="h-7 text-xs text-muted-foreground hover:text-foreground"
            onClick={(e) => {
              e.stopPropagation()
              onSyncToTempo()
            }}
          >
            {isSynced ? (
              <>
                <RefreshCw className="w-3 h-3 mr-1" strokeWidth={1.5} />
                Re-export
              </>
            ) : (
              <>
                <Upload className="w-3 h-3 mr-1" strokeWidth={1.5} />
                Export
              </>
            )}
          </Button>
        )}
      </div>
    </div>
  )
}
