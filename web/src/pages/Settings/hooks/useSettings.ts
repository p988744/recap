import { useEffect, useState } from 'react'
import { config as configService, sources as sourcesService, auth, gitlab, tempo, claude } from '@/services'
import type { ConfigResponse, SourcesResponse, ClaudeProject, GitLabProject, GitLabProjectInfo } from '@/types'
import { useAuth } from '@/lib/auth'

export type SettingsSection = 'profile' | 'account' | 'integrations' | 'preferences' | 'about'

export interface SettingsMessage {
  type: 'success' | 'error'
  text: string
}

export function useSettings() {
  const { user, logout, appStatus, token, isAuthenticated } = useAuth()
  const [activeSection, setActiveSection] = useState<SettingsSection>('profile')
  const [config, setConfig] = useState<ConfigResponse | null>(null)
  const [sources, setSources] = useState<SourcesResponse | null>(null)
  const [loading, setLoading] = useState(true)
  const [message, setMessage] = useState<SettingsMessage | null>(null)

  useEffect(() => {
    if (!isAuthenticated || !token) return

    async function fetchData() {
      try {
        const [configData, sourcesData] = await Promise.all([
          configService.getConfig(),
          sourcesService.getSources(),
        ])
        setConfig(configData)
        setSources(sourcesData)
      } catch (err) {
        console.error('Failed to fetch data:', err)
        setMessage({ type: 'error', text: err instanceof Error ? err.message : '載入設定失敗' })
      } finally {
        setLoading(false)
      }
    }
    fetchData()
  }, [isAuthenticated, token])

  const refreshConfig = async () => {
    const updated = await configService.getConfig()
    setConfig(updated)
    return updated
  }

  const refreshSources = async () => {
    const updated = await sourcesService.getSources()
    setSources(updated)
    return updated
  }

  return {
    // Auth
    user,
    logout,
    appStatus,
    isAuthenticated,
    // State
    activeSection,
    setActiveSection,
    config,
    setConfig,
    sources,
    setSources,
    loading,
    message,
    setMessage,
    // Refresh helpers
    refreshConfig,
    refreshSources,
  }
}

export function useProfileForm(user: ReturnType<typeof useAuth>['user']) {
  const [profileName, setProfileName] = useState('')
  const [profileEmail, setProfileEmail] = useState('')
  const [profileTitle, setProfileTitle] = useState('')
  const [profileEmployeeId, setProfileEmployeeId] = useState('')
  const [profileDepartment, setProfileDepartment] = useState('')
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    if (user) {
      setProfileName(user.name || '')
      setProfileEmail(user.email || '')
      setProfileTitle(user.title || '')
      setProfileEmployeeId(user.employee_id || '')
      setProfileDepartment(user.department_id || '')
    }
  }, [user])

  const handleSave = async (setMessage: (msg: SettingsMessage | null) => void) => {
    setSaving(true)
    setMessage(null)
    try {
      await auth.updateProfile({
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
      setSaving(false)
    }
  }

  return {
    profileName,
    setProfileName,
    profileEmail,
    setProfileEmail,
    profileTitle,
    setProfileTitle,
    profileEmployeeId,
    setProfileEmployeeId,
    profileDepartment,
    setProfileDepartment,
    saving,
    handleSave,
  }
}

export function usePreferencesForm(config: ConfigResponse | null) {
  const [dailyHours, setDailyHours] = useState(8)
  const [normalizeHours, setNormalizeHours] = useState(true)
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    if (config) {
      setDailyHours(config.daily_work_hours)
      setNormalizeHours(config.normalize_hours)
    }
  }, [config])

  const handleSave = async (setMessage: (msg: SettingsMessage | null) => void) => {
    setSaving(true)
    setMessage(null)
    try {
      await configService.updateConfig({
        daily_work_hours: dailyHours,
        normalize_hours: normalizeHours,
      })
      setMessage({ type: 'success', text: '偏好設定已儲存' })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '儲存失敗' })
    } finally {
      setSaving(false)
    }
  }

  return {
    dailyHours,
    setDailyHours,
    normalizeHours,
    setNormalizeHours,
    saving,
    handleSave,
  }
}

