import { useCallback, useEffect, useState } from 'react'
import { GitBranch, Bot, FolderOpen } from 'lucide-react'
import { Card } from '@/components/ui/card'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
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
    <Dialog open={isOpen} onOpenChange={(open) => { if (!open) onClose() }}>
      <DialogContent className="max-w-md max-h-[80vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle className="truncate">
            {detail?.display_name || detail?.project_name || projectName || ''}
          </DialogTitle>
        </DialogHeader>

        <div className="space-y-4 pt-2">
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
      </DialogContent>
    </Dialog>
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
