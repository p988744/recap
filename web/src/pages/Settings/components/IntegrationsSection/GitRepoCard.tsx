import {
  FolderGit2,
  CheckCircle2,
  XCircle,
  Plus,
  Trash2,
  Loader2,
  Terminal,
} from 'lucide-react'
import { Card } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import type { SourcesResponse, ClaudeProject } from '@/types'
import type { SettingsMessage } from '../../hooks/useSettings'
import { sources as sourcesService } from '@/services'

interface GitRepoCardProps {
  sources: SourcesResponse | null
  claudeProjects: ClaudeProject[]
  newRepoPath: string
  setNewRepoPath: (v: string) => void
  adding: boolean
  onAdd: (
    setMessage: (msg: SettingsMessage | null) => void,
    refreshSources: () => Promise<SourcesResponse>
  ) => Promise<void>
  onRemove: (
    repoId: string,
    setMessage: (msg: SettingsMessage | null) => void,
    refreshSources: () => Promise<SourcesResponse>
  ) => Promise<void>
  setMessage: (msg: SettingsMessage | null) => void
  refreshSources: () => Promise<SourcesResponse>
  setSources: (sources: SourcesResponse) => void
}

export function GitRepoCard({
  sources,
  claudeProjects,
  newRepoPath,
  setNewRepoPath,
  adding,
  onAdd,
  onRemove,
  setMessage,
  refreshSources,
  setSources,
}: GitRepoCardProps) {
  const handleAddClaudeProject = async (path: string, name: string) => {
    setMessage(null)
    try {
      await sourcesService.addGitRepo(path)
      const updated = await refreshSources()
      setSources(updated)
      setMessage({ type: 'success', text: `已新增 ${name}` })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '新增失敗' })
    }
  }

  const existingPaths = sources?.git_repos?.map(r => r.path) || []
  const unaddedProjects = claudeProjects.filter(p => !existingPaths.some(ep => ep === p.path))

  return (
    <Card className="p-6">
      <div className="flex items-center gap-3 mb-6">
        <div className="w-10 h-10 rounded-lg bg-emerald-500/10 flex items-center justify-center">
          <FolderGit2 className="w-5 h-5 text-emerald-600" strokeWidth={1.5} />
        </div>
        <div className="flex-1">
          <h3 className="font-medium text-foreground">本地 Git 倉庫</h3>
          <p className="text-xs text-muted-foreground">追蹤本地 Git 專案的 commits</p>
        </div>
        {sources?.git_repos && sources.git_repos.length > 0 ? (
          <span className="flex items-center gap-1.5 text-xs text-sage">
            <CheckCircle2 className="w-3.5 h-3.5" strokeWidth={1.5} />
            {sources.git_repos.length} 個倉庫
          </span>
        ) : (
          <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <XCircle className="w-3.5 h-3.5" strokeWidth={1.5} />
            未設定
          </span>
        )}
      </div>

      <div className="space-y-4">
        {/* Existing repos */}
        {sources?.git_repos && sources.git_repos.length > 0 && (
          <div className="space-y-2">
            {sources.git_repos.map((repo) => (
              <div
                key={repo.id}
                className="flex items-center justify-between p-3 bg-muted/30 rounded-lg"
              >
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium text-foreground truncate">{repo.name}</p>
                  <p className="text-xs text-muted-foreground truncate">{repo.path}</p>
                </div>
                <div className="flex items-center gap-2 ml-4">
                  {repo.valid ? (
                    <CheckCircle2 className="w-4 h-4 text-sage" strokeWidth={1.5} />
                  ) : (
                    <XCircle className="w-4 h-4 text-destructive" strokeWidth={1.5} />
                  )}
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => onRemove(repo.id, setMessage, refreshSources)}
                    className="text-muted-foreground hover:text-destructive"
                  >
                    <Trash2 className="w-4 h-4" strokeWidth={1.5} />
                  </Button>
                </div>
              </div>
            ))}
          </div>
        )}

        {/* Claude Code detected repos */}
        {claudeProjects.length > 0 && (
          <div className="space-y-2">
            <p className="text-xs text-muted-foreground flex items-center gap-1.5">
              <Terminal className="w-3.5 h-3.5" strokeWidth={1.5} />
              從 Claude Code 專案偵測到的 Git 倉庫
            </p>
            {unaddedProjects.slice(0, 5).map((project) => (
              <div
                key={project.path}
                className="flex items-center justify-between p-3 border border-dashed border-purple-300/50 bg-purple-50/30 rounded-lg"
              >
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium text-foreground truncate">{project.name}</p>
                  <p className="text-xs text-muted-foreground truncate">{project.path}</p>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => handleAddClaudeProject(project.path, project.name)}
                  className="text-purple-600 border-purple-300 hover:bg-purple-100"
                >
                  <Plus className="w-4 h-4" strokeWidth={1.5} />
                  新增
                </Button>
              </div>
            ))}
            {unaddedProjects.length === 0 && (
              <p className="text-xs text-muted-foreground text-center py-2">
                所有 Claude Code 專案都已新增
              </p>
            )}
          </div>
        )}

        {/* Add new repo */}
        <div className="flex items-center gap-2">
          <Input
            value={newRepoPath}
            onChange={(e) => setNewRepoPath(e.target.value)}
            placeholder="輸入 Git 倉庫路徑，例如 ~/Projects/my-app"
            className="flex-1"
            onKeyDown={(e) => e.key === 'Enter' && onAdd(setMessage, refreshSources)}
          />
          <Button
            variant="outline"
            onClick={() => onAdd(setMessage, refreshSources)}
            disabled={adding || !newRepoPath.trim()}
          >
            {adding ? (
              <Loader2 className="w-4 h-4 animate-spin" strokeWidth={1.5} />
            ) : (
              <Plus className="w-4 h-4" strokeWidth={1.5} />
            )}
            新增
          </Button>
        </div>
      </div>
    </Card>
  )
}