export function useLlmForm(config: ConfigResponse | null) {
  const [llmProvider, setLlmProvider] = useState('openai')
  const [llmModel, setLlmModel] = useState('gpt-4o-mini')
  const [llmApiKey, setLlmApiKey] = useState('')
  const [llmBaseUrl, setLlmBaseUrl] = useState('')
  const [showLlmKey, setShowLlmKey] = useState(false)
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    if (config) {
      setLlmProvider(config.llm_provider || 'openai')
      setLlmModel(config.llm_model || 'gpt-4o-mini')
      setLlmBaseUrl(config.llm_base_url || '')
    }
  }, [config])

  const handleProviderChange = (providerId: string) => {
    setLlmProvider(providerId)
    if (providerId === 'openai') setLlmModel('gpt-4o-mini')
    else if (providerId === 'anthropic') setLlmModel('claude-3-5-sonnet-20241022')
    else if (providerId === 'ollama') setLlmModel('llama3.2')
    else setLlmModel('')
    if (providerId === 'ollama') setLlmBaseUrl('http://localhost:11434')
    else if (providerId !== 'openai-compatible') setLlmBaseUrl('')
  }

  const handleSave = async (
    setMessage: (msg: SettingsMessage | null) => void,
    refreshConfig: () => Promise<ConfigResponse>
  ) => {
    setSaving(true)
    setMessage(null)
    try {
      await configService.updateLlmConfig({
        provider: llmProvider,
        model: llmModel,
        api_key: llmApiKey || undefined,
        base_url: llmBaseUrl || undefined,
      })
      await refreshConfig()
      setLlmApiKey('')
      setMessage({ type: 'success', text: 'LLM 設定已儲存' })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '儲存失敗' })
    } finally {
      setSaving(false)
    }
  }

  return {
    llmProvider,
    llmModel,
    setLlmModel,
    llmApiKey,
    setLlmApiKey,
    llmBaseUrl,
    setLlmBaseUrl,
    showLlmKey,
    setShowLlmKey,
    saving,
    handleProviderChange,
    handleSave,
  }
}

export function useJiraForm(config: ConfigResponse | null) {
  const [jiraUrl, setJiraUrl] = useState('')
  const [jiraAuthType, setJiraAuthType] = useState<'pat' | 'basic'>('pat')
  const [jiraToken, setJiraToken] = useState('')
  const [jiraEmail, setJiraEmail] = useState('')
  const [tempoToken, setTempoToken] = useState('')
  const [showToken, setShowToken] = useState(false)
  const [saving, setSaving] = useState(false)
  const [testing, setTesting] = useState(false)

  useEffect(() => {
    if (config) {
      setJiraUrl(config.jira_url || '')
      setJiraAuthType(config.auth_type === 'basic' ? 'basic' : 'pat')
    }
  }, [config])

  const handleSave = async (
    setMessage: (msg: SettingsMessage | null) => void,
    refreshConfig: () => Promise<ConfigResponse>
  ) => {
    setSaving(true)
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

      await configService.updateJiraConfig(payload)
      await refreshConfig()
      setJiraToken('')
      setTempoToken('')
      setMessage({ type: 'success', text: 'Jira 設定已儲存' })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '儲存失敗' })
    } finally {
      setSaving(false)
    }
  }

  const handleTest = async (setMessage: (msg: SettingsMessage | null) => void) => {
    setTesting(true)
    setMessage(null)
    try {
      const result = await tempo.testConnection()
      setMessage({ type: 'success', text: result.message })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '連線失敗' })
    } finally {
      setTesting(false)
    }
  }

  return {
    jiraUrl,
    setJiraUrl,
    jiraAuthType,
    setJiraAuthType,
    jiraToken,
    setJiraToken,
    jiraEmail,
    setJiraEmail,
    tempoToken,
    setTempoToken,
    showToken,
    setShowToken,
    saving,
    testing,
    handleSave,
    handleTest,
  }
}

