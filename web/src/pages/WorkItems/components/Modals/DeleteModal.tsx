import { Trash2 } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import type { WorkItem } from '@/types'

interface DeleteModalProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  itemToDelete: WorkItem | null
  onConfirm: () => void
  onCancel: () => void
}

export function DeleteModal({
  open,
  onOpenChange,
  itemToDelete,
  onConfirm,
  onCancel,
}: DeleteModalProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle className="font-display text-xl">確認刪除</DialogTitle>
          <DialogDescription>
            確定要刪除工作項目「{itemToDelete?.title}」嗎？此操作無法復原。
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="ghost" onClick={onCancel}>
            取消
          </Button>
          <Button variant="destructive" onClick={onConfirm}>
            <Trash2 className="w-4 h-4 mr-2" strokeWidth={1.5} />
            刪除
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
