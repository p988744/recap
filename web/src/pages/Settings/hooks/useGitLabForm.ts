import { useEffect, useState } from 'react'
import { gitlab } from '@/services'
import type { ConfigResponse, GitLabProject, GitLabProjectInfo } from '@/types'
import type { SettingsMessage } from './types'

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
