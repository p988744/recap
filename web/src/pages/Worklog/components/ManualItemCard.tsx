import { Pencil, Trash2, Upload, RefreshCw, Check } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { ManualWorkItem } from '@/types/worklog'
import type { WorklogSyncRecord } from '@/types'

interface ManualItemCardProps {
  item: ManualWorkItem
  onEdit: () => void
  onDelete: () => void
  syncRecord?: WorklogSyncRecord
  onSyncToTempo?: () => void
}

export function ManualItemCard({ item, onEdit, onDelete, syncRecord, onSyncToTempo }: ManualItemCardProps) {
  const isSynced = !!syncRecord

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
        {/* Sync status row */}
        {isSynced && (
          <div className="flex items-center gap-1.5 mt-1.5 text-xs text-green-700">
            <Check className="w-3 h-3" strokeWidth={2} />
            <span>
              Synced to {syncRecord.jira_issue_key} · {syncRecord.hours}h · {syncRecord.synced_at.slice(5, 16).replace('T', ' ')}
            </span>
          </div>
        )}
      </div>

      {/* Actions */}
      <div className="flex items-center gap-1 opacity-0 group-hover/item:opacity-100 transition-opacity shrink-0">
        {onSyncToTempo && (
          <Button variant="ghost" size="icon" className="h-7 w-7" onClick={onSyncToTempo} title={isSynced ? 'Re-sync to Tempo' : 'Sync to Tempo'}>
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
