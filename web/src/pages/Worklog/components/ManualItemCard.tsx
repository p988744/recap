import { Pencil, Trash2 } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { ManualWorkItem } from '@/types/worklog'

interface ManualItemCardProps {
  item: ManualWorkItem
  onEdit: () => void
  onDelete: () => void
}

export function ManualItemCard({ item, onEdit, onDelete }: ManualItemCardProps) {
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
      </div>

      {/* Actions */}
      <div className="flex items-center gap-1 opacity-0 group-hover/item:opacity-100 transition-opacity shrink-0">
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
