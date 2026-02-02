import { useState, useEffect, useMemo } from 'react'
import { Pencil, Target, Wrench, Star, FileText, BookOpen, ChevronDown, ChevronUp } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import * as projectsService from '@/services/projects'
import type { ProjectReadmeResponse } from '@/services/projects'
import { EditDescriptionModal } from '../Modals/EditDescriptionModal'
import { MarkdownSummary } from '@/components/MarkdownSummary'
import type { ProjectDetail, ProjectDescription } from '@/types'

const README_PREVIEW_LENGTH = 200

interface InfoTabProps {
  projectName: string
  detail: ProjectDetail
  onUpdate: () => void
}

export function InfoTab({ projectName, detail, onUpdate: _onUpdate }: InfoTabProps) {
  const [description, setDescription] = useState<ProjectDescription | null>(null)
  const [isLoadingDesc, setIsLoadingDesc] = useState(true)
  const [showEditModal, setShowEditModal] = useState(false)

  // README state
  const [readme, setReadme] = useState<ProjectReadmeResponse | null>(null)
  const [isLoadingReadme, setIsLoadingReadme] = useState(true)
  const [isReadmeExpanded, setIsReadmeExpanded] = useState(false)

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

  const fetchReadme = async () => {
    try {
      setIsLoadingReadme(true)
      const data = await projectsService.getProjectReadme(projectName)
      setReadme(data)
    } catch (err) {
      console.error('Failed to load README:', err)
    } finally {
      setIsLoadingReadme(false)
    }
  }

  useEffect(() => {
    fetchDescription()
    fetchReadme()
  }, [projectName])

  // Compute truncated README preview
  const readmePreview = useMemo(() => {
    if (!readme?.content) return null
    const content = readme.content
    if (content.length <= README_PREVIEW_LENGTH) return content
    // Find a good break point (end of word/line)
    const truncated = content.slice(0, README_PREVIEW_LENGTH)
    const lastNewline = truncated.lastIndexOf('\n')
    const lastSpace = truncated.lastIndexOf(' ')
    const breakPoint = lastNewline > README_PREVIEW_LENGTH - 50 ? lastNewline : lastSpace
    return truncated.slice(0, breakPoint > 0 ? breakPoint : README_PREVIEW_LENGTH) + '...'
  }, [readme?.content])

  const needsExpansion = readme?.content && readme.content.length > README_PREVIEW_LENGTH

  const handleDescriptionSaved = () => {
    setShowEditModal(false)
    fetchDescription()
  }

  return (
    <div className="space-y-10">
      {/* README Section */}
      {!isLoadingReadme && readme?.content && (
        <section>
          <div className="flex items-center gap-2 mb-4">
            <BookOpen className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
            <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
              README
            </h2>
            {readme.file_name && (
              <span className="text-[10px] text-muted-foreground/60">
                ({readme.file_name})
              </span>
            )}
          </div>
          <Card>
            <CardContent className="p-6">
              <div className="relative">
                {isReadmeExpanded ? (
                  <MarkdownSummary content={readme.content} />
                ) : (
                  <MarkdownSummary content={readmePreview || readme.content} />
                )}
                {needsExpansion && (
                  <div className={`${!isReadmeExpanded ? 'mt-2 pt-2 border-t border-border' : 'mt-4'}`}>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="text-xs text-muted-foreground hover:text-foreground p-0 h-auto"
                      onClick={() => setIsReadmeExpanded(!isReadmeExpanded)}
                    >
                      {isReadmeExpanded ? (
                        <>
                          <ChevronUp className="w-3 h-3 mr-1" strokeWidth={1.5} />
                          收起
                        </>
                      ) : (
                        <>
                          <ChevronDown className="w-3 h-3 mr-1" strokeWidth={1.5} />
                          閱讀更多
                        </>
                      )}
                    </Button>
                  </div>
                )}
              </div>
            </CardContent>
          </Card>
        </section>
      )}

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
