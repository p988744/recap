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
import { useIntegrations } from '../../context'
import { formatFileSize, formatTimestamp } from '../../hooks'

export function ClaudeCodeCardV2() {
  const { sources, setMessage, refreshSources, claudeCode } = useIntegrations()
  const {
    projects,
    loading,
    selectedProjects,
    expandedProjects,
    importing,
    selectedSessionCount,
    loadSessions,
    toggleExpandProject,
    toggleProjectSelection,
    selectAllProjects,
    clearSelection,
    handleImport,
  } = claudeCode

  return (
    <Card className="p-6">
      <div className="flex items-center gap-3 mb-6">
        <div className="w-10 h-10 rounded-lg bg-violet-500/10 flex items-center justify-center">
          <Terminal className="w-5 h-5 text-violet-600" strokeWidth={1.5} />
        </div>
        <div className="flex-1">
          <h3 className="font-medium text-foreground">Claude Code</h3>
          <p className="text-xs text-muted-foreground">匯入 Claude Code 的工作 session</p>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={() => loadSessions(sources, setMessage, refreshSources)}
          disabled={loading}
        >
          {loading ? (
            <Loader2 className="w-4 h-4 animate-spin" strokeWidth={1.5} />
          ) : (
            <RefreshCw className="w-4 h-4" strokeWidth={1.5} />
          )}
          掃描
        </Button>
      </div>

      {projects.length > 0 && (
        <>
          {/* Selection controls */}
          <div className="flex items-center justify-between mb-4 pb-4 border-b border-border">
            <div className="flex items-center gap-2">
              <Button variant="ghost" size="sm" onClick={selectAllProjects}>
                全選
              </Button>
              <Button variant="ghost" size="sm" onClick={clearSelection}>
                清除
              </Button>
            </div>
            <span className="text-xs text-muted-foreground">
              已選 {selectedProjects.size} 個專案，{selectedSessionCount} 個 session
            </span>
          </div>

          {/* Project list */}
          <div className="space-y-2 max-h-80 overflow-y-auto">
            {projects.map((project) => (
              <div key={project.path} className="border border-border rounded">
                <div
                  className="flex items-center gap-2 p-3 cursor-pointer hover:bg-foreground/5"
                  onClick={() => toggleExpandProject(project.path)}
                >
                  {expandedProjects.has(project.path) ? (
                    <ChevronDown className="w-4 h-4 text-muted-foreground" />
                  ) : (
                    <ChevronRight className="w-4 h-4 text-muted-foreground" />
                  )}
                  <input
                    type="checkbox"
                    checked={selectedProjects.has(project.path)}
                    onChange={(e) => {
                      e.stopPropagation()
                      toggleProjectSelection(project.path)
                    }}
                    onClick={(e) => e.stopPropagation()}
                    className="rounded"
                  />
                  <FolderGit2 className="w-4 h-4 text-muted-foreground" />
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium truncate">{project.name}</p>
                    <p className="text-xs text-muted-foreground">
                      {project.sessions.length} sessions
                    </p>
                  </div>
                </div>

                {/* Expanded sessions */}
                {expandedProjects.has(project.path) && project.sessions.length > 0 && (
                  <div className="border-t border-border bg-foreground/[0.02]">
                    {project.sessions.slice(0, 5).map((session) => (
                      <div
                        key={session.session_id}
                        className="px-3 py-2 pl-12 text-xs flex items-center gap-4 border-b border-border/50 last:border-0"
                      >
                        <span className="text-muted-foreground font-mono">
                          {session.session_id.slice(0, 8)}
                        </span>
                        <span className="text-muted-foreground">
                          {formatTimestamp(session.last_timestamp)}
                        </span>
                        <span className="text-muted-foreground">
                          {formatFileSize(session.file_size)}
                        </span>
                      </div>
                    ))}
                    {project.sessions.length > 5 && (
                      <div className="px-3 py-2 pl-12 text-xs text-muted-foreground">
                        +{project.sessions.length - 5} more sessions
                      </div>
                    )}
                  </div>
                )}
              </div>
            ))}
          </div>

          {/* Import button */}
          <div className="mt-4 pt-4 border-t border-border">
            <Button
              onClick={() => handleImport(setMessage)}
              disabled={importing || selectedProjects.size === 0}
              className="w-full"
            >
              {importing ? (
                <Loader2 className="w-4 h-4 animate-spin" strokeWidth={1.5} />
              ) : (
                <Download className="w-4 h-4" strokeWidth={1.5} />
              )}
              匯入 {selectedSessionCount} 個 session
            </Button>
          </div>
        </>
      )}

      {projects.length === 0 && !loading && (
        <p className="text-sm text-muted-foreground text-center py-8">
          點擊「掃描」以偵測 Claude Code sessions
        </p>
      )}
    </Card>
  )
}