export function useGitLabForm(config: ConfigResponse | null) {
  const [gitlabUrl, setGitlabUrl] = useState('')
  const [gitlabToken, setGitlabToken] = useState('')
  const [showToken, setShowToken] = useState(false)
  const [saving, setSaving] = useState(false)
  const [testing, setTesting] = useState(false)
  const [projects, setProjects] = useState<GitLabProject[]>([])
  const [searchResults, setSearchResults] = useState<GitLabProjectInfo[]>([])
  const [search, setSearch] = useState('')
  const [searching, setSearching] = useState(false)
  const [syncing, setSyncing] = useState(false)

  useEffect(() => {
    if (config) {
      setGitlabUrl(config.gitlab_url || '')
    }
  }, [config])

  const loadProjects = async () => {
    try {
      const projectsList = await gitlab.listProjects()
      setProjects(projectsList)
    } catch (err) {
      console.error('Failed to load GitLab projects:', err)
    }
  }

  const handleSave = async (
    setMessage: (msg: SettingsMessage | null) => void,
    refreshConfig: () => Promise<ConfigResponse>
  ) => {
    if (!gitlabUrl.trim()) return
    setSaving(true)
    setMessage(null)
    try {
      await gitlab.configure({ gitlab_url: gitlabUrl.trim(), gitlab_pat: gitlabToken })
      await refreshConfig()
      setGitlabToken('')
      setMessage({ type: 'success', text: 'GitLab 設定已儲存' })
      loadProjects()
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '儲存失敗' })
    } finally {
      setSaving(false)
    }
  }

  const handleTest = async (setMessage: (msg: SettingsMessage | null) => void) => {
    setTesting(true)
    setMessage(null)
    try {
      await gitlab.searchProjects({ search: '' })
      setMessage({ type: 'success', text: 'GitLab 連線成功' })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '連線失敗' })
    } finally {
      setTesting(false)
    }
  }

  const handleSearch = async (setMessage: (msg: SettingsMessage | null) => void) => {
    if (!search.trim()) return
    setSearching(true)
    try {
      const results = await gitlab.searchProjects({ search })
      setSearchResults(results)
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '搜尋失敗' })
    } finally {
      setSearching(false)
    }
  }

  const handleAddProject = async (projectId: number, setMessage: (msg: SettingsMessage | null) => void) => {
    try {
      await gitlab.addProject({ gitlab_project_id: projectId })
      setMessage({ type: 'success', text: '已新增 GitLab 專案' })
      loadProjects()
      setSearchResults(prev => prev.filter(p => p.id !== projectId))
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '新增失敗' })
    }
  }

  const handleRemoveProject = async (id: string, setMessage: (msg: SettingsMessage | null) => void) => {
    try {
      await gitlab.removeProject(id)
      setMessage({ type: 'success', text: '已移除 GitLab 專案' })
      loadProjects()
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '移除失敗' })
    }
  }

  const handleSync = async (setMessage: (msg: SettingsMessage | null) => void) => {
    setSyncing(true)
    setMessage(null)
    try {
      const result = await gitlab.sync()
      setMessage({ type: 'success', text: `已同步 ${result.work_items_created} 個工作項目` })
      loadProjects()
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '同步失敗' })
    } finally {
      setSyncing(false)
    }
  }

  const handleRemoveConfig = async (
    setMessage: (msg: SettingsMessage | null) => void,
    refreshConfig: () => Promise<ConfigResponse>
  ) => {
    try {
      await gitlab.removeConfig()
      await refreshConfig()
      setProjects([])
      setSearchResults([])
      setMessage({ type: 'success', text: 'GitLab 設定已移除' })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '移除失敗' })
    }
  }

  return {
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
    loadProjects,
    handleSave,
    handleTest,
    handleSearch,
    handleAddProject,
    handleRemoveProject,
    handleSync,
    handleRemoveConfig,
  }
}

