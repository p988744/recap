import { useState } from 'react'
import { Pencil, Trash2, Upload, RefreshCw, Link } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { ManualWorkItem } from '@/types/worklog'
import type { WorklogSyncRecord } from '@/types'
import { JiraBadge } from './JiraBadge'
import { IssueKeyCombobox } from './IssueKeyCombobox'

interface ManualItemCardProps {
  item: ManualWorkItem
  onEdit: () => void
  onDelete: () => void
  syncRecord?: WorklogSyncRecord
  onSyncToTempo?: () => void
  mappedIssueKey?: string
  onIssueKeyChange?: (issueKey: string) => void
}

export function ManualItemCard({ item, onEdit, onDelete, syncRecord, onSyncToTempo, mappedIssueKey, onIssueKeyChange }: ManualItemCardProps) {
  const isSynced = !!syncRecord
  const [editing, setEditing] = useState(false)
  const [editValue, setEditValue] = useState('')
  const displayKey = syncRecord?.jira_issue_key ?? mappedIssueKey ?? ''

  return (
    <div className="group/item border border-border rounded-lg bg-white/60">
      <div className="flex items-start">
        {/* Content area */}
        <div className="flex-1 px-4 py-3 flex items-start gap-3">
          {/* Indicator — matches ProjectCard chevron area sizing */}
          <div className="mt-0.5 text-muted-foreground">
            <div className="w-4 h-4 flex items-center justify-center">
              <div className="w-2 h-2 rounded-full bg-muted-foreground/30" />
            </div>
          </div>

          {/* Content */}
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-1">
              <span className="text-sm font-medium text-foreground">{item.title}</span>
              {item.hours > 0 && (
                <span className="text-xs text-muted-foreground">{item.hours}h</span>
              )}
            </div>

            {item.description && (
              <p className="text-sm text-muted-foreground line-clamp-1">{item.description}</p>
            )}

            {/* Jira issue key: badge + inline edit */}
            {onIssueKeyChange && (
              <div className="mt-2">
                {editing ? (
                  <div className="max-w-[220px]">
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
                    onClick={() => { setEditValue(displayKey); setEditing(true) }}
                    className="cursor-pointer hover:opacity-80 transition-opacity"
                  >
                    <JiraBadge issueKey={displayKey} />
                  </button>
                ) : (
                  <button
                    type="button"
                    onClick={() => { setEditValue(''); setEditing(true) }}
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

        {/* Right side actions — Export always visible, Edit/Delete on hover */}
        <div className="px-2 py-3 flex items-center gap-1 shrink-0">
          <div className="flex items-center gap-1 opacity-0 group-hover/item:opacity-100 transition-opacity">
            <Button variant="ghost" size="icon" className="h-7 w-7" onClick={onEdit} title="Edit">
              <Pencil className="w-3 h-3" strokeWidth={1.5} />
            </Button>
            <Button variant="ghost" size="icon" className="h-7 w-7 text-destructive" onClick={onDelete} title="Delete">
              <Trash2 className="w-3 h-3" strokeWidth={1.5} />
            </Button>
          </div>
          {onSyncToTempo && (
            <Button
              variant="ghost"
              size="sm"
              className="h-7 text-xs text-muted-foreground hover:text-foreground"
              onClick={onSyncToTempo}
            >
              {isSynced ? (
                <><RefreshCw className="w-3 h-3 mr-1" strokeWidth={1.5} />Re-export</>
              ) : (
                <><Upload className="w-3 h-3 mr-1" strokeWidth={1.5} />Export</>
              )}
            </Button>
          )}
        </div>
      </div>
    </div>
  )
}
