import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useJiraForm } from './useJiraForm'
import { config as configService, tempo } from '@/services'

vi.mock('@/services', () => ({
  config: {
    updateJiraConfig: vi.fn(),
  },
  tempo: {
    testConnection: vi.fn(),
  },
}))

describe('useJiraForm', () => {
  const mockConfig = {
    daily_work_hours: 8,
    normalize_hours: true,
    timezone: null,
    week_start_day: 1,
    jira_url: 'https://company.atlassian.net',
    jira_configured: true,
    tempo_configured: false,
    gitlab_url: null,
    gitlab_configured: false,
    llm_provider: '',
    llm_model: '',
    llm_base_url: null,
    llm_configured: false,
    auth_type: 'pat',
    use_git_mode: false,
    git_repos: [] as string[],
    outlook_enabled: false,
  }

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('should initialize with default values when config is null', () => {
    const { result } = renderHook(() => useJiraForm(null))

    expect(result.current.jiraUrl).toBe('')
    expect(result.current.jiraAuthType).toBe('pat')
    expect(result.current.jiraToken).toBe('')
    expect(result.current.saving).toBe(false)
    expect(result.current.testing).toBe(false)
  })

  it('should initialize with config values when config is provided', () => {
    const { result } = renderHook(() => useJiraForm(mockConfig))

    expect(result.current.jiraUrl).toBe('https://company.atlassian.net')
    expect(result.current.jiraAuthType).toBe('pat')
  })

  it('should update jira URL', () => {
    const { result } = renderHook(() => useJiraForm(mockConfig))

    act(() => {
      result.current.setJiraUrl('https://new-url.atlassian.net')
    })

    expect(result.current.jiraUrl).toBe('https://new-url.atlassian.net')
  })

  it('should toggle auth type', () => {
    const { result } = renderHook(() => useJiraForm(mockConfig))

    act(() => {
      result.current.setJiraAuthType('basic')
    })

    expect(result.current.jiraAuthType).toBe('basic')
  })

  it('should toggle show token', () => {
    const { result } = renderHook(() => useJiraForm(null))

    expect(result.current.showToken).toBe(false)

    act(() => {
      result.current.setShowToken(true)
    })

    expect(result.current.showToken).toBe(true)
  })

  it('should save jira config with PAT auth', async () => {
    vi.mocked(configService.updateJiraConfig).mockResolvedValue({ message: 'success' })
    const setMessage = vi.fn()
    const refreshConfig = vi.fn().mockResolvedValue(mockConfig)
    const { result } = renderHook(() => useJiraForm(mockConfig))

    act(() => {
      result.current.setJiraToken('test-pat-token')
    })

    await act(async () => {
      await result.current.handleSave(setMessage, refreshConfig)
    })

    expect(configService.updateJiraConfig).toHaveBeenCalledWith({
      jira_url: 'https://company.atlassian.net',
      auth_type: 'pat',
      jira_pat: 'test-pat-token',
    })
    expect(refreshConfig).toHaveBeenCalled()
    expect(setMessage).toHaveBeenCalledWith({ type: 'success', text: 'Jira 設定已儲存' })
    expect(result.current.jiraToken).toBe('')
  })

  it('should save jira config with basic auth', async () => {
    vi.mocked(configService.updateJiraConfig).mockResolvedValue({ message: 'success' })
    const setMessage = vi.fn()
    const refreshConfig = vi.fn().mockResolvedValue(mockConfig)
    const { result } = renderHook(() => useJiraForm(mockConfig))

    act(() => {
      result.current.setJiraAuthType('basic')
      result.current.setJiraEmail('test@company.com')
      result.current.setJiraToken('test-api-token')
    })

    await act(async () => {
      await result.current.handleSave(setMessage, refreshConfig)
    })

    expect(configService.updateJiraConfig).toHaveBeenCalledWith({
      jira_url: 'https://company.atlassian.net',
      auth_type: 'basic',
      jira_api_token: 'test-api-token',
      jira_email: 'test@company.com',
    })
  })

  it('should handle save error', async () => {
    vi.mocked(configService.updateJiraConfig).mockRejectedValue(new Error('Invalid token'))
    const setMessage = vi.fn()
    const refreshConfig = vi.fn()
    const { result } = renderHook(() => useJiraForm(mockConfig))

    await act(async () => {
      await result.current.handleSave(setMessage, refreshConfig)
    })

    expect(setMessage).toHaveBeenCalledWith({ type: 'error', text: 'Invalid token' })
  })

  it('should test connection successfully', async () => {
    vi.mocked(tempo.testConnection).mockResolvedValue({ success: true, message: 'Connection successful' })
    const setMessage = vi.fn()
    const { result } = renderHook(() => useJiraForm(mockConfig))

    await act(async () => {
      await result.current.handleTest(setMessage)
    })

    expect(tempo.testConnection).toHaveBeenCalled()
    expect(setMessage).toHaveBeenCalledWith({ type: 'success', text: 'Connection successful' })
  })

  it('should handle test connection error', async () => {
    vi.mocked(tempo.testConnection).mockRejectedValue(new Error('Connection failed'))
    const setMessage = vi.fn()
    const { result } = renderHook(() => useJiraForm(mockConfig))

    await act(async () => {
      await result.current.handleTest(setMessage)
    })

    expect(setMessage).toHaveBeenCalledWith({ type: 'error', text: 'Connection failed' })
  })
})
