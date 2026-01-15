import { useEffect, useState } from 'react'
import {
  User,
  Link2,
  Bot,
  CheckCircle2,
  XCircle,
  Save,
  Terminal,
  Eye,
  EyeOff,
  Loader2,
  GitBranch,
  Settings,
  FolderGit2,
  Plus,
  Trash2,
  RefreshCw,
  Download,
  ChevronDown,
  ChevronRight,
  Sparkles,
  Cloud,
  LogOut,
} from 'lucide-react'
import { Card } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { api, ConfigResponse, SourcesResponse, ClaudeProject } from '@/lib/api'
import { useAuth } from '@/lib/auth'

type SettingsSection = 'profile' | 'account' | 'integrations' | 'preferences' | 'about'

export function SettingsPage() {
  const { user, logout, appStatus, token, isAuthenticated } = useAuth()
  const [activeSection, setActiveSection] = useState<SettingsSection>('profile')
  const [config, setConfig] = useState<ConfigResponse | null>(null)
  const [sources, setSources] = useState<SourcesResponse | null>(null)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [savingJira, setSavingJira] = useState(false)
  const [savingProfile, setSavingProfile] = useState(false)
  const [testingJira, setTestingJira] = useState(false)
  const [showToken, setShowToken] = useState(false)
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null)

  // Profile form state
  const [profileName, setProfileName] = useState('')
  const [profileEmail, setProfileEmail] = useState('')
  const [profileTitle, setProfileTitle] = useState('')
  const [profileEmployeeId, setProfileEmployeeId] = useState('')
  const [profileDepartment, setProfileDepartment] = useState('')

  // Work hours form state
  const [dailyHours, setDailyHours] = useState(8)
  const [normalizeHours, setNormalizeHours] = useState(true)

  // Jira form state
  const [jiraUrl, setJiraUrl] = useState('')
  const [jiraAuthType, setJiraAuthType] = useState<'pat' | 'basic'>('pat')
  const [jiraToken, setJiraToken] = useState('')
  const [jiraEmail, setJiraEmail] = useState('')
  const [tempoToken, setTempoToken] = useState('')

  // GitLab form state
  const [gitlabUrl, setGitlabUrl] = useState('')
  const [gitlabToken, setGitlabToken] = useState('')
  const [showGitlabToken, setShowGitlabToken] = useState(false)
  const [savingGitlab, setSavingGitlab] = useState(false)
  const [testingGitlab, setTestingGitlab] = useState(false)
  const [gitlabProjects, setGitlabProjects] = useState<Array<{
    id: string
    gitlab_project_id: number
    name: string
    path_with_namespace: string
    last_synced: string | null
  }>>([])
  const [gitlabSearchResults, setGitlabSearchResults] = useState<Array<{
    id: number
    name: string
    path_with_namespace: string
  }>>([])
  const [gitlabSearch, setGitlabSearch] = useState('')
  const [searchingGitlab, setSearchingGitlab] = useState(false)
  const [syncingGitlab, setSyncingGitlab] = useState(false)

  // Local Git repo form state
  const [newRepoPath, setNewRepoPath] = useState('')
  const [addingRepo, setAddingRepo] = useState(false)

  // Claude Code sessions state
  const [claudeProjects, setClaudeProjects] = useState<ClaudeProject[]>([])
  const [loadingClaude, setLoadingClaude] = useState(false)
  const [selectedProjects, setSelectedProjects] = useState<Set<string>>(() => {
    // Load from localStorage
    const saved = localStorage.getItem('recap-selected-claude-projects')
    return saved ? new Set(JSON.parse(saved)) : new Set()
  })
  const [expandedProjects, setExpandedProjects] = useState<Set<string>>(new Set())
  const [importingSessions, setImportingSessions] = useState(false)

  // LLM configuration state
  const [llmProvider, setLlmProvider] = useState('openai')
  const [llmModel, setLlmModel] = useState('gpt-4o-mini')
  const [llmApiKey, setLlmApiKey] = useState('')
  const [llmBaseUrl, setLlmBaseUrl] = useState('')
  const [showLlmKey, setShowLlmKey] = useState(false)
  const [savingLlm, setSavingLlm] = useState(false)

  useEffect(() => {
    // Only fetch data when authenticated
    if (!isAuthenticated || !token) {
      return
    }

    async function fetchData() {
      try {
        const [configData, sourcesData] = await Promise.all([
          api.getConfig(),
          api.getSources(),
        ])
        setConfig(configData)
        setSources(sourcesData)
        setDailyHours(configData.daily_work_hours)
        setNormalizeHours(configData.normalize_hours)
        setJiraUrl(configData.jira_url || '')
        setJiraAuthType(configData.auth_type === 'basic' ? 'basic' : 'pat')
        // LLM settings
        setLlmProvider(configData.llm_provider || 'openai')
        setLlmModel(configData.llm_model || 'gpt-4o-mini')
        setLlmBaseUrl(configData.llm_base_url || '')
        // GitLab settings
        setGitlabUrl(configData.gitlab_url || '')
      } catch (err) {
        console.error('Failed to fetch data:', err)
        setMessage({ type: 'error', text: err instanceof Error ? err.message : '載入設定失敗' })
      } finally {
        setLoading(false)
      }
    }
    fetchData()
  }, [isAuthenticated, token])

  useEffect(() => {
    if (user) {
      setProfileName(user.name || '')
      setProfileEmail(user.email || '')
      setProfileTitle(user.title || '')
      setProfileEmployeeId(user.employee_id || '')
      setProfileDepartment(user.department_id || '')
    }
  }, [user])

  // Save selected projects to localStorage
  useEffect(() => {
    localStorage.setItem('recap-selected-claude-projects', JSON.stringify(Array.from(selectedProjects)))
  }, [selectedProjects])

  // Auto-load Claude sessions and GitLab projects when viewing integrations
  useEffect(() => {
    // Only auto-load when authenticated and viewing integrations
    if (!isAuthenticated || !token || activeSection !== 'integrations') {
      return
    }
    if (claudeProjects.length === 0 && !loadingClaude) {
      loadClaudeSessions()
    }
    if (config?.gitlab_configured && gitlabProjects.length === 0) {
      loadGitlabProjects()
    }
  }, [activeSection, config?.gitlab_configured, isAuthenticated, token])

  const handleSaveProfile = async () => {
    setSavingProfile(true)
    setMessage(null)
    try {
      await api.updateProfile({
        name: profileName,
        email: profileEmail || undefined,
        title: profileTitle,
        employee_id: profileEmployeeId || undefined,
        department_id: profileDepartment || undefined,
      })
      setMessage({ type: 'success', text: '個人資料已更新' })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '更新失敗' })
    } finally {
      setSavingProfile(false)
    }
  }

  const handleSavePreferences = async () => {
    setSaving(true)
    setMessage(null)
    try {
      await api.updateConfig({
        daily_work_hours: dailyHours,
        normalize_hours: normalizeHours,
      } as Partial<ConfigResponse>)
      setMessage({ type: 'success', text: '偏好設定已儲存' })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '儲存失敗' })
    } finally {
      setSaving(false)
    }
  }

  const handleSaveLlm = async () => {
    setSavingLlm(true)
    setMessage(null)
    try {
      await api.updateLlmConfig({
        provider: llmProvider,
        model: llmModel,
        api_key: llmApiKey || undefined,
        base_url: llmBaseUrl || undefined,
      })
      const updated = await api.getConfig()
      setConfig(updated)
      setLlmApiKey('') // Clear the key field after saving
      setMessage({ type: 'success', text: 'LLM 設定已儲存' })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '儲存失敗' })
    } finally {
      setSavingLlm(false)
    }
  }

  const handleSaveJira = async () => {
    setSavingJira(true)
    setMessage(null)
    try {
      const payload: {
        jira_url?: string
        jira_pat?: string
        jira_email?: string
        jira_api_token?: string
        auth_type?: string
        tempo_api_token?: string
      } = {
        jira_url: jiraUrl,
        auth_type: jiraAuthType,
      }

      if (jiraToken) {
        if (jiraAuthType === 'pat') {
          payload.jira_pat = jiraToken
        } else {
          payload.jira_api_token = jiraToken
        }
      }

      if (jiraAuthType === 'basic' && jiraEmail) {
        payload.jira_email = jiraEmail
      }

      if (tempoToken) {
        payload.tempo_api_token = tempoToken
      }

      await api.updateJiraConfig(payload)
      const updated = await api.getConfig()
      setConfig(updated)
      setJiraToken('')
      setTempoToken('')
      setMessage({ type: 'success', text: 'Jira 設定已儲存' })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '儲存失敗' })
    } finally {
      setSavingJira(false)
    }
  }

  const handleTestJira = async () => {
    setTestingJira(true)
    setMessage(null)
    try {
      const result = await api.testJira()
      setMessage({ type: 'success', text: result.message })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '連線失敗' })
    } finally {
      setTestingJira(false)
    }
  }

  const handleSaveGitlab = async () => {
    if (!gitlabUrl.trim()) return
    setSavingGitlab(true)
    setMessage(null)
    try {
      await api.configureGitLab(gitlabUrl.trim(), gitlabToken)
      const updated = await api.getConfig()
      setConfig(updated)
      setGitlabToken('')
      setMessage({ type: 'success', text: 'GitLab 設定已儲存' })
      // Reload projects after configuration
      loadGitlabProjects()
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '儲存失敗' })
    } finally {
      setSavingGitlab(false)
    }
  }

  const handleTestGitlab = async () => {
    setTestingGitlab(true)
    setMessage(null)
    try {
      // Try to search projects as a connection test
      await api.getGitLabRemoteProjects('', 1, 1)
      setMessage({ type: 'success', text: 'GitLab 連線成功' })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '連線失敗' })
    } finally {
      setTestingGitlab(false)
    }
  }

  const loadGitlabProjects = async () => {
    try {
      const projects = await api.getGitLabTrackedProjects()
      setGitlabProjects(projects)
    } catch (err) {
      console.error('Failed to load GitLab projects:', err)
    }
  }

  const handleSearchGitlab = async () => {
    if (!gitlabSearch.trim()) return
    setSearchingGitlab(true)
    try {
      const results = await api.getGitLabRemoteProjects(gitlabSearch)
      setGitlabSearchResults(results)
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '搜尋失敗' })
    } finally {
      setSearchingGitlab(false)
    }
  }

  const handleAddGitlabProject = async (projectId: number) => {
    try {
      await api.addGitLabProject(projectId)
      setMessage({ type: 'success', text: '已新增 GitLab 專案' })
      loadGitlabProjects()
      // Remove from search results
      setGitlabSearchResults(prev => prev.filter(p => p.id !== projectId))
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '新增失敗' })
    }
  }

  const handleRemoveGitlabProject = async (id: string) => {
    try {
      await api.removeGitLabProject(id)
      setMessage({ type: 'success', text: '已移除 GitLab 專案' })
      loadGitlabProjects()
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '移除失敗' })
    }
  }

  const handleSyncGitlab = async () => {
    setSyncingGitlab(true)
    setMessage(null)
    try {
      const results = await api.syncAllGitLabProjects()
      const total = results.reduce((sum, r) => sum + r.work_items_created, 0)
      setMessage({ type: 'success', text: `已同步 ${total} 個工作項目` })
      loadGitlabProjects()
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '同步失敗' })
    } finally {
      setSyncingGitlab(false)
    }
  }

  const handleRemoveGitlabConfig = async () => {
    try {
      await api.removeGitLabConfig()
      const updated = await api.getConfig()
      setConfig(updated)
      setGitlabProjects([])
      setGitlabSearchResults([])
      setMessage({ type: 'success', text: 'GitLab 設定已移除' })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '移除失敗' })
    }
  }

  const handleAddRepo = async () => {
    if (!newRepoPath.trim()) return
    setAddingRepo(true)
    setMessage(null)
    try {
      await api.addGitRepo(newRepoPath.trim())
      const updated = await api.getSources()
      setSources(updated)
      setNewRepoPath('')
      setMessage({ type: 'success', text: '已新增 Git 倉庫' })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '新增失敗' })
    } finally {
      setAddingRepo(false)
    }
  }

  const handleRemoveRepo = async (repoId: string) => {
    setMessage(null)
    try {
      await api.removeGitRepo(repoId)
      const updated = await api.getSources()
      setSources(updated)
      setMessage({ type: 'success', text: '已移除 Git 倉庫' })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '移除失敗' })
    }
  }

  const loadClaudeSessions = async () => {
    setLoadingClaude(true)
    setMessage(null)
    try {
      const projects = await api.getClaudeSessions()
      setClaudeProjects(projects)
      // Expand the first project by default
      if (projects.length > 0) {
        setExpandedProjects(new Set([projects[0].path]))
      }

      // Auto-add detected Git repos from Claude Code projects
      if (projects.length > 0 && sources) {
        const existingPaths = sources.git_repos?.map(r => r.path) || []
        const newProjects = projects.filter(p => !existingPaths.includes(p.path))

        if (newProjects.length > 0) {
          // Add all new repos in parallel
          const addPromises = newProjects.map(p =>
            api.addGitRepo(p.path).catch(() => null) // Ignore individual failures
          )
          await Promise.all(addPromises)

          // Refresh sources to show the added repos
          const updated = await api.getSources()
          setSources(updated)
          setMessage({
            type: 'success',
            text: `已自動新增 ${newProjects.length} 個 Git 倉庫`
          })
        }
      }
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '載入失敗' })
    } finally {
      setLoadingClaude(false)
    }
  }

  const toggleExpandProject = (path: string) => {
    setExpandedProjects(prev => {
      const next = new Set(prev)
      if (next.has(path)) {
        next.delete(path)
      } else {
        next.add(path)
      }
      return next
    })
  }

  const toggleProjectSelection = (path: string) => {
    setSelectedProjects(prev => {
      const next = new Set(prev)
      if (next.has(path)) {
        next.delete(path)
      } else {
        next.add(path)
      }
      return next
    })
  }

  const selectAllProjects = () => {
    setSelectedProjects(new Set(claudeProjects.map(p => p.path)))
  }

  const clearSelection = () => {
    setSelectedProjects(new Set())
  }

  // Calculate selected session count from selected projects
  const selectedSessionCount = claudeProjects
    .filter(p => selectedProjects.has(p.path))
    .reduce((acc, p) => acc + p.sessions.length, 0)

  const handleImportSessions = async () => {
    if (selectedProjects.size === 0) return
    setImportingSessions(true)
    setMessage(null)
    try {
      // Get all session IDs from selected projects
      const sessionIds = claudeProjects
        .filter(p => selectedProjects.has(p.path))
        .flatMap(p => p.sessions.map(s => s.session_id))

      const result = await api.importClaudeSessions(sessionIds)
      setMessage({
        type: 'success',
        text: `已匯入 ${result.imported} 個 session，建立 ${result.work_items_created} 個工作項目`,
      })
      // Keep selection for future imports (don't clear)
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '匯入失敗' })
    } finally {
      setImportingSessions(false)
    }
  }

  const formatFileSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
  }

  const formatTimestamp = (ts?: string): string => {
    if (!ts) return '-'
    const date = new Date(ts)
    return date.toLocaleString('zh-TW', {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    })
  }

  const sections = [
    { id: 'profile' as const, label: '個人資料', icon: User },
    { id: 'account' as const, label: '帳號', icon: Cloud },
    { id: 'integrations' as const, label: '整合服務', icon: Link2 },
    { id: 'preferences' as const, label: '偏好設定', icon: Settings },
    { id: 'about' as const, label: '關於', icon: Bot },
  ]

  if (loading) {
    return (
      <div className="flex items-center justify-center h-96">
        <div className="w-6 h-6 border border-border border-t-charcoal/60 rounded-full animate-spin" />
      </div>
    )
  }

  return (
    <div className="flex gap-8">
      {/* Sidebar Navigation */}
      <aside className="w-48 shrink-0">
        <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-4">
          設定
        </p>
        <nav className="space-y-1">
          {sections.map((section) => (
            <button
              key={section.id}
              onClick={() => {
                setActiveSection(section.id)
                setMessage(null)
              }}
              className={`w-full flex items-center gap-3 px-3 py-2 text-sm transition-colors ${
                activeSection === section.id
                  ? 'text-foreground bg-foreground/5 border-l-2 border-foreground -ml-px'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
            >
              <section.icon className="w-4 h-4" strokeWidth={1.5} />
              {section.label}
            </button>
          ))}
        </nav>
      </aside>

      {/* Content */}
      <main className="flex-1 max-w-2xl">
        {/* Message */}
        {message && (
          <div className={`mb-6 p-3 text-sm ${
            message.type === 'success'
              ? 'bg-sage/10 text-sage border-l-2 border-sage'
              : 'bg-destructive/10 text-destructive border-l-2 border-destructive'
          }`}>
            {message.text}
          </div>
        )}

        {/* Profile Section */}
        {activeSection === 'profile' && (
          <section className="animate-fade-up opacity-0 delay-1">
            <h2 className="font-display text-2xl text-foreground mb-6">個人資料</h2>

            <Card className="p-6">
              <div className="space-y-6">
                <div>
                  <Label htmlFor="profile-name" className="mb-2 block">名稱</Label>
                  <Input
                    id="profile-name"
                    value={profileName}
                    onChange={(e) => setProfileName(e.target.value)}
                    placeholder="您的名稱"
                  />
                </div>

                <div>
                  <Label htmlFor="profile-email" className="mb-2 block">
                    Email <span className="text-muted-foreground text-xs">(選填)</span>
                  </Label>
                  <Input
                    id="profile-email"
                    type="text"
                    value={profileEmail}
                    onChange={(e) => setProfileEmail(e.target.value)}
                    placeholder="your@email.com"
                  />
                  <p className="text-xs text-muted-foreground mt-1">用於通知和報告寄送</p>
                </div>

                <div>
                  <Label htmlFor="profile-title" className="mb-2 block">職稱</Label>
                  <Input
                    id="profile-title"
                    value={profileTitle}
                    onChange={(e) => setProfileTitle(e.target.value)}
                    placeholder="例如：軟體工程師"
                  />
                </div>

                <div>
                  <Label htmlFor="profile-employee-id" className="mb-2 block">
                    員工編號 <span className="text-muted-foreground text-xs">(選填)</span>
                  </Label>
                  <Input
                    id="profile-employee-id"
                    value={profileEmployeeId}
                    onChange={(e) => setProfileEmployeeId(e.target.value)}
                    placeholder="例如：EMP001"
                  />
                </div>

                <div>
                  <Label htmlFor="profile-department" className="mb-2 block">
                    部門 <span className="text-muted-foreground text-xs">(選填)</span>
                  </Label>
                  <Input
                    id="profile-department"
                    value={profileDepartment}
                    onChange={(e) => setProfileDepartment(e.target.value)}
                    placeholder="例如：研發部"
                  />
                </div>

                <div className="pt-4 border-t border-border">
                  <Button onClick={handleSaveProfile} disabled={savingProfile}>
                    {savingProfile ? (
                      <Loader2 className="w-4 h-4 animate-spin" strokeWidth={1.5} />
                    ) : (
                      <Save className="w-4 h-4" strokeWidth={1.5} />
                    )}
                    {savingProfile ? '儲存中...' : '儲存'}
                  </Button>
                </div>
              </div>
            </Card>
          </section>
        )}

        {/* Account Section */}
        {activeSection === 'account' && (
          <section className="animate-fade-up opacity-0 delay-1">
            <h2 className="font-display text-2xl text-foreground mb-6">帳號</h2>

            <Card className="p-6">
              <div className="space-y-6">
                {/* Current account status */}
                <div className="flex items-center gap-4">
                  <div className="w-12 h-12 rounded-full bg-foreground/10 flex items-center justify-center">
                    <User className="w-6 h-6 text-foreground" strokeWidth={1.5} />
                  </div>
                  <div className="flex-1">
                    <p className="text-sm font-medium text-foreground">{user?.name || '本地使用者'}</p>
                    <p className="text-xs text-muted-foreground">{user?.email || '本地模式'}</p>
                  </div>
                  {appStatus?.local_mode && (
                    <span className="px-2 py-1 text-xs bg-amber-100 text-amber-700 rounded">
                      本地模式
                    </span>
                  )}
                </div>

                <div className="pt-4 border-t border-border">
                  <p className="text-sm text-foreground mb-2">本地優先模式</p>
                  <p className="text-xs text-muted-foreground leading-relaxed mb-4">
                    目前 Recap 以本地模式運行，所有資料儲存在您的裝置上。
                    未來將支援雲端同步功能，讓您可以在多台裝置間同步工作記錄。
                  </p>
                </div>

                {/* Future cloud sync placeholder */}
                <div className="pt-4 border-t border-border">
                  <div className="flex items-center gap-3 text-muted-foreground">
                    <Cloud className="w-5 h-5" strokeWidth={1.5} />
                    <div>
                      <p className="text-sm">雲端同步</p>
                      <p className="text-xs">即將推出</p>
                    </div>
                  </div>
                </div>

                {/* Logout button */}
                <div className="pt-4 border-t border-border">
                  <Button variant="outline" onClick={logout} className="text-destructive hover:text-destructive">
                    <LogOut className="w-4 h-4" strokeWidth={1.5} />
                    登出
                  </Button>
                  <p className="text-xs text-muted-foreground mt-2">
                    登出後將清除本地登入狀態，重新啟動 App 會自動登入。
                  </p>
                </div>
              </div>
            </Card>
          </section>
        )}

        {/* Integrations Section */}
        {activeSection === 'integrations' && (
          <section className="space-y-8 animate-fade-up opacity-0 delay-1">
            <h2 className="font-display text-2xl text-foreground">整合服務</h2>

            {/* Local Git Repos */}
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
                            onClick={() => handleRemoveRepo(repo.id)}
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
                    {claudeProjects
                      .filter(project => {
                        // Filter out projects already in git_repos
                        const existingPaths = sources?.git_repos?.map(r => r.path) || []
                        // Backend returns correct path from session cwd
                        return !existingPaths.some(p => p === project.path)
                      })
                      .slice(0, 5)
                      .map((project) => (
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
                              onClick={async () => {
                                setMessage(null)
                                try {
                                  await api.addGitRepo(project.path)
                                  const updated = await api.getSources()
                                  setSources(updated)
                                  setMessage({ type: 'success', text: `已新增 ${project.name}` })
                                } catch (err) {
                                  setMessage({ type: 'error', text: err instanceof Error ? err.message : '新增失敗' })
                                }
                              }}
                              className="text-purple-600 border-purple-300 hover:bg-purple-100"
                            >
                              <Plus className="w-4 h-4" strokeWidth={1.5} />
                              新增
                            </Button>
                          </div>
                        )
                      )}
                    {claudeProjects.filter(project => {
                      const existingPaths = sources?.git_repos?.map(r => r.path) || []
                      return !existingPaths.some(p => p === project.path)
                    }).length === 0 && (
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
                    onKeyDown={(e) => e.key === 'Enter' && handleAddRepo()}
                  />
                  <Button
                    variant="outline"
                    onClick={handleAddRepo}
                    disabled={addingRepo || !newRepoPath.trim()}
                  >
                    {addingRepo ? (
                      <Loader2 className="w-4 h-4 animate-spin" strokeWidth={1.5} />
                    ) : (
                      <Plus className="w-4 h-4" strokeWidth={1.5} />
                    )}
                    新增
                  </Button>
                </div>
              </div>
            </Card>

            {/* Claude Code */}
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
                  onClick={loadClaudeSessions}
                  disabled={loadingClaude}
                >
                  {loadingClaude ? (
                    <Loader2 className="w-4 h-4 animate-spin" strokeWidth={1.5} />
                  ) : (
                    <RefreshCw className="w-4 h-4" strokeWidth={1.5} />
                  )}
                  {claudeProjects.length > 0 ? '重新載入' : '載入 Sessions'}
                </Button>
              </div>

              {/* Sessions List */}
              {claudeProjects.length > 0 ? (
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
                          onClick={selectAllProjects}
                          disabled={selectedProjects.size === claudeProjects.length}
                          className="h-7 px-2 text-xs"
                        >
                          全選
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={clearSelection}
                          disabled={selectedProjects.size === 0}
                          className="h-7 px-2 text-xs"
                        >
                          清除
                        </Button>
                      </div>
                    </div>
                    <Button
                      size="sm"
                      onClick={handleImportSessions}
                      disabled={selectedProjects.size === 0 || importingSessions}
                    >
                      {importingSessions ? (
                        <Loader2 className="w-4 h-4 animate-spin" strokeWidth={1.5} />
                      ) : (
                        <Download className="w-4 h-4" strokeWidth={1.5} />
                      )}
                      匯入為工作項目
                    </Button>
                  </div>

                  {/* Projects */}
                  <div className="space-y-2 max-h-96 overflow-y-auto">
                    {claudeProjects.map((project) => {
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
                              onChange={() => toggleProjectSelection(project.path)}
                              className="w-4 h-4 accent-purple-600"
                            />
                            <button
                              onClick={() => toggleExpandProject(project.path)}
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

            {/* Jira / Tempo */}
            <Card className="p-6">
              <div className="flex items-center gap-3 mb-6">
                <div className="w-10 h-10 rounded-lg bg-blue-500/10 flex items-center justify-center">
                  <Link2 className="w-5 h-5 text-blue-600" strokeWidth={1.5} />
                </div>
                <div className="flex-1">
                  <h3 className="font-medium text-foreground">Jira / Tempo</h3>
                  <p className="text-xs text-muted-foreground">工時記錄與同步</p>
                </div>
                {config?.jira_configured ? (
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
                  <Label htmlFor="jira-url" className="mb-2 block text-xs">Jira URL</Label>
                  <Input
                    id="jira-url"
                    type="url"
                    value={jiraUrl}
                    onChange={(e) => setJiraUrl(e.target.value)}
                    placeholder="https://your-company.atlassian.net"
                  />
                </div>

                <div>
                  <Label className="mb-2 block text-xs">認證方式</Label>
                  <div className="flex items-center gap-4">
                    <label className="flex items-center gap-2 cursor-pointer">
                      <input
                        type="radio"
                        name="auth-type"
                        checked={jiraAuthType === 'pat'}
                        onChange={() => setJiraAuthType('pat')}
                        className="w-4 h-4 accent-foreground"
                      />
                      <span className="text-sm">PAT</span>
                    </label>
                    <label className="flex items-center gap-2 cursor-pointer">
                      <input
                        type="radio"
                        name="auth-type"
                        checked={jiraAuthType === 'basic'}
                        onChange={() => setJiraAuthType('basic')}
                        className="w-4 h-4 accent-foreground"
                      />
                      <span className="text-sm">Basic Auth</span>
                    </label>
                  </div>
                </div>

                {jiraAuthType === 'basic' && (
                  <div>
                    <Label htmlFor="jira-email" className="mb-2 block text-xs">Email</Label>
                    <Input
                      id="jira-email"
                      type="email"
                      value={jiraEmail}
                      onChange={(e) => setJiraEmail(e.target.value)}
                      placeholder="your-email@company.com"
                    />
                  </div>
                )}

                <div>
                  <Label htmlFor="jira-token" className="mb-2 block text-xs">
                    {jiraAuthType === 'pat' ? 'Personal Access Token' : 'API Token'}
                  </Label>
                  <div className="relative">
                    <Input
                      id="jira-token"
                      type={showToken ? 'text' : 'password'}
                      value={jiraToken}
                      onChange={(e) => setJiraToken(e.target.value)}
                      placeholder={config?.jira_configured ? '••••••••（已設定）' : '輸入 Token'}
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

                <div>
                  <Label htmlFor="tempo-token" className="mb-2 block text-xs">
                    Tempo API Token <span className="text-muted-foreground">(選用)</span>
                  </Label>
                  <Input
                    id="tempo-token"
                    type="password"
                    value={tempoToken}
                    onChange={(e) => setTempoToken(e.target.value)}
                    placeholder={config?.tempo_configured ? '••••••••（已設定）' : '留空使用 Jira worklog'}
                  />
                </div>

                <div className="flex items-center gap-3 pt-4 border-t border-border">
                  <Button variant="outline" onClick={handleSaveJira} disabled={savingJira}>
                    {savingJira ? <Loader2 className="w-4 h-4 animate-spin" /> : <Save className="w-4 h-4" />}
                    儲存
                  </Button>
                  <Button variant="ghost" onClick={handleTestJira} disabled={testingJira || !config?.jira_configured}>
                    {testingJira ? <Loader2 className="w-4 h-4 animate-spin" /> : <Link2 className="w-4 h-4" />}
                    測試連線
                  </Button>
                </div>
              </div>
            </Card>

            {/* GitLab */}
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
                      type={showGitlabToken ? 'text' : 'password'}
                      value={gitlabToken}
                      onChange={(e) => setGitlabToken(e.target.value)}
                      placeholder={config?.gitlab_configured ? '••••••••（已設定）' : '輸入 GitLab PAT'}
                      className="pr-10"
                    />
                    <button
                      type="button"
                      onClick={() => setShowGitlabToken(!showGitlabToken)}
                      className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                    >
                      {showGitlabToken ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                    </button>
                  </div>
                  <p className="text-xs text-muted-foreground mt-1">
                    需要 read_api 權限
                  </p>
                </div>

                <div className="flex items-center gap-3 pt-4 border-t border-border">
                  <Button variant="outline" onClick={handleSaveGitlab} disabled={savingGitlab || !gitlabUrl.trim()}>
                    {savingGitlab ? <Loader2 className="w-4 h-4 animate-spin" /> : <Save className="w-4 h-4" />}
                    儲存
                  </Button>
                  <Button variant="ghost" onClick={handleTestGitlab} disabled={testingGitlab || !config?.gitlab_configured}>
                    {testingGitlab ? <Loader2 className="w-4 h-4 animate-spin" /> : <Link2 className="w-4 h-4" />}
                    測試連線
                  </Button>
                  {config?.gitlab_configured && (
                    <Button variant="ghost" onClick={handleRemoveGitlabConfig} className="text-destructive hover:text-destructive">
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
                          onClick={handleSyncGitlab}
                          disabled={syncingGitlab || gitlabProjects.length === 0}
                        >
                          {syncingGitlab ? (
                            <Loader2 className="w-4 h-4 animate-spin" strokeWidth={1.5} />
                          ) : (
                            <RefreshCw className="w-4 h-4" strokeWidth={1.5} />
                          )}
                          同步全部
                        </Button>
                      </div>

                      {gitlabProjects.length > 0 ? (
                        <div className="space-y-2">
                          {gitlabProjects.map((project) => (
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
                                onClick={() => handleRemoveGitlabProject(project.id)}
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
                          value={gitlabSearch}
                          onChange={(e) => setGitlabSearch(e.target.value)}
                          placeholder="搜尋 GitLab 專案..."
                          className="flex-1"
                          onKeyDown={(e) => e.key === 'Enter' && handleSearchGitlab()}
                        />
                        <Button
                          variant="outline"
                          onClick={handleSearchGitlab}
                          disabled={searchingGitlab || !gitlabSearch.trim()}
                        >
                          {searchingGitlab ? (
                            <Loader2 className="w-4 h-4 animate-spin" strokeWidth={1.5} />
                          ) : (
                            <RefreshCw className="w-4 h-4" strokeWidth={1.5} />
                          )}
                          搜尋
                        </Button>
                      </div>

                      {gitlabSearchResults.length > 0 && (
                        <div className="mt-3 space-y-2 max-h-48 overflow-y-auto">
                          {gitlabSearchResults.map((project) => (
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
                                onClick={() => handleAddGitlabProject(project.id)}
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
          </section>
        )}

        {/* Preferences Section */}
        {activeSection === 'preferences' && (
          <section className="animate-fade-up opacity-0 delay-1">
            <h2 className="font-display text-2xl text-foreground mb-6">偏好設定</h2>

            <Card className="p-6">
              <div className="space-y-6">
                <div>
                  <Label htmlFor="daily-hours" className="mb-2 block">每日標準工時</Label>
                  <div className="flex items-center gap-3">
                    <Input
                      id="daily-hours"
                      type="number"
                      value={dailyHours}
                      onChange={(e) => setDailyHours(Number(e.target.value))}
                      min={1}
                      max={24}
                      step={0.5}
                      className="w-24"
                    />
                    <span className="text-sm text-muted-foreground">小時</span>
                  </div>
                </div>

                <div>
                  <label className="flex items-center gap-3 cursor-pointer">
                    <div className="relative">
                      <input
                        type="checkbox"
                        checked={normalizeHours}
                        onChange={(e) => setNormalizeHours(e.target.checked)}
                        className="sr-only peer"
                      />
                      <div className="w-10 h-5 bg-foreground/15 peer-checked:bg-foreground transition-colors" />
                      <div className="absolute top-0.5 left-0.5 w-4 h-4 bg-white transition-transform peer-checked:translate-x-5" />
                    </div>
                    <div>
                      <span className="text-sm text-foreground">自動正規化工時</span>
                      <p className="text-xs text-muted-foreground">將每日工時調整為標準工時</p>
                    </div>
                  </label>
                </div>

                <div className="pt-4 border-t border-border">
                  <Button onClick={handleSavePreferences} disabled={saving}>
                    {saving ? <Loader2 className="w-4 h-4 animate-spin" /> : <Save className="w-4 h-4" />}
                    {saving ? '儲存中...' : '儲存'}
                  </Button>
                </div>
              </div>
            </Card>

            {/* LLM Settings */}
            <Card className="p-6 mt-6">
              <div className="flex items-center gap-3 mb-6">
                <div className="w-10 h-10 rounded-lg bg-amber-500/10 flex items-center justify-center">
                  <Sparkles className="w-5 h-5 text-amber-600" strokeWidth={1.5} />
                </div>
                <div className="flex-1">
                  <h3 className="font-medium text-foreground">LLM 設定</h3>
                  <p className="text-xs text-muted-foreground">設定 AI 模型用於分析和建議</p>
                </div>
                {config?.llm_configured ? (
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
                {/* Provider Selection */}
                <div>
                  <Label className="mb-2 block text-xs">提供者</Label>
                  <div className="grid grid-cols-2 gap-2">
                    {[
                      { id: 'openai', label: 'OpenAI', desc: 'GPT-4o, GPT-4 等' },
                      { id: 'anthropic', label: 'Anthropic', desc: 'Claude 系列' },
                      { id: 'ollama', label: 'Ollama', desc: '本地部署' },
                      { id: 'openai-compatible', label: '相容 API', desc: '自架 OpenAI 相容服務' },
                    ].map((provider) => (
                      <button
                        key={provider.id}
                        onClick={() => {
                          setLlmProvider(provider.id)
                          // Set default model for provider
                          if (provider.id === 'openai') setLlmModel('gpt-4o-mini')
                          else if (provider.id === 'anthropic') setLlmModel('claude-3-5-sonnet-20241022')
                          else if (provider.id === 'ollama') setLlmModel('llama3.2')
                          else setLlmModel('')
                          // Set default base URL for Ollama
                          if (provider.id === 'ollama') setLlmBaseUrl('http://localhost:11434')
                          else if (provider.id !== 'openai-compatible') setLlmBaseUrl('')
                        }}
                        className={`p-3 text-left border rounded-lg transition-colors ${
                          llmProvider === provider.id
                            ? 'border-foreground bg-foreground/5'
                            : 'border-border hover:border-foreground/30'
                        }`}
                      >
                        <p className="text-sm font-medium">{provider.label}</p>
                        <p className="text-xs text-muted-foreground">{provider.desc}</p>
                      </button>
                    ))}
                  </div>
                </div>

                {/* Model */}
                <div>
                  <Label htmlFor="llm-model" className="mb-2 block text-xs">模型名稱</Label>
                  <Input
                    id="llm-model"
                    value={llmModel}
                    onChange={(e) => setLlmModel(e.target.value)}
                    placeholder={
                      llmProvider === 'openai' ? 'gpt-4o-mini' :
                      llmProvider === 'anthropic' ? 'claude-3-5-sonnet-20241022' :
                      llmProvider === 'ollama' ? 'llama3.2' : '輸入模型名稱'
                    }
                  />
                </div>

                {/* API Key (not needed for Ollama) */}
                {llmProvider !== 'ollama' && (
                  <div>
                    <Label htmlFor="llm-api-key" className="mb-2 block text-xs">API Key</Label>
                    <div className="relative">
                      <Input
                        id="llm-api-key"
                        type={showLlmKey ? 'text' : 'password'}
                        value={llmApiKey}
                        onChange={(e) => setLlmApiKey(e.target.value)}
                        placeholder={config?.llm_configured ? '••••••••（已設定）' : '輸入 API Key'}
                        className="pr-10"
                      />
                      <button
                        type="button"
                        onClick={() => setShowLlmKey(!showLlmKey)}
                        className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                      >
                        {showLlmKey ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                      </button>
                    </div>
                  </div>
                )}

                {/* Base URL (for Ollama and OpenAI-compatible) */}
                {(llmProvider === 'ollama' || llmProvider === 'openai-compatible') && (
                  <div>
                    <Label htmlFor="llm-base-url" className="mb-2 block text-xs">API URL</Label>
                    <Input
                      id="llm-base-url"
                      type="url"
                      value={llmBaseUrl}
                      onChange={(e) => setLlmBaseUrl(e.target.value)}
                      placeholder={llmProvider === 'ollama' ? 'http://localhost:11434' : 'https://your-api.example.com/v1'}
                    />
                    <p className="text-xs text-muted-foreground mt-1">
                      {llmProvider === 'ollama' ? 'Ollama 服務地址' : 'OpenAI 相容的 API 端點'}
                    </p>
                  </div>
                )}

                <div className="pt-4 border-t border-border">
                  <Button onClick={handleSaveLlm} disabled={savingLlm}>
                    {savingLlm ? <Loader2 className="w-4 h-4 animate-spin" /> : <Save className="w-4 h-4" />}
                    儲存 LLM 設定
                  </Button>
                </div>
              </div>
            </Card>
          </section>
        )}

        {/* About Section */}
        {activeSection === 'about' && (
          <section className="animate-fade-up opacity-0 delay-1">
            <h2 className="font-display text-2xl text-foreground mb-6">關於</h2>

            <Card className="p-6">
              <div className="space-y-6">
                <div className="flex items-center gap-4">
                  <div className="w-16 h-16 rounded-2xl bg-[#1F1D1A] flex flex-col items-center justify-center p-2">
                    <span className="text-[#F9F7F2] text-sm font-display font-medium tracking-tight">Recap</span>
                    <div className="w-10 h-0.5 bg-[#B09872] mt-0.5 rounded-full opacity-70" />
                  </div>
                  <div>
                    <h3 className="font-display text-xl text-foreground">Recap</h3>
                    <p className="text-sm text-muted-foreground">v2.1.0</p>
                  </div>
                </div>

                <div className="pt-4 border-t border-border">
                  <p className="text-sm text-foreground mb-2">自動回顧你的工作</p>
                  <p className="text-xs text-muted-foreground leading-relaxed">
                    Recap 自動追蹤您從 GitLab、Claude Code 等來源的工作記錄，
                    協助您生成報告並同步到 Jira Tempo。
                  </p>
                </div>

                <div className="pt-4 border-t border-border space-y-2">
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-muted-foreground">API 狀態</span>
                    <span className="flex items-center gap-1.5 text-sage">
                      <CheckCircle2 className="w-3.5 h-3.5" />
                      運行中
                    </span>
                  </div>
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-muted-foreground">資料庫</span>
                    <span className="text-foreground">SQLite</span>
                  </div>
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-muted-foreground">框架</span>
                    <span className="text-foreground">Tauri v2</span>
                  </div>
                </div>
              </div>
            </Card>
          </section>
        )}
      </main>
    </div>
  )
}
