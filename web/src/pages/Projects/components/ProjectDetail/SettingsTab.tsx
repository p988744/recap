import { useState } from 'react'
import { Settings, AlertTriangle, Trash2 } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import { Card, CardContent } from '@/components/ui/card'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from '@/components/ui/alert-dialog'
import { projects as projectsService } from '@/services'
import type { ProjectDetail } from '@/types'

interface SettingsTabProps {
  projectName: string
  detail: ProjectDetail
  onUpdate: () => void
}

export function SettingsTab({ projectName, detail, onUpdate }: SettingsTabProps) {
  const [isVisible, setIsVisible] = useState(!detail.hidden)
  const [isSaving, setIsSaving] = useState(false)

  const handleVisibilityChange = async (visible: boolean) => {
    try {
      setIsSaving(true)
      await projectsService.setProjectVisibility(projectName, !visible)
      setIsVisible(visible)
      onUpdate()
    } catch (err) {
      console.error('Failed to update visibility:', err)
      setIsVisible(!visible) // Revert on error
    } finally {
      setIsSaving(false)
    }
  }

  const handleRemoveProject = async () => {
    try {
      await projectsService.removeManualProject(projectName)
      onUpdate()
      // TODO: Navigate back to project list
    } catch (err) {
      console.error('Failed to remove project:', err)
    }
  }

  return (
    <div className="space-y-10">
      {/* Project Settings */}
      <section>
        <div className="flex items-center gap-2 mb-4">
          <Settings className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
          <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
            專案設定
          </h2>
        </div>
        <Card>
          <CardContent className="p-6 space-y-4">
            <div className="space-y-2">
              <Label>顯示名稱</Label>
            <Input
              value={detail.display_name || detail.project_name}
              disabled
              className="bg-muted"
            />
            <p className="text-xs text-muted-foreground">
              顯示名稱目前無法修改
            </p>
          </div>

          <div className="space-y-2">
            <Label>專案路徑</Label>
            <Input
              value={detail.project_path || '無'}
              disabled
              className="bg-muted font-mono text-xs"
            />
          </div>

            <div className="flex items-center justify-between">
              <div className="space-y-0.5">
                <Label>可見性</Label>
                <p className="text-xs text-muted-foreground">
                  在專案列表中顯示此專案
                </p>
              </div>
              <Switch
                checked={isVisible}
                onCheckedChange={handleVisibilityChange}
                disabled={isSaving}
              />
            </div>
          </CardContent>
        </Card>
      </section>

      {/* Danger Zone */}
      <section>
        <div className="flex items-center gap-2 mb-4">
          <AlertTriangle className="w-4 h-4 text-destructive" strokeWidth={1.5} />
          <h2 className="text-[10px] uppercase tracking-[0.2em] text-destructive">
            危險區域
          </h2>
        </div>
        <Card className="border-destructive/50">
          <CardContent className="p-6">
            <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium">移除專案</p>
              <p className="text-xs text-muted-foreground">
                從 Recap 中移除此專案（不會刪除實際檔案）
              </p>
            </div>
            <AlertDialog>
              <AlertDialogTrigger asChild>
                <Button variant="destructive" size="sm">
                  <Trash2 className="w-4 h-4 mr-1" />
                  移除專案
                </Button>
              </AlertDialogTrigger>
              <AlertDialogContent>
                <AlertDialogHeader>
                  <AlertDialogTitle>確定要移除專案？</AlertDialogTitle>
                  <AlertDialogDescription>
                    此操作會將專案從 Recap 中移除。工作項目紀錄不會被刪除，
                    但專案描述和摘要會一併刪除。
                  </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                  <AlertDialogCancel>取消</AlertDialogCancel>
                  <AlertDialogAction onClick={handleRemoveProject}>
                    確認移除
                  </AlertDialogAction>
                </AlertDialogFooter>
              </AlertDialogContent>
            </AlertDialog>
          </div>
        </CardContent>
        </Card>
      </section>
    </div>
  )
}
