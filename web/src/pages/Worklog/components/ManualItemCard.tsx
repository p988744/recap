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
    <div className="group/item border border-border rounded-lg bg-white/60 px-4 py-3 flex items-start gap-3">
      {/* Indicator dot */}
      <div className="mt-1.5 w-2 h-2 rounded-full bg-muted-foreground/30 shrink-0" />

      {/* Content */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-foreground">{item.title}</span>
          {item.hours > 0 && (
            <span className="text-xs text-muted-foreground">{item.hours}h</span>
          )}
        </div>
        {item.description && (
          <p className="text-sm text-muted-foreground mt-0.5 line-clamp-1">{item.description}</p>
        )}
        {/* Jira issue key: badge + inline edit */}
        {onIssueKeyChange && (
          <div className="mt-1.5">
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

      {/* Actions */}
      <div className="flex items-center gap-1 opacity-0 group-hover/item:opacity-100 transition-opacity shrink-0">
        {onSyncToTempo && (
          <Button variant="ghost" size="icon" className="h-7 w-7" onClick={onSyncToTempo} title={isSynced ? 'Re-export to Tempo' : 'Export to Tempo'}>
            {isSynced ? <RefreshCw className="w-3 h-3" strokeWidth={1.5} /> : <Upload className="w-3 h-3" strokeWidth={1.5} />}
          </Button>
        )}
        <Button variant="ghost" size="icon" className="h-7 w-7" onClick={onEdit}>
          <Pencil className="w-3 h-3" strokeWidth={1.5} />
        </Button>
        <Button variant="ghost" size="icon" className="h-7 w-7 text-destructive" onClick={onDelete}>
          <Trash2 className="w-3 h-3" strokeWidth={1.5} />
        </Button>
      </div>
    </div>
  )
}
