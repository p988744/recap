import {
  Terminal,
  RefreshCw,
  Loader2,
  Download,
  ChevronDown,
  ChevronRight,
  FolderGit2,
} from 'lucide-react'
import { Card } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import type { ClaudeProject, SourcesResponse } from '@/types'
import type { SettingsMessage } from '../../hooks/useSettings'
import { formatFileSize, formatTimestamp } from '../../hooks/useSettings'

interface ClaudeCodeCardProps {
  projects: ClaudeProject[]
  loading: boolean
  selectedProjects: Set<string>
  expandedProjects: Set<string>
  importing: boolean
  selectedSessionCount: number
  onLoadSessions: (
    sources: SourcesResponse | null,
    setMessage: (msg: SettingsMessage | null) => void,
    refreshSources: () => Promise<SourcesResponse>
  ) => Promise<void>
  onToggleExpand: (path: string) => void
  onToggleSelection: (path: string) => void
  onSelectAll: () => void
  onClearSelection: () => void
  onImport: (setMessage: (msg: SettingsMessage | null) => void) => Promise<void>
  sources: SourcesResponse | null
  setMessage: (msg: SettingsMessage | null) => void
  refreshSources: () => Promise<SourcesResponse>
}

export function ClaudeCodeCard({
  projects,
  loading,
  selectedProjects,
  expandedProjects,
  importing,
  selectedSessionCount,
  onLoadSessions,
  onToggleExpand,
  onToggleSelection,
  onSelectAll,
  onClearSelection,
  onImport,
  sources,
  setMessage,
  refreshSources,
}: ClaudeCodeCardProps) {
  return (
    <Card className="p-6">
      <div className="flex items-center gap-3 mb-6">
        <div className="w-10 h-10 rounded-lg bg-purple-500/10 flex items-center justify-center">
          <Terminal className="w-5 h-5 text-purple-600" strokeWidth={1.5} />
        </div>
        <div className="flex-1">
          <h3 className="font-medium text-foreground">Claude Code</h3>
          <p className="text-xs text-muted-foreground">匯入開發 Sessions</p>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={() => onLoadSessions(sources, setMessage, refreshSources)}
          disabled={loading}
        >
          {loading ? (
            <Loader2 className="w-4 h-4 animate-spin" strokeWidth={1.5} />
          ) : (
            <RefreshCw className="w-4 h-4" strokeWidth={1.5} />
          )}
          {projects.length > 0 ? '重新載入' : '載入 Sessions'}
        </Button>
      </div>

      {/* Sessions List */}
      {projects.length > 0 ? (
        <div className="space-y-4">
          {/* Action bar */}
          <div className="flex items-center justify-between p-3 bg-muted/30 rounded-lg">
            <div className="flex items-center gap-3">
              <span className="text-sm text-muted-foreground">
                已選擇 {selectedProjects.size} 個專案（{selectedSessionCount} sessions）
              </span>
              <div className="flex items-center gap-1">
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={onSelectAll}
                  disabled={selectedProjects.size === projects.length}
                  className="h-7 px-2 text-xs"
                >
                  全選
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={onClearSelection}
                  disabled={selectedProjects.size === 0}
                  className="h-7 px-2 text-xs"
                >
                  清除
                </Button>
              </div>
            </div>
            <Button
              size="sm"
              onClick={() => onImport(setMessage)}
              disabled={selectedProjects.size === 0 || importing}
            >
              {importing ? (
                <Loader2 className="w-4 h-4 animate-spin" strokeWidth={1.5} />
              ) : (
                <Download className="w-4 h-4" strokeWidth={1.5} />
              )}
              匯入為工作項目
            </Button>
          </div>

          {/* Projects */}
          <div className="space-y-2 max-h-96 overflow-y-auto">
            {projects.map((project) => {
              const isSelected = selectedProjects.has(project.path)
              return (
                <div
                  key={project.path}
                  className={`border rounded-lg overflow-hidden transition-colors ${
                    isSelected ? 'border-purple-400 bg-purple-50/30' : 'border-border'
                  }`}
                >
                  {/* Project header */}
                  <div className="flex items-center gap-2 p-3 bg-muted/20">
                    <input
                      type="checkbox"
                      checked={isSelected}
                      onChange={() => onToggleSelection(project.path)}
                      className="w-4 h-4 accent-purple-600"
                    />
                    <button
                      onClick={() => onToggleExpand(project.path)}
                      className="flex items-center gap-2 flex-1 hover:opacity-70 transition-opacity text-left"
                    >
                      {expandedProjects.has(project.path) ? (
                        <ChevronDown className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
                      ) : (
                        <ChevronRight className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
                      )}
                      <FolderGit2 className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
                      <span className="flex-1 text-sm font-medium truncate">{project.name}</span>
                      <span className="text-xs text-muted-foreground">
                        {project.sessions.length} sessions
                      </span>
                    </button>
                  </div>

                  {/* Sessions (view only) */}
                  {expandedProjects.has(project.path) && (
                    <div className="divide-y divide-border/50">
                      {project.sessions.map((session) => (
                        <div
                          key={session.session_id}
                          className="flex items-start gap-3 p-3 pl-10"
                        >
                          <div className="flex-1 min-w-0">
                            <div className="flex items-center gap-2">
                              <span className="text-xs font-mono text-purple-600">
                                {session.slug}
                              </span>
                              {session.git_branch && (
                                <span className="text-xs text-muted-foreground">
                                  ({session.git_branch})
                                </span>
                              )}
                            </div>
                            {session.first_message && (
                              <p className="text-xs text-muted-foreground mt-1 line-clamp-2">
                                {session.first_message}
                              </p>
                            )}
                            <div className="flex items-center gap-4 mt-1 text-[10px] text-muted-foreground">
                              <span>{formatTimestamp(session.last_timestamp)}</span>
                              <span>{session.message_count} messages</span>
                              <span>{formatFileSize(session.file_size)}</span>
                            </div>
                          </div>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )
            })}
          </div>
        </div>
      ) : (
        <div className="text-center py-8">
          <Terminal className="w-8 h-8 text-muted-foreground mx-auto mb-2" strokeWidth={1.5} />
          <p className="text-sm text-muted-foreground">
            點擊「載入 Sessions」讀取本地 Claude Code 資料
          </p>
          <p className="text-xs text-muted-foreground mt-1">
            資料位於 ~/.claude/projects/
          </p>
        </div>
      )}
    </Card>
  )
}
