import { useState, useEffect } from 'react'
import { Pencil, Target, Wrench, Star, FileText } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
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
    <div className="space-y-10">
      {/* Project Description */}
      <section>
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <Target className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
            <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
              專案描述
            </h2>
          </div>
          <Button variant="ghost" size="sm" onClick={() => setShowEditModal(true)}>
            <Pencil className="w-4 h-4 mr-1" />
            編輯
          </Button>
        </div>
        <Card>
          <CardContent className="p-6 space-y-4">
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
            <div className="text-center py-6">
              <Target className="w-8 h-8 text-muted-foreground/50 mx-auto mb-3" strokeWidth={1.5} />
              <p className="text-sm text-muted-foreground mb-3">
                尚未填寫專案描述
              </p>
              <Button variant="outline" size="sm" onClick={() => setShowEditModal(true)}>
                <Pencil className="w-3 h-3 mr-1.5" />
                新增描述
              </Button>
            </div>
          )}
        </CardContent>
        </Card>
      </section>

      {/* Stats */}
      <section>
        <div className="flex items-center gap-2 mb-4">
          <Star className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
          <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
            統計
          </h2>
        </div>
        <Card>
          <CardContent className="p-6">
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
      </section>

      {/* Sources */}
      <section>
        <div className="flex items-center gap-2 mb-4">
          <Wrench className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
          <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
            資料來源
          </h2>
        </div>
        <Card>
          <CardContent className="p-6">
            <div className="space-y-3">
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
      </section>

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
