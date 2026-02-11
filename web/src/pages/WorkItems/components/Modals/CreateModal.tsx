import { Clock } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import type { WorkItemFormData } from '../../hooks/useWorkItems'
import type { QuickPickItem } from '../../hooks/useRecentManualItems'
import { ProjectSelector } from '../ProjectSelector'

interface CreateModalProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  formData: WorkItemFormData
  setFormData: (data: WorkItemFormData) => void
  onSubmit: (e: React.FormEvent) => void
  onCancel: () => void
  recentItems?: QuickPickItem[]
  onQuickPick?: (item: QuickPickItem) => void
}

export function CreateModal({
  open,
  onOpenChange,
  formData,
  setFormData,
  onSubmit,
  onCancel,
  recentItems,
  onQuickPick,
}: CreateModalProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle className="font-display text-xl">新增工作項目</DialogTitle>
        </DialogHeader>
        {recentItems && recentItems.length > 0 && (
          <div className="space-y-2 pb-3 border-b">
            <p className="text-xs text-muted-foreground">
              <Clock className="w-3 h-3 inline mr-1" />最近使用
            </p>
            <div className="flex flex-wrap gap-1.5">
              {recentItems.map((item, i) => (
                <button
                  key={i}
                  type="button"
                  className="inline-flex items-center gap-1 px-2.5 py-1 text-xs
                             rounded-full bg-muted/50 hover:bg-muted border
                             border-border hover:border-foreground/20 transition-colors"
                  onClick={() => onQuickPick?.(item)}
                >
                  <span className="truncate max-w-[160px]">{item.title}</span>
                  {item.hours > 0 && (
                    <span className="text-muted-foreground tabular-nums">{item.hours}h</span>
                  )}
                </button>
              ))}
            </div>
          </div>
        )}
        <form onSubmit={onSubmit} className="space-y-4">
          <div className="space-y-2">
            <Label>標題</Label>
            <Input
              value={formData.title}
              onChange={(e) => setFormData({ ...formData, title: e.target.value })}
              required
            />
          </div>
          <div className="space-y-2">
            <Label>描述</Label>
            <Textarea
              value={formData.description}
              onChange={(e) => setFormData({ ...formData, description: e.target.value })}
              rows={3}
            />
          </div>
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label>日期</Label>
              <Input
                type="date"
                value={formData.date}
                onChange={(e) => setFormData({ ...formData, date: e.target.value })}
                required
              />
            </div>
            <div className="space-y-2">
              <Label>工時 (小時)</Label>
              <Input
                type="number"
                step="0.5"
                min="0"
                value={formData.hours}
                onChange={(e) => setFormData({ ...formData, hours: parseFloat(e.target.value) || 0 })}
              />
            </div>
          </div>
          <div className="space-y-2">
            <Label>所屬專案</Label>
            <ProjectSelector
              value={formData.project_name}
              onChange={(value) => setFormData({ ...formData, project_name: value })}
              placeholder="選擇或新增專案..."
            />
          </div>
          <div className="space-y-2">
            <Label>Jira Issue Key</Label>
            <Input
              placeholder="ABC-123"
              value={formData.jira_issue_key}
              onChange={(e) => setFormData({ ...formData, jira_issue_key: e.target.value })}
            />
          </div>
          <DialogFooter>
            <Button type="button" variant="ghost" onClick={onCancel}>
              取消
            </Button>
            <Button type="submit">
              建立
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
