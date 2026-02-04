import { useState, useEffect, useMemo } from 'react'
import { Target, Wrench, Star, BookOpen, ChevronDown, ChevronUp } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import * as projectsService from '@/services/projects'
import type { ProjectReadmeResponse } from '@/services/projects'
import { MarkdownSummary } from '@/components/MarkdownSummary'
import type { ProjectDetail } from '@/types'

const README_PREVIEW_LENGTH = 200

interface InfoTabProps {
  projectName: string
  detail: ProjectDetail
  onUpdate: () => void
}

export function InfoTab({ projectName, detail, onUpdate: _onUpdate }: InfoTabProps) {
  // README state
  const [readme, setReadme] = useState<ProjectReadmeResponse | null>(null)
  const [isLoadingReadme, setIsLoadingReadme] = useState(true)
  const [isReadmeExpanded, setIsReadmeExpanded] = useState(false)

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

  return (
    <div className="space-y-10">
      {/* Project Description - shows README content */}
      <section>
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <Target className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
            <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
              專案描述
            </h2>
            {readme?.file_name && (
              <span className="text-[10px] text-muted-foreground/60">
                ({readme.file_name})
              </span>
            )}
          </div>
        </div>
        <Card>
          <CardContent className="p-6">
            {isLoadingReadme ? (
              <p className="text-sm text-muted-foreground">載入中...</p>
            ) : readme?.content ? (
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
            ) : (
              <div className="text-center py-6">
                <BookOpen className="w-8 h-8 text-muted-foreground/50 mx-auto mb-3" strokeWidth={1.5} />
                <p className="text-sm text-muted-foreground">
                  此專案沒有 README 檔案
                </p>
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
    </div>
  )
}
