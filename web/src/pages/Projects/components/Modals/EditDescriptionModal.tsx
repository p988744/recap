import { useState, useEffect } from 'react'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { Label } from '@/components/ui/label'
import { projects as projectsService } from '@/services'
import type { ProjectDescription } from '@/types'

interface EditDescriptionModalProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  projectName: string
  initialData: ProjectDescription | null
  onSaved: () => void
}

export function EditDescriptionModal({
  open,
  onOpenChange,
  projectName,
  initialData,
  onSaved,
}: EditDescriptionModalProps) {
  const [goal, setGoal] = useState('')
  const [techStack, setTechStack] = useState('')
  const [keyFeatures, setKeyFeatures] = useState('')
  const [notes, setNotes] = useState('')
  const [isSaving, setIsSaving] = useState(false)

  // Reset form when modal opens
  useEffect(() => {
    if (open) {
      setGoal(initialData?.goal || '')
      setTechStack(initialData?.tech_stack || '')
      setKeyFeatures(initialData?.key_features?.join('\n') || '')
      setNotes(initialData?.notes || '')
    }
  }, [open, initialData])

  const handleSave = async () => {
    try {
      setIsSaving(true)

      // Parse key features (one per line)
      const features = keyFeatures
        .split('\n')
        .map(s => s.trim())
        .filter(s => s.length > 0)

      await projectsService.updateProjectDescription({
        project_name: projectName,
        goal: goal.trim() || null,
        tech_stack: techStack.trim() || null,
        key_features: features.length > 0 ? features : null,
        notes: notes.trim() || null,
      })

      onSaved()
    } catch (err) {
      console.error('Failed to save description:', err)
    } finally {
      setIsSaving(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>編輯專案描述</DialogTitle>
        </DialogHeader>

        <div className="space-y-4 py-4">
          <div className="space-y-2">
            <Label htmlFor="goal">專案目標</Label>
            <Textarea
              id="goal"
              placeholder="描述專案要解決的問題或達成的目標"
              value={goal}
              onChange={(e) => setGoal(e.target.value)}
              rows={2}
            />
            <p className="text-xs text-muted-foreground">
              這會作為 AI 生成摘要的背景資訊
            </p>
          </div>

          <div className="space-y-2">
            <Label htmlFor="tech-stack">技術棧</Label>
            <Input
              id="tech-stack"
              placeholder="例：Tauri, React, Rust, SQLite"
              value={techStack}
              onChange={(e) => setTechStack(e.target.value)}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="key-features">關鍵功能</Label>
            <Textarea
              id="key-features"
              placeholder={"每行一項功能\n例：\n自動捕獲工作 session\nGit commit 追蹤\n工作摘要生成"}
              value={keyFeatures}
              onChange={(e) => setKeyFeatures(e.target.value)}
              rows={4}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="notes">備註</Label>
            <Textarea
              id="notes"
              placeholder="其他補充資訊、開發計畫、注意事項等"
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              rows={3}
            />
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            取消
          </Button>
          <Button onClick={handleSave} disabled={isSaving}>
            {isSaving ? '儲存中...' : '儲存'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
