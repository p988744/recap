import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useGitLabForm } from './useGitLabForm'
import { gitlab } from '@/services'

vi.mock('@/services', () => ({
  gitlab: {
    configure: vi.fn(),
    searchProjects: vi.fn(),
    listProjects: vi.fn(),
    addProject: vi.fn(),
    removeProject: vi.fn(),
    sync: vi.fn(),
    removeConfig: vi.fn(),
  },
}))

describe('useGitLabForm', () => {
  const mockConfig = {
    daily_work_hours: 8,
    normalize_hours: true,
    timezone: null,
    week_start_day: 1,
    jira_url: null,
    jira_configured: false,
    tempo_configured: false,
    gitlab_url: 'https://gitlab.company.com',
    gitlab_configured: true,
    llm_provider: '',
    llm_model: '',
    llm_base_url: null,
    llm_configured: false,
    auth_type: 'pat',
    use_git_mode: false,
    git_repos: [] as string[],
    outlook_enabled: false,
  }

  const mockProjects = [
    { id: 'proj-1', user_id: 'user-1', gitlab_project_id: 123, name: 'Project A', path_with_namespace: 'team/project-a', gitlab_url: 'https://gitlab.company.com', default_branch: 'main', enabled: true, created_at: '2024-01-01T00:00:00Z' },
    { id: 'proj-2', user_id: 'user-1', gitlab_project_id: 456, name: 'Project B', path_with_namespace: 'team/project-b', gitlab_url: 'https://gitlab.company.com', default_branch: 'main', enabled: true, created_at: '2024-01-01T00:00:00Z' },
  ]

  const mockSearchResults = [
    { id: 789, name: 'Project C', path_with_namespace: 'team/project-c', web_url: 'https://gitlab.com/team/project-c' },
    { id: 101, name: 'Project D', path_with_namespace: 'team/project-d', web_url: 'https://gitlab.com/team/project-d' },
  ]

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('should initialize with default values when config is null', () => {
    const { result } = renderHook(() => useGitLabForm(null))

    expect(result.current.gitlabUrl).toBe('')
    expect(result.current.gitlabToken).toBe('')
    expect(result.current.showToken).toBe(false)
    expect(result.current.saving).toBe(false)
    expect(result.current.testing).toBe(false)
    expect(result.current.projects).toEqual([])
    expect(result.current.searchResults).toEqual([])
    expect(result.current.search).toBe('')
    expect(result.current.searching).toBe(false)
    expect(result.current.syncing).toBe(false)
  })

  it('should initialize with config values when provided', () => {
    const { result } = renderHook(() => useGitLabForm(mockConfig))

    expect(result.current.gitlabUrl).toBe('https://gitlab.company.com')
  })

  it('should update gitlab URL', () => {
    const { result } = renderHook(() => useGitLabForm(mockConfig))

    act(() => {
      result.current.setGitlabUrl('https://new-gitlab.com')
    })

    expect(result.current.gitlabUrl).toBe('https://new-gitlab.com')
  })

  it('should toggle show token', () => {
    const { result } = renderHook(() => useGitLabForm(null))

    expect(result.current.showToken).toBe(false)

    act(() => {
      result.current.setShowToken(true)
    })

    expect(result.current.showToken).toBe(true)
  })

  it('should load projects successfully', async () => {
    vi.mocked(gitlab.listProjects).mockResolvedValue(mockProjects)
    const { result } = renderHook(() => useGitLabForm(mockConfig))

    await act(async () => {
      await result.current.loadProjects()
    })

    expect(gitlab.listProjects).toHaveBeenCalled()
    expect(result.current.projects).toEqual(mockProjects)
  })

  it('should handle load projects error silently', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    vi.mocked(gitlab.listProjects).mockRejectedValue(new Error('Failed to load'))
    const { result } = renderHook(() => useGitLabForm(mockConfig))

    await act(async () => {
      await result.current.loadProjects()
    })

    expect(consoleSpy).toHaveBeenCalled()
    expect(result.current.projects).toEqual([])
    consoleSpy.mockRestore()
  })

  it('should not save when URL is empty', async () => {
    const setMessage = vi.fn()
    const refreshConfig = vi.fn()
    const { result } = renderHook(() => useGitLabForm(null))

    await act(async () => {
      await result.current.handleSave(setMessage, refreshConfig)
    })

    expect(gitlab.configure).not.toHaveBeenCalled()
  })

  it('should save gitlab config successfully', async () => {
    vi.mocked(gitlab.configure).mockResolvedValue({ message: 'success' })
    vi.mocked(gitlab.listProjects).mockResolvedValue(mockProjects)
    const setMessage = vi.fn()
    const refreshConfig = vi.fn().mockResolvedValue(mockConfig)
    const { result } = renderHook(() => useGitLabForm(mockConfig))

    act(() => {
      result.current.setGitlabToken('test-token')
    })

    await act(async () => {
      await result.current.handleSave(setMessage, refreshConfig)
    })

    expect(gitlab.configure).toHaveBeenCalledWith({
      gitlab_url: 'https://gitlab.company.com',
      gitlab_pat: 'test-token',
    })
    expect(refreshConfig).toHaveBeenCalled()
    expect(setMessage).toHaveBeenCalledWith({ type: 'success', text: 'GitLab 設定已儲存' })
    expect(result.current.gitlabToken).toBe('')
  })

  it('should handle save error', async () => {
    vi.mocked(gitlab.configure).mockRejectedValue(new Error('Invalid token'))
    const setMessage = vi.fn()
    const refreshConfig = vi.fn()
    const { result } = renderHook(() => useGitLabForm(mockConfig))

    act(() => {
      result.current.setGitlabToken('invalid-token')
    })

    await act(async () => {
      await result.current.handleSave(setMessage, refreshConfig)
    })

    expect(setMessage).toHaveBeenCalledWith({ type: 'error', text: 'Invalid token' })
  })

  it('should test connection successfully', async () => {
    vi.mocked(gitlab.searchProjects).mockResolvedValue([])
    const setMessage = vi.fn()
    const { result } = renderHook(() => useGitLabForm(mockConfig))

    await act(async () => {
      await result.current.handleTest(setMessage)
    })

    expect(gitlab.searchProjects).toHaveBeenCalledWith({ search: '' })
    expect(setMessage).toHaveBeenCalledWith({ type: 'success', text: 'GitLab 連線成功' })
  })

  it('should handle test connection error', async () => {
    vi.mocked(gitlab.searchProjects).mockRejectedValue(new Error('Connection failed'))
    const setMessage = vi.fn()
    const { result } = renderHook(() => useGitLabForm(mockConfig))

    await act(async () => {
      await result.current.handleTest(setMessage)
    })

    expect(setMessage).toHaveBeenCalledWith({ type: 'error', text: 'Connection failed' })
  })

  it('should not search when search term is empty', async () => {
    const setMessage = vi.fn()
    const { result } = renderHook(() => useGitLabForm(mockConfig))

    await act(async () => {
      await result.current.handleSearch(setMessage)
    })

    expect(gitlab.searchProjects).not.toHaveBeenCalled()
  })

  it('should search projects successfully', async () => {
    vi.mocked(gitlab.searchProjects).mockResolvedValue(mockSearchResults)
    const setMessage = vi.fn()
    const { result } = renderHook(() => useGitLabForm(mockConfig))

    act(() => {
      result.current.setSearch('project')
    })

    await act(async () => {
      await result.current.handleSearch(setMessage)
    })

    expect(gitlab.searchProjects).toHaveBeenCalledWith({ search: 'project' })
    expect(result.current.searchResults).toEqual(mockSearchResults)
  })

  it('should handle search error', async () => {
    vi.mocked(gitlab.searchProjects).mockRejectedValue(new Error('Search failed'))
    const setMessage = vi.fn()
    const { result } = renderHook(() => useGitLabForm(mockConfig))

    act(() => {
      result.current.setSearch('project')
    })

    await act(async () => {
      await result.current.handleSearch(setMessage)
    })

    expect(setMessage).toHaveBeenCalledWith({ type: 'error', text: 'Search failed' })
  })

  it('should add project successfully', async () => {
    vi.mocked(gitlab.addProject).mockResolvedValue({ id: 'new-proj', user_id: 'user-1', gitlab_project_id: 789, name: 'Project C', path_with_namespace: 'team/project-c', gitlab_url: 'https://gitlab.company.com', default_branch: 'main', enabled: true, created_at: '2024-01-01T00:00:00Z' })
    vi.mocked(gitlab.listProjects).mockResolvedValue(mockProjects)
    const setMessage = vi.fn()
    const { result } = renderHook(() => useGitLabForm(mockConfig))

    // Set initial search results
    act(() => {
      result.current.setSearch('test')
    })
    vi.mocked(gitlab.searchProjects).mockResolvedValue(mockSearchResults)
    await act(async () => {
      await result.current.handleSearch(setMessage)
    })

    await act(async () => {
      await result.current.handleAddProject(789, setMessage)
    })

    expect(gitlab.addProject).toHaveBeenCalledWith({ gitlab_project_id: 789 })
    expect(setMessage).toHaveBeenCalledWith({ type: 'success', text: '已新增 GitLab 專案' })
    // Should remove the added project from search results
    expect(result.current.searchResults.find(p => p.id === 789)).toBeUndefined()
  })

  it('should handle add project error', async () => {
    vi.mocked(gitlab.addProject).mockRejectedValue(new Error('Add failed'))
    const setMessage = vi.fn()
    const { result } = renderHook(() => useGitLabForm(mockConfig))

    await act(async () => {
      await result.current.handleAddProject(789, setMessage)
    })

    expect(setMessage).toHaveBeenCalledWith({ type: 'error', text: 'Add failed' })
  })

  it('should remove project successfully', async () => {
    vi.mocked(gitlab.removeProject).mockResolvedValue({ message: 'success' })
    vi.mocked(gitlab.listProjects).mockResolvedValue([])
    const setMessage = vi.fn()
    const { result } = renderHook(() => useGitLabForm(mockConfig))

    await act(async () => {
      await result.current.handleRemoveProject('proj-1', setMessage)
    })

    expect(gitlab.removeProject).toHaveBeenCalledWith('proj-1')
    expect(setMessage).toHaveBeenCalledWith({ type: 'success', text: '已移除 GitLab 專案' })
  })

  it('should handle remove project error', async () => {
    vi.mocked(gitlab.removeProject).mockRejectedValue(new Error('Remove failed'))
    const setMessage = vi.fn()
    const { result } = renderHook(() => useGitLabForm(mockConfig))

    await act(async () => {
      await result.current.handleRemoveProject('proj-1', setMessage)
    })

    expect(setMessage).toHaveBeenCalledWith({ type: 'error', text: 'Remove failed' })
  })

  it('should sync projects successfully', async () => {
    vi.mocked(gitlab.sync).mockResolvedValue({ synced_commits: 10, synced_merge_requests: 2, work_items_created: 5 })
    vi.mocked(gitlab.listProjects).mockResolvedValue(mockProjects)
    const setMessage = vi.fn()
    const { result } = renderHook(() => useGitLabForm(mockConfig))

    await act(async () => {
      await result.current.handleSync(setMessage)
    })

    expect(gitlab.sync).toHaveBeenCalled()
    expect(setMessage).toHaveBeenCalledWith({ type: 'success', text: '已同步 5 個工作項目' })
  })

  it('should handle sync error', async () => {
    vi.mocked(gitlab.sync).mockRejectedValue(new Error('Sync failed'))
    const setMessage = vi.fn()
    const { result } = renderHook(() => useGitLabForm(mockConfig))

    await act(async () => {
      await result.current.handleSync(setMessage)
    })

    expect(setMessage).toHaveBeenCalledWith({ type: 'error', text: 'Sync failed' })
  })

  it('should remove config successfully', async () => {
    vi.mocked(gitlab.removeConfig).mockResolvedValue({ message: 'success' })
    const setMessage = vi.fn()
    const refreshConfig = vi.fn().mockResolvedValue({ ...mockConfig, gitlab_configured: false })
    const { result } = renderHook(() => useGitLabForm(mockConfig))

    // Set some projects first
    vi.mocked(gitlab.listProjects).mockResolvedValue(mockProjects)
    await act(async () => {
      await result.current.loadProjects()
    })

    await act(async () => {
      await result.current.handleRemoveConfig(setMessage, refreshConfig)
    })

    expect(gitlab.removeConfig).toHaveBeenCalled()
    expect(refreshConfig).toHaveBeenCalled()
    expect(setMessage).toHaveBeenCalledWith({ type: 'success', text: 'GitLab 設定已移除' })
    expect(result.current.projects).toEqual([])
    expect(result.current.searchResults).toEqual([])
  })

  it('should handle remove config error', async () => {
    vi.mocked(gitlab.removeConfig).mockRejectedValue(new Error('Remove config failed'))
    const setMessage = vi.fn()
    const refreshConfig = vi.fn()
    const { result } = renderHook(() => useGitLabForm(mockConfig))

    await act(async () => {
      await result.current.handleRemoveConfig(setMessage, refreshConfig)
    })

    expect(setMessage).toHaveBeenCalledWith({ type: 'error', text: 'Remove config failed' })
  })
})
