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
import { sources as sourcesService } from '@/services'
import { useIntegrations } from '../../context'

export function GitRepoCardV2() {
  const { sources, setSources, setMessage, refreshSources, claudeCode, gitRepo } = useIntegrations()
  const { projects: claudeProjects } = claudeCode
  const { newRepoPath, setNewRepoPath, adding, handleAdd, handleRemove } = gitRepo

  const handleSetMode = async (mode: string) => {
    try {
      await sourcesService.setSourceMode(mode)
      const updated = await refreshSources()
      setSources(updated)
      setMessage({ type: 'success', text: `已切換為${mode === 'local' ? '本地' : '雲端'}模式` })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '切換失敗' })
    }
  }

  return (
    <Card className="p-6">
      <div className="flex items-center gap-3 mb-6">
        <div className="w-10 h-10 rounded-lg bg-sage/10 flex items-center justify-center">
          <FolderGit2 className="w-5 h-5 text-sage" strokeWidth={1.5} />
        </div>
        <div className="flex-1">
          <h3 className="font-medium text-foreground">本地 Git 倉庫</h3>
          <p className="text-xs text-muted-foreground">追蹤本地 Git 專案的 commit 紀錄</p>
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

      {/* Mode Selection */}
      <div className="mb-4">
        <p className="text-xs text-muted-foreground mb-2">資料來源模式</p>
        <div className="flex gap-2">
          <button
            onClick={() => handleSetMode('local')}
            className={`px-3 py-1.5 text-xs rounded border transition-colors ${
              sources?.mode === 'local'
                ? 'bg-foreground text-background border-foreground'
                : 'border-border hover:border-foreground/30'
            }`}
          >
            <Terminal className="w-3 h-3 inline mr-1" />
            本地掃描
          </button>
          <button
            onClick={() => handleSetMode('claude')}
            className={`px-3 py-1.5 text-xs rounded border transition-colors ${
              sources?.mode === 'claude'
                ? 'bg-foreground text-background border-foreground'
                : 'border-border hover:border-foreground/30'
            }`}
          >
            <Terminal className="w-3 h-3 inline mr-1" />
            Claude Code
          </button>
        </div>
      </div>

      {/* Add new repo */}
      <div className="flex gap-2 mb-4">
        <Input
          value={newRepoPath}
          onChange={(e) => setNewRepoPath(e.target.value)}
          placeholder="/path/to/repo"
          className="flex-1"
          list="claude-projects-list"
        />
        <datalist id="claude-projects-list">
          {claudeProjects.map((p) => (
            <option key={p.path} value={p.path} />
          ))}
        </datalist>
        <Button
          onClick={() => handleAdd(setMessage, refreshSources)}
          disabled={adding || !newRepoPath.trim()}
          size="sm"
        >
          {adding ? (
            <Loader2 className="w-4 h-4 animate-spin" strokeWidth={1.5} />
          ) : (
            <Plus className="w-4 h-4" strokeWidth={1.5} />
          )}
          新增
        </Button>
      </div>

      {/* Repo list */}
      {sources?.git_repos && sources.git_repos.length > 0 && (
        <div className="space-y-2">
          {sources.git_repos.map((repo) => (
            <div
              key={repo.id}
              className="flex items-center justify-between p-3 bg-foreground/5 rounded"
            >
              <div className="flex-1 min-w-0">
                <p className="text-sm font-medium truncate">{repo.name}</p>
                <p className="text-xs text-muted-foreground truncate">{repo.path}</p>
              </div>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => handleRemove(repo.id, setMessage, refreshSources)}
                className="text-muted-foreground hover:text-destructive"
              >
                <Trash2 className="w-4 h-4" strokeWidth={1.5} />
              </Button>
            </div>
          ))}
        </div>
      )}
    </Card>
  )
}