export function useGitRepoForm() {
  const [newRepoPath, setNewRepoPath] = useState('')
  const [adding, setAdding] = useState(false)

  const handleAdd = async (
    setMessage: (msg: SettingsMessage | null) => void,
    refreshSources: () => Promise<SourcesResponse>
  ) => {
    if (!newRepoPath.trim()) return
    setAdding(true)
    setMessage(null)
    try {
      await sourcesService.addGitRepo(newRepoPath.trim())
      await refreshSources()
      setNewRepoPath('')
      setMessage({ type: 'success', text: '已新增 Git 倉庫' })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '新增失敗' })
    } finally {
      setAdding(false)
    }
  }

  const handleRemove = async (
    repoId: string,
    setMessage: (msg: SettingsMessage | null) => void,
    refreshSources: () => Promise<SourcesResponse>
  ) => {
    setMessage(null)
    try {
      await sourcesService.removeGitRepo(repoId)
      await refreshSources()
      setMessage({ type: 'success', text: '已移除 Git 倉庫' })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '移除失敗' })
    }
  }

  return {
    newRepoPath,
    setNewRepoPath,
    adding,
    handleAdd,
    handleRemove,
  }
}

export function useClaudeCodeForm() {
  const [projects, setProjects] = useState<ClaudeProject[]>([])
  const [loading, setLoading] = useState(false)
  const [selectedProjects, setSelectedProjects] = useState<Set<string>>(() => {
    const saved = localStorage.getItem('recap-selected-claude-projects')
    return saved ? new Set(JSON.parse(saved)) : new Set()
  })
  const [expandedProjects, setExpandedProjects] = useState<Set<string>>(new Set())
  const [importing, setImporting] = useState(false)

  useEffect(() => {
    localStorage.setItem('recap-selected-claude-projects', JSON.stringify(Array.from(selectedProjects)))
  }, [selectedProjects])

  const loadSessions = async (
    sources: SourcesResponse | null,
    setMessage: (msg: SettingsMessage | null) => void,
    refreshSources: () => Promise<SourcesResponse>
  ) => {
    setLoading(true)
    setMessage(null)
    try {
      const projectsList = await claude.listSessions()
      setProjects(projectsList)
      if (projectsList.length > 0) {
        setExpandedProjects(new Set([projectsList[0].path]))
      }

      // Auto-add detected Git repos
      if (projectsList.length > 0 && sources) {
        const existingPaths = sources.git_repos?.map(r => r.path) || []
        const newProjects = projectsList.filter(p => !existingPaths.includes(p.path))

        if (newProjects.length > 0) {
          const addPromises = newProjects.map(p =>
            sourcesService.addGitRepo(p.path).catch(() => null)
          )
          await Promise.all(addPromises)
          await refreshSources()
          setMessage({
            type: 'success',
            text: `已自動新增 ${newProjects.length} 個 Git 倉庫`
          })
        }
      }
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '載入失敗' })
    } finally {
      setLoading(false)
    }
  }

  const toggleExpandProject = (path: string) => {
    setExpandedProjects(prev => {
      const next = new Set(prev)
      if (next.has(path)) next.delete(path)
      else next.add(path)
      return next
    })
  }

  const toggleProjectSelection = (path: string) => {
    setSelectedProjects(prev => {
      const next = new Set(prev)
      if (next.has(path)) next.delete(path)
      else next.add(path)
      return next
    })
  }

  const selectAllProjects = () => {
    setSelectedProjects(new Set(projects.map(p => p.path)))
  }

  const clearSelection = () => {
    setSelectedProjects(new Set())
  }

  const selectedSessionCount = projects
    .filter(p => selectedProjects.has(p.path))
    .reduce((acc, p) => acc + p.sessions.length, 0)

  const handleImport = async (setMessage: (msg: SettingsMessage | null) => void) => {
    if (selectedProjects.size === 0) return
    setImporting(true)
    setMessage(null)
    try {
      const sessionIds = projects
        .filter(p => selectedProjects.has(p.path))
        .flatMap(p => p.sessions.map(s => s.session_id))

      const result = await claude.importSessions({ session_ids: sessionIds })
      setMessage({
        type: 'success',
        text: `已匯入 ${result.imported} 個 session，建立 ${result.work_items_created} 個工作項目`,
      })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '匯入失敗' })
    } finally {
      setImporting(false)
    }
  }

  return {
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
  }
}

// Utility functions
export const formatFileSize = (bytes: number): string => {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

export const formatTimestamp = (ts?: string): string => {
  if (!ts) return '-'
  const date = new Date(ts)
  return date.toLocaleString('zh-TW', {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
}
