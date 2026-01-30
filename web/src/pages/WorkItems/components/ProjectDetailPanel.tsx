import { useCallback, useEffect, useState } from 'react'
import { X, GitBranch, Bot, GitMerge, FileText, Clock, FolderOpen, Layers } from 'lucide-react'
import { Card } from '@/components/ui/card'
import { projects as projectsService } from '@/services'
import type { ProjectDetail } from '@/types'

const SOURCE_LABELS: Record<string, string> = {
  git: 'Git',
  claude_code: 'Claude Code',
  gitlab: 'GitLab',
  manual: '手動',
}

const SOURCE_ICONS: Record<string, typeof GitBranch> = {
  git: GitBranch,
  claude_code: Bot,
  gitlab: GitMerge,
  manual: FileText,
  aggregated: Layers,
}

function formatHours(hours: number): string {
  if (hours < 1) return `${Math.round(hours * 60)}m`
  return `${hours.toFixed(1)}h`
}

interface ProjectDetailPanelProps {
  projectName: string | null
  onClose: () => void
}

export function ProjectDetailPanel({ projectName, onClose }: ProjectDetailPanelProps) {
  const [detail, setDetail] = useState<ProjectDetail | null>(null)
  const [loading, setLoading] = useState(false)

  const fetchDetail = useCallback(async (name: string) => {
    setLoading(true)
    try {
      const data = await projectsService.getProjectDetail(name)
      setDetail(data)
    } catch (err) {
      console.error('Failed to fetch project detail:', err)
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    if (projectName) {
      fetchDetail(projectName)
    } else {
      setDetail(null)
    }
  }, [projectName, fetchDetail])

  const isOpen = projectName !== null

  return (
    <>
      {/* Backdrop */}
      {isOpen && (
        <div
          className="fixed inset-0 bg-black/20 z-40 transition-opacity"
          onClick={onClose}
        />
      )}

      {/* Panel */}
      <div
        className={`fixed top-0 right-0 h-full w-[420px] max-w-[90vw] bg-background border-l border-border z-50 shadow-xl
          transform transition-transform duration-200 ease-out
          ${isOpen ? 'translate-x-0' : 'translate-x-full'}`}
      >
        {/* Panel header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-border">
          <h3 className="font-display text-lg text-foreground truncate">
            {detail?.display_name || detail?.project_name || projectName || ''}
          </h3>
          <button
            onClick={onClose}
            className="p-1.5 rounded hover:bg-foreground/10 transition-colors shrink-0 ml-2"
          >
            <X className="w-4 h-4 text-muted-foreground" />
          </button>
        </div>

        {/* Panel content */}
        <div className="overflow-y-auto h-[calc(100%-57px)] px-5 py-5 space-y-6">
          {loading ? (
            <div className="flex items-center justify-center h-48">
              <div className="w-5 h-5 border border-border border-t-charcoal/60 rounded-full animate-spin" />
            </div>
          ) : detail ? (
            <>
              {/* Project path */}
              {detail.project_path && (
                <p className="text-xs text-muted-foreground flex items-center gap-1">
                  <FolderOpen className="w-3 h-3 shrink-0" />
                  <span className="truncate">{detail.project_path}</span>
                </p>
              )}

              {/* Stats */}
              <div className="grid grid-cols-3 gap-3">
                <Card className="p-3">
                  <p className="text-xs text-muted-foreground">工作項目</p>
                  <p className="text-lg font-display text-foreground mt-1">{detail.stats.total_items}</p>
                </Card>
                <Card className="p-3">
                  <p className="text-xs text-muted-foreground">總時數</p>
                  <p className="text-lg font-display text-foreground mt-1">{formatHours(detail.stats.total_hours)}</p>
                </Card>
                <Card className="p-3">
                  <p className="text-xs text-muted-foreground">時間範圍</p>
                  <p className="text-sm text-foreground mt-1.5">
                    {detail.stats.date_range
                      ? `${detail.stats.date_range[0].slice(5)} ~ ${detail.stats.date_range[1].slice(5)}`
                      : '-'}
                  </p>
                </Card>
              </div>

              {/* Sources */}
              {detail.sources.length > 0 && (
                <div>
                  <h4 className="text-xs uppercase tracking-wider text-muted-foreground mb-2">來源</h4>
                  <Card className="divide-y divide-border">
                    {detail.sources.map((src) => {
                      const Icon = SOURCE_ICONS[src.source] || FileText
                      return (
                        <div key={src.source} className="px-3 py-2.5">
                          <div className="flex items-center gap-3">
                            <Icon className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
                            <span className="text-sm text-foreground flex-1">
                              {SOURCE_LABELS[src.source] || src.source}
                            </span>
                            <span className="text-xs text-muted-foreground">{src.item_count} 項</span>
                            {src.latest_date && (
                              <span className="text-xs text-muted-foreground">
                                最近 {src.latest_date.slice(5)}
                              </span>
                            )}
                          </div>
                          {src.project_path && (
                            <p className="text-xs text-muted-foreground mt-1 ml-7 truncate">
                              {src.project_path}
                            </p>
                          )}
                        </div>
                      )
                    })}
                  </Card>
                </div>
              )}

              {/* Recent Items */}
              {detail.recent_items.length > 0 && (
                <div>
                  <h4 className="text-xs uppercase tracking-wider text-muted-foreground mb-2">
                    最近工作項目
                  </h4>
                  <Card className="divide-y divide-border">
                    {detail.recent_items.map((item) => (
                      <div key={item.id} className="px-3 py-2.5">
                        <div className="flex items-start justify-between gap-2">
                          <p className="text-sm text-foreground leading-snug flex-1 min-w-0 truncate">
                            {item.title}
                          </p>
                          <span className="text-xs text-muted-foreground shrink-0 flex items-center gap-1">
                            <Clock className="w-3 h-3" />
                            {formatHours(item.hours)}
                          </span>
                        </div>
                        <p className="text-xs text-muted-foreground mt-0.5">
                          {item.date} &middot; {SOURCE_LABELS[item.source] || item.source}
                        </p>
                      </div>
                    ))}
                  </Card>
                </div>
              )}
            </>
          ) : null}
        </div>
      </div>
    </>
  )
}
