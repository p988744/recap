import { Link2 } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import type { WorkItem } from '@/types'

interface JiraModalProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  selectedItem: WorkItem | null
  jiraKey: string
  setJiraKey: (key: string) => void
  jiraTitle: string
  setJiraTitle: (title: string) => void
  onSubmit: (e: React.FormEvent) => void
  onCancel: () => void
}

export function JiraModal({
  open,
  onOpenChange,
  selectedItem,
  jiraKey,
  setJiraKey,
  jiraTitle,
  setJiraTitle,
  onSubmit,
  onCancel,
}: JiraModalProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle className="font-display text-xl">對應 Jira Issue</DialogTitle>
          <DialogDescription className="truncate">
            {selectedItem?.title}
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={onSubmit} className="space-y-4">
          <div className="space-y-2">
            <Label>Jira Issue Key</Label>
            <Input
              placeholder="ABC-123"
              value={jiraKey}
              onChange={(e) => setJiraKey(e.target.value)}
              required
            />
          </div>
          <div className="space-y-2">
            <Label>Issue 標題 (選填)</Label>
            <Input
              placeholder="Issue title"
              value={jiraTitle}
              onChange={(e) => setJiraTitle(e.target.value)}
            />
          </div>
          {selectedItem?.jira_issue_suggested && (
            <div className="p-3 bg-muted border-l-2 border-l-accent">
              <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-1">建議對應</p>
              <button
                type="button"
                onClick={() => setJiraKey(selectedItem.jira_issue_suggested!)}
                className="text-sm text-accent hover:underline"
              >
                {selectedItem.jira_issue_suggested}
              </button>
            </div>
          )}
          <DialogFooter>
            <Button type="button" variant="ghost" onClick={onCancel}>
              取消
            </Button>
            <Button type="submit">
              <Link2 className="w-4 h-4 mr-2" strokeWidth={1.5} />
              對應
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
