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

interface CreateModalProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  formData: WorkItemFormData
  setFormData: (data: WorkItemFormData) => void
  onSubmit: (e: React.FormEvent) => void
  onCancel: () => void
}

export function CreateModal({
  open,
  onOpenChange,
  formData,
  setFormData,
  onSubmit,
  onCancel,
}: CreateModalProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle className="font-display text-xl">新增工作項目</DialogTitle>
        </DialogHeader>
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
