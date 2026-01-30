import { useState, useEffect } from 'react'
import { Pencil, Target, Wrench, Star, FileText } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { projects as projectsService } from '@/services'
import { EditDescriptionModal } from '../Modals/EditDescriptionModal'
import type { ProjectDetail, ProjectDescription } from '@/types'

interface InfoTabProps {
  projectName: string
  detail: ProjectDetail
  onUpdate: () => void
}

export function InfoTab({ projectName, detail, onUpdate: _onUpdate }: InfoTabProps) {
  const [description, setDescription] = useState<ProjectDescription | null>(null)
  const [isLoadingDesc, setIsLoadingDesc] = useState(true)
  const [showEditModal, setShowEditModal] = useState(false)

  const fetchDescription = async () => {
    try {
      setIsLoadingDesc(true)
      const data = await projectsService.getProjectDescription(projectName)
      setDescription(data)
    } catch (err) {
      console.error('Failed to load description:', err)
    } finally {
      setIsLoadingDesc(false)
    }
  }

  useEffect(() => {
    fetchDescription()
  }, [projectName])

  const handleDescriptionSaved = () => {
    setShowEditModal(false)
    fetchDescription()
  }

  return (
    <div className="space-y-6">
      {/* Project Description */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between pb-2">
          <CardTitle className="text-base">專案描述</CardTitle>
          <Button variant="ghost" size="sm" onClick={() => setShowEditModal(true)}>
            <Pencil className="w-4 h-4 mr-1" />
            編輯
          </Button>
        </CardHeader>
        <CardContent className="space-y-4">
          {isLoadingDesc ? (
            <p className="text-sm text-muted-foreground">載入中...</p>
          ) : description ? (
            <>
              {description.goal && (
                <div>
                  <div className="flex items-center gap-2 text-sm font-medium mb-1">
                    <Target className="w-4 h-4 text-muted-foreground" />
                    專案目標
                  </div>
                  <p className="text-sm text-muted-foreground pl-6">{description.goal}</p>
                </div>
              )}
              {description.tech_stack && (
                <div>
                  <div className="flex items-center gap-2 text-sm font-medium mb-1">
                    <Wrench className="w-4 h-4 text-muted-foreground" />
                    技術棧
                  </div>
                  <p className="text-sm text-muted-foreground pl-6">{description.tech_stack}</p>
                </div>
              )}
              {description.key_features && description.key_features.length > 0 && (
                <div>
                  <div className="flex items-center gap-2 text-sm font-medium mb-1">
                    <Star className="w-4 h-4 text-muted-foreground" />
                    關鍵功能
                  </div>
                  <ul className="text-sm text-muted-foreground pl-6 list-disc list-inside">
                    {description.key_features.map((feature, i) => (
                      <li key={i}>{feature}</li>
                    ))}
                  </ul>
                </div>
              )}
              {description.notes && (
                <div>
                  <div className="flex items-center gap-2 text-sm font-medium mb-1">
                    <FileText className="w-4 h-4 text-muted-foreground" />
                    備註
                  </div>
                  <p className="text-sm text-muted-foreground pl-6 whitespace-pre-wrap">{description.notes}</p>
                </div>
              )}
              {!description.goal && !description.tech_stack && !description.key_features?.length && !description.notes && (
                <p className="text-sm text-muted-foreground">尚未填寫專案描述</p>
              )}
            </>
          ) : (
            <p className="text-sm text-muted-foreground">
              尚未填寫專案描述。點擊編輯按鈕新增。
            </p>
          )}
        </CardContent>
      </Card>

      {/* Stats */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-base">統計</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-3 gap-4 text-center">
            <div>
              <p className="text-2xl font-semibold">{detail.stats.total_hours.toFixed(1)}</p>
              <p className="text-xs text-muted-foreground">總時數</p>
            </div>
            <div>
              <p className="text-2xl font-semibold">{detail.stats.total_items}</p>
              <p className="text-xs text-muted-foreground">工作項目</p>
            </div>
            <div>
              <p className="text-2xl font-semibold">{detail.sources.length}</p>
              <p className="text-xs text-muted-foreground">資料來源</p>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Sources */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-base">資料來源</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-2">
            {detail.sources.map((source) => (
              <div key={source.source} className="flex items-center justify-between text-sm">
                <span className="capitalize">{source.source.replace('_', ' ')}</span>
                <div className="flex items-center gap-4 text-muted-foreground">
                  <span>{source.item_count} 項目</span>
                  {source.latest_date && (
                    <span>最近: {source.latest_date}</span>
                  )}
                </div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>

      {/* Edit Modal */}
      <EditDescriptionModal
        open={showEditModal}
        onOpenChange={setShowEditModal}
        projectName={projectName}
        initialData={description}
        onSaved={handleDescriptionSaved}
      />
    </div>
  )
}
