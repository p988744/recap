import {
  GitBranch,
  CheckCircle2,
  XCircle,
  Save,
  Loader2,
  Eye,
  EyeOff,
  Link2,
  Trash2,
  RefreshCw,
  Plus,
} from 'lucide-react'
import { Card } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { useIntegrations } from '../../context'
import { formatTimestamp } from '../../hooks'

export function GitLabCardV2() {
  const { config, setMessage, refreshConfig, gitlab } = useIntegrations()
  const {
    gitlabUrl,
    setGitlabUrl,
    gitlabToken,
    setGitlabToken,
    showToken,
    setShowToken,
    saving,
    testing,
    projects,
    searchResults,
    search,
    setSearch,
    searching,
    syncing,
    handleSave,
    handleTest,
    handleSearch,
    handleAddProject,
    handleRemoveProject,
    handleSync,
    handleRemoveConfig,
  } = gitlab

  return (
    <Card className="p-6">
      <div className="flex items-center gap-3 mb-6">
        <div className="w-10 h-10 rounded-lg bg-orange-500/10 flex items-center justify-center">
          <GitBranch className="w-5 h-5 text-orange-600" strokeWidth={1.5} />
        </div>
        <div className="flex-1">
          <h3 className="font-medium text-foreground">GitLab</h3>
          <p className="text-xs text-muted-foreground">連接 GitLab 同步 commit 紀錄</p>
        </div>
        {config?.gitlab_configured ? (
          <span className="flex items-center gap-1.5 text-xs text-sage">
            <CheckCircle2 className="w-3.5 h-3.5" strokeWidth={1.5} />
            已設定
          </span>
        ) : (
          <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <XCircle className="w-3.5 h-3.5" strokeWidth={1.5} />
            未設定
          </span>
        )}
      </div>

      <div className="space-y-4">
        <div>
          <Label htmlFor="gitlab-url" className="mb-2 block text-xs">GitLab URL</Label>
          <Input
            id="gitlab-url"
            value={gitlabUrl}
            onChange={(e) => setGitlabUrl(e.target.value)}
            placeholder="https://gitlab.example.com"
          />
        </div>

        <div>
          <Label htmlFor="gitlab-token" className="mb-2 block text-xs">Personal Access Token</Label>
          <div className="relative">
            <Input
              id="gitlab-token"
              type={showToken ? 'text' : 'password'}
              value={gitlabToken}
              onChange={(e) => setGitlabToken(e.target.value)}
              placeholder={config?.gitlab_configured ? '••••••••（已設定）' : '輸入 token'}
              className="pr-10"
            />
            <button
              type="button"
              onClick={() => setShowToken(!showToken)}
              className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
            >
              {showToken ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
            </button>
          </div>
        </div>

        <div className="flex gap-2 pt-4 border-t border-border">
          <Button onClick={() => handleSave(setMessage, refreshConfig)} disabled={saving}>
            {saving ? <Loader2 className="w-4 h-4 animate-spin" /> : <Save className="w-4 h-4" />}
            儲存
          </Button>
          <Button
            variant="outline"
            onClick={() => handleTest(setMessage)}
            disabled={testing || !config?.gitlab_configured}
          >
            {testing ? <Loader2 className="w-4 h-4 animate-spin" /> : null}
            測試
          </Button>
          {config?.gitlab_configured && (
            <Button
              variant="outline"
              onClick={() => handleRemoveConfig(setMessage, refreshConfig)}
              className="text-destructive hover:text-destructive"
            >
              <Trash2 className="w-4 h-4" />
              移除
            </Button>
          )}
        </div>
      </div>

      {/* Projects Section (only shown when configured) */}
      {config?.gitlab_configured && (
        <div className="mt-6 pt-6 border-t border-border">
          <div className="flex items-center justify-between mb-4">
            <h4 className="text-sm font-medium">已追蹤的專案</h4>
            <Button
              variant="outline"
              size="sm"
              onClick={() => handleSync(setMessage)}
              disabled={syncing}
            >
              {syncing ? (
                <Loader2 className="w-4 h-4 animate-spin" />
              ) : (
                <RefreshCw className="w-4 h-4" />
              )}
              同步
            </Button>
          </div>

          {/* Project list */}
          {projects.length > 0 ? (
            <div className="space-y-2 mb-4">
              {projects.map((project) => (
                <div
                  key={project.id}
                  className="flex items-center justify-between p-3 bg-foreground/5 rounded"
                >
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <Link2 className="w-3.5 h-3.5 text-muted-foreground" />
                      <p className="text-sm font-medium truncate">{project.name}</p>
                    </div>
                    <p className="text-xs text-muted-foreground mt-0.5">
                      最後同步: {formatTimestamp(project.last_synced)}
                    </p>
                  </div>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => handleRemoveProject(project.id, setMessage)}
                    className="text-muted-foreground hover:text-destructive"
                  >
                    <Trash2 className="w-4 h-4" />
                  </Button>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-sm text-muted-foreground mb-4">尚未追蹤任何專案</p>
          )}

          {/* Search projects */}
          <div>
            <Label className="mb-2 block text-xs">搜尋專案</Label>
            <div className="flex gap-2">
              <Input
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                placeholder="輸入專案名稱..."
                className="flex-1"
              />
              <Button
                onClick={() => handleSearch(setMessage)}
                disabled={searching || !search.trim()}
                size="sm"
              >
                {searching ? <Loader2 className="w-4 h-4 animate-spin" /> : '搜尋'}
              </Button>
            </div>
          </div>

          {/* Search results */}
          {searchResults.length > 0 && (
            <div className="mt-4 space-y-2">
              {searchResults.map((project) => (
                <div
                  key={project.id}
                  className="flex items-center justify-between p-3 border border-border rounded"
                >
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium truncate">{project.path_with_namespace}</p>
                    <p className="text-xs text-muted-foreground truncate">{project.web_url}</p>
                  </div>
                  <Button
                    size="sm"
                    onClick={() => handleAddProject(project.id, setMessage)}
                  >
                    <Plus className="w-4 h-4" />
                    新增
                  </Button>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </Card>
  )
}
