import { describe, it, expect, vi } from 'vitest'
import { renderHook } from '@testing-library/react'
import { IntegrationsProvider, useIntegrations } from './IntegrationsContext'
import type { ReactNode } from 'react'

// Mock the hooks
vi.mock('../hooks/useJiraForm', () => ({
  useJiraForm: () => ({
    jiraUrl: '',
    setJiraUrl: vi.fn(),
    jiraAuthType: 'pat' as const,
    setJiraAuthType: vi.fn(),
    jiraToken: '',
    setJiraToken: vi.fn(),
    jiraEmail: '',
    setJiraEmail: vi.fn(),
    tempoToken: '',
    setTempoToken: vi.fn(),
    showToken: false,
    setShowToken: vi.fn(),
    saving: false,
    testing: false,
    handleSave: vi.fn(),
    handleTest: vi.fn(),
  }),
}))

vi.mock('../hooks/useGitLabForm', () => ({
  useGitLabForm: () => ({
    gitlabUrl: '',
    setGitlabUrl: vi.fn(),
    gitlabToken: '',
    setGitlabToken: vi.fn(),
    showToken: false,
    setShowToken: vi.fn(),
    saving: false,
    testing: false,
    projects: [],
    searchResults: [],
    search: '',
    setSearch: vi.fn(),
    searching: false,
    syncing: false,
    loadProjects: vi.fn(),
    handleSave: vi.fn(),
    handleTest: vi.fn(),
    handleSearch: vi.fn(),
    handleAddProject: vi.fn(),
    handleRemoveProject: vi.fn(),
    handleSync: vi.fn(),
    handleRemoveConfig: vi.fn(),
  }),
}))

vi.mock('../hooks/useGitRepoForm', () => ({
  useGitRepoForm: () => ({
    newRepoPath: '',
    setNewRepoPath: vi.fn(),
    adding: false,
    handleAdd: vi.fn(),
    handleRemove: vi.fn(),
  }),
}))

vi.mock('../hooks/useClaudeCodeForm', () => ({
  useClaudeCodeForm: () => ({
    projects: [],
    loading: false,
    selectedProjects: new Set(),
    expandedProjects: new Set(),
    importing: false,
    selectedSessionCount: 0,
    loadSessions: vi.fn(),
    toggleExpandProject: vi.fn(),
    toggleProjectSelection: vi.fn(),
    selectAllProjects: vi.fn(),
    clearSelection: vi.fn(),
    handleImport: vi.fn(),
  }),
}))

describe('IntegrationsContext', () => {
  const mockConfig = {
    daily_work_hours: 8,
    normalize_hours: true,
    timezone: null,
    week_start_day: 1,
    jira_url: null,
    jira_configured: false,
    tempo_configured: false,
    gitlab_url: null,
    gitlab_configured: false,
    llm_provider: '',
    llm_model: '',
    llm_base_url: null,
    llm_configured: false,
    auth_type: '',
    use_git_mode: false,
    git_repos: [] as string[],
    outlook_enabled: false,
  }

  const mockSources = {
    mode: 'local',
    git_repos: [],
    claude_connected: false,
    claude_path: '',
  }

  const defaultProps = {
    config: mockConfig,
    sources: mockSources,
    setSources: vi.fn(),
    setMessage: vi.fn(),
    refreshConfig: vi.fn().mockResolvedValue(mockConfig),
    refreshSources: vi.fn().mockResolvedValue(mockSources),
    isAuthenticated: true,
  }

  const wrapper = ({ children }: { children: ReactNode }) => (
    <IntegrationsProvider {...defaultProps}>{children}</IntegrationsProvider>
  )

  it('should throw error when used outside provider', () => {
    // Suppress console.error for this test
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {})

    expect(() => {
      renderHook(() => useIntegrations())
    }).toThrow('useIntegrations must be used within IntegrationsProvider')

    consoleSpy.mockRestore()
  })

  it('should provide context values when used within provider', () => {
    const { result } = renderHook(() => useIntegrations(), { wrapper })

    expect(result.current.config).toEqual(mockConfig)
    expect(result.current.sources).toEqual(mockSources)
    expect(result.current.isAuthenticated).toBe(true)
  })

  it('should provide gitRepo hook values', () => {
    const { result } = renderHook(() => useIntegrations(), { wrapper })

    expect(result.current.gitRepo).toBeDefined()
    expect(result.current.gitRepo.newRepoPath).toBe('')
    expect(typeof result.current.gitRepo.handleAdd).toBe('function')
  })

  it('should provide claudeCode hook values', () => {
    const { result } = renderHook(() => useIntegrations(), { wrapper })

    expect(result.current.claudeCode).toBeDefined()
    expect(result.current.claudeCode.projects).toEqual([])
    expect(result.current.claudeCode.loading).toBe(false)
  })

  it('should provide jira hook values', () => {
    const { result } = renderHook(() => useIntegrations(), { wrapper })

    expect(result.current.jira).toBeDefined()
    expect(result.current.jira.jiraUrl).toBe('')
    expect(result.current.jira.jiraAuthType).toBe('pat')
  })

  it('should provide gitlab hook values', () => {
    const { result } = renderHook(() => useIntegrations(), { wrapper })

    expect(result.current.gitlab).toBeDefined()
    expect(result.current.gitlab.gitlabUrl).toBe('')
    expect(result.current.gitlab.projects).toEqual([])
  })

  it('should provide setMessage and refresh functions', () => {
    const { result } = renderHook(() => useIntegrations(), { wrapper })

    expect(typeof result.current.setMessage).toBe('function')
    expect(typeof result.current.refreshConfig).toBe('function')
    expect(typeof result.current.refreshSources).toBe('function')
  })
})
