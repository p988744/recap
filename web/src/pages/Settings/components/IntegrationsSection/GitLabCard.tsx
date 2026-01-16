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
import type { ConfigResponse, GitLabProject, GitLabProjectInfo } from '@/types'
import type { SettingsMessage } from '../../hooks/useSettings'
import { formatTimestamp } from '../../hooks/useSettings'

interface GitLabCardProps {
  config: ConfigResponse | null
  gitlabUrl: string
  setGitlabUrl: (v: string) => void
  gitlabToken: string
  setGitlabToken: (v: string) => void
  showToken: boolean
  setShowToken: (v: boolean) => void
  saving: boolean
  testing: boolean
  projects: GitLabProject[]
  searchResults: GitLabProjectInfo[]
  search: string
  setSearch: (v: string) => void
  searching: boolean
  syncing: boolean
  onSave: (
    setMessage: (msg: SettingsMessage | null) => void,
    refreshConfig: () => Promise<ConfigResponse>
  ) => Promise<void>
  onTest: (setMessage: (msg: SettingsMessage | null) => void) => Promise<void>
  onSearch: (setMessage: (msg: SettingsMessage | null) => void) => Promise<void>
  onAddProject: (projectId: number, setMessage: (msg: SettingsMessage | null) => void) => Promise<void>
  onRemoveProject: (id: string, setMessage: (msg: SettingsMessage | null) => void) => Promise<void>
  onSync: (setMessage: (msg: SettingsMessage | null) => void) => Promise<void>
  onRemoveConfig: (
    setMessage: (msg: SettingsMessage | null) => void,
    refreshConfig: () => Promise<ConfigResponse>
  ) => Promise<void>
  setMessage: (msg: SettingsMessage | null) => void
  refreshConfig: () => Promise<ConfigResponse>
}

export function GitLabCard({
  config,
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
  onSave,
  onTest,
  onSearch,
  onAddProject,
  onRemoveProject,
  onSync,
  onRemoveConfig,
  setMessage,
  refreshConfig,
}: GitLabCardProps) {
  return (
    <Card className="p-6">
      <div className="flex items-center gap-3 mb-6">
        <div className="w-10 h-10 rounded-lg bg-orange-500/10 flex items-center justify-center">
          <GitBranch className="w-5 h-5 text-orange-600" strokeWidth={1.5} />
        </div>
        <div className="flex-1">
          <h3 className="font-medium text-foreground">GitLab</h3>
          <p className="text-xs text-muted-foreground">Commits 與 MR 追蹤</p>
        </div>
        {config?.gitlab_configured ? (
          <span className="flex items-center gap-1.5 text-xs text-sage">
            <CheckCircle2 className="w-3.5 h-3.5" strokeWidth={1.5} />
            已連接
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
            type="url"
            value={gitlabUrl}
            onChange={(e) => setGitlabUrl(e.target.value)}
            placeholder="https://gitlab.company.com"
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
              placeholder={config?.gitlab_configured ? '••••••••（已設定）' : '輸入 GitLab PAT'}
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
          <p className="text-xs text-muted-foreground mt-1">
            需要 read_api 權限
          </p>
        </div>

        <div className="flex items-center gap-3 pt-4 border-t border-border">
          <Button variant="outline" onClick={() => onSave(setMessage, refreshConfig)} disabled={saving || !gitlabUrl.trim()}>
            {saving ? <Loader2 className="w-4 h-4 animate-spin" /> : <Save className="w-4 h-4" />}
            儲存
          </Button>
          <Button variant="ghost" onClick={() => onTest(setMessage)} disabled={testing || !config?.gitlab_configured}>
            {testing ? <Loader2 className="w-4 h-4 animate-spin" /> : <Link2 className="w-4 h-4" />}
            測試連線
          </Button>
          {config?.gitlab_configured && (
            <Button variant="ghost" onClick={() => onRemoveConfig(setMessage, refreshConfig)} className="text-destructive hover:text-destructive">
              <Trash2 className="w-4 h-4" />
              移除設定
            </Button>
          )}
        </div>

        {/* GitLab Projects */}
        {config?.gitlab_configured && (
          <>
            <div className="pt-4 border-t border-border">
              <div className="flex items-center justify-between mb-4">
                <Label className="text-xs">已追蹤的專案</Label>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => onSync(setMessage)}
                  disabled={syncing || projects.length === 0}
                >
                  {syncing ? (
                    <Loader2 className="w-4 h-4 animate-spin" strokeWidth={1.5} />
                  ) : (
                    <RefreshCw className="w-4 h-4" strokeWidth={1.5} />
                  )}
                  同步全部
                </Button>
              </div>

              {projects.length > 0 ? (
                <div className="space-y-2">
                  {projects.map((project) => (
                    <div
                      key={project.id}
                      className="flex items-center justify-between p-3 bg-muted/30 rounded-lg"
                    >
                      <div className="flex-1 min-w-0">
                        <p className="text-sm font-medium text-foreground truncate">{project.name}</p>
                        <p className="text-xs text-muted-foreground truncate">{project.path_with_namespace}</p>
                        {project.last_synced && (
                          <p className="text-[10px] text-muted-foreground mt-1">
                            上次同步: {formatTimestamp(project.last_synced)}
                          </p>
                        )}
                      </div>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => onRemoveProject(project.id, setMessage)}
                        className="text-muted-foreground hover:text-destructive"
                      >
                        <Trash2 className="w-4 h-4" strokeWidth={1.5} />
                      </Button>
                    </div>
                  ))}
                </div>
              ) : (
                <p className="text-sm text-muted-foreground text-center py-4">
                  尚未追蹤任何專案
                </p>
              )}
            </div>

            {/* Search GitLab Projects */}
            <div className="pt-4 border-t border-border">
              <Label className="mb-2 block text-xs">新增專案</Label>
              <div className="flex items-center gap-2">
                <Input
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                  placeholder="搜尋 GitLab 專案..."
                  className="flex-1"
                  onKeyDown={(e) => e.key === 'Enter' && onSearch(setMessage)}
                />
                <Button
                  variant="outline"
                  onClick={() => onSearch(setMessage)}
                  disabled={searching || !search.trim()}
                >
                  {searching ? (
                    <Loader2 className="w-4 h-4 animate-spin" strokeWidth={1.5} />
                  ) : (
                    <RefreshCw className="w-4 h-4" strokeWidth={1.5} />
                  )}
                  搜尋
                </Button>
              </div>

              {searchResults.length > 0 && (
                <div className="mt-3 space-y-2 max-h-48 overflow-y-auto">
                  {searchResults.map((project) => (
                    <div
                      key={project.id}
                      className="flex items-center justify-between p-3 border border-dashed border-orange-300/50 bg-orange-50/30 rounded-lg"
                    >
                      <div className="flex-1 min-w-0">
                        <p className="text-sm font-medium text-foreground truncate">{project.name}</p>
                        <p className="text-xs text-muted-foreground truncate">{project.path_with_namespace}</p>
                      </div>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => onAddProject(project.id, setMessage)}
                        className="text-orange-600 border-orange-300 hover:bg-orange-100"
                      >
                        <Plus className="w-4 h-4" strokeWidth={1.5} />
                        新增
                      </Button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </>
        )}
      </div>
    </Card>
  )
}
