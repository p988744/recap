import { useCallback, useEffect, useState } from 'react'
import { X, GitBranch, Bot, FolderOpen } from 'lucide-react'
import { Card } from '@/components/ui/card'
import { projects as projectsService } from '@/services'
import type { ProjectDetail, ProjectDirectories } from '@/types'

interface ProjectSourcePanelProps {
  projectName: string | null
  onClose: () => void
}

export function ProjectSourcePanel({ projectName, onClose }: ProjectSourcePanelProps) {
  const [detail, setDetail] = useState<ProjectDetail | null>(null)
  const [dirs, setDirs] = useState<ProjectDirectories | null>(null)
  const [loading, setLoading] = useState(false)

  const fetchData = useCallback(async (name: string) => {
    setLoading(true)
    try {
      const [detailData, dirsData] = await Promise.all([
        projectsService.getProjectDetail(name),
        projectsService.getProjectDirectories(name),
      ])
      setDetail(detailData)
      setDirs(dirsData)
    } catch (err) {
      console.error('Failed to fetch project data:', err)
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    if (projectName) {
      fetchData(projectName)
    } else {
      setDetail(null)
      setDirs(null)
    }
  }, [projectName, fetchData])

  const isOpen = projectName !== null
  const claudeDirs = dirs?.claude_code_dirs ?? []
  const totalSessions = claudeDirs.reduce((sum, d) => sum + d.session_count, 0)

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
        {/* Header */}
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

        {/* Content */}
        <div className="overflow-y-auto h-[calc(100%-57px)] px-5 py-5 space-y-5">
          {loading ? (
            <div className="flex items-center justify-center h-32">
              <div className="w-5 h-5 border border-border border-t-charcoal/60 rounded-full animate-spin" />
            </div>
          ) : detail ? (
            <>
              {/* Project base path */}
              <DirectoryRow
                icon={<FolderOpen className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />}
                label="專案路徑"
                path={detail.project_path}
              />

              {/* Git repo */}
              <DirectoryRow
                icon={<GitBranch className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />}
                label="Git Repo"
                path={dirs?.git_repo_path}
              />

              {/* Claude Code sessions */}
              <div>
                <div className="flex items-center gap-2.5 mb-3">
                  <Bot className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
                  <span className="text-sm font-medium text-foreground">Claude Code</span>
                  {totalSessions > 0 && (
                    <span className="text-xs text-muted-foreground ml-auto">
                      {claudeDirs.length} 個目錄 · {totalSessions} 個 session
                    </span>
                  )}
                </div>
                {claudeDirs.length > 0 ? (
                  <Card className="divide-y divide-border">
                    {claudeDirs.map((dir) => (
                      <div key={dir.path} className="px-4 py-3">
                        <p className="text-xs text-muted-foreground break-all leading-relaxed">
                          {dir.path}
                        </p>
                        <p className="text-xs text-muted-foreground/60 mt-1">
                          {dir.session_count} 個 session
                        </p>
                      </div>
                    ))}
                  </Card>
                ) : (
                  <Card className="p-4">
                    <p className="text-xs text-muted-foreground/50 italic">未偵測到</p>
                  </Card>
                )}
              </div>
            </>
          ) : null}
        </div>
      </div>
    </>
  )
}

function DirectoryRow({
  icon,
  label,
  path,
}: {
  icon: React.ReactNode
  label: string
  path: string | null | undefined
}) {
  return (
    <Card className="p-4">
      <div className="flex items-center gap-2.5 mb-2">
        {icon}
        <span className="text-sm font-medium text-foreground">{label}</span>
      </div>
      {path ? (
        <p className="text-xs text-muted-foreground ml-[26px] break-all leading-relaxed">
          {path}
        </p>
      ) : (
        <p className="text-xs text-muted-foreground/50 ml-[26px] italic">
          未偵測到
        </p>
      )}
    </Card>
  )
}
