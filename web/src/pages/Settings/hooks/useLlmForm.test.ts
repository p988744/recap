import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useLlmForm } from './useLlmForm'
import { config as configService } from '@/services'

vi.mock('@/services', () => ({
  config: {
    updateLlmConfig: vi.fn(),
  },
}))

describe('useLlmForm', () => {
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
    llm_provider: 'anthropic',
    llm_model: 'claude-3-5-sonnet-20241022',
    llm_base_url: '',
    llm_configured: true,
    auth_type: '',
    use_git_mode: false,
    git_repos: [] as string[],
    outlook_enabled: false,
  }

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('should initialize with default values when config is null', () => {
    const { result } = renderHook(() => useLlmForm(null))

    expect(result.current.llmProvider).toBe('openai')
    expect(result.current.llmModel).toBe('gpt-4o-mini')
    expect(result.current.llmBaseUrl).toBe('')
  })

  it('should initialize with config values when config is provided', () => {
    const { result } = renderHook(() => useLlmForm(mockConfig))

    expect(result.current.llmProvider).toBe('anthropic')
    expect(result.current.llmModel).toBe('claude-3-5-sonnet-20241022')
  })

  it('should change model when provider changes to openai', () => {
    const { result } = renderHook(() => useLlmForm(mockConfig))

    act(() => {
      result.current.handleProviderChange('openai')
    })

    expect(result.current.llmProvider).toBe('openai')
    expect(result.current.llmModel).toBe('gpt-4o-mini')
  })

  it('should change model when provider changes to anthropic', () => {
    const { result } = renderHook(() => useLlmForm(null))

    act(() => {
      result.current.handleProviderChange('anthropic')
    })

    expect(result.current.llmProvider).toBe('anthropic')
    expect(result.current.llmModel).toBe('claude-3-5-sonnet-20241022')
  })

  it('should set base URL when provider changes to ollama', () => {
    const { result } = renderHook(() => useLlmForm(null))

    act(() => {
      result.current.handleProviderChange('ollama')
    })

    expect(result.current.llmProvider).toBe('ollama')
    expect(result.current.llmModel).toBe('llama3.2')
    expect(result.current.llmBaseUrl).toBe('http://localhost:11434')
  })

  it('should toggle show API key', () => {
    const { result } = renderHook(() => useLlmForm(null))

    expect(result.current.showLlmKey).toBe(false)

    act(() => {
      result.current.setShowLlmKey(true)
    })

    expect(result.current.showLlmKey).toBe(true)
  })

  it('should call configService.updateLlmConfig on save', async () => {
    vi.mocked(configService.updateLlmConfig).mockResolvedValue({ message: 'success' })
    const setMessage = vi.fn()
    const refreshConfig = vi.fn().mockResolvedValue(mockConfig)
    const { result } = renderHook(() => useLlmForm(mockConfig))

    act(() => {
      result.current.setLlmApiKey('sk-test-key')
    })

    await act(async () => {
      await result.current.handleSave(setMessage, refreshConfig)
    })

    expect(configService.updateLlmConfig).toHaveBeenCalledWith({
      provider: 'anthropic',
      model: 'claude-3-5-sonnet-20241022',
      api_key: 'sk-test-key',
      base_url: undefined,
    })
    expect(refreshConfig).toHaveBeenCalled()
    expect(setMessage).toHaveBeenCalledWith({ type: 'success', text: 'LLM 設定已儲存' })
    // API key should be cleared after save
    expect(result.current.llmApiKey).toBe('')
  })

  it('should handle save error', async () => {
    vi.mocked(configService.updateLlmConfig).mockRejectedValue(new Error('Invalid API key'))
    const setMessage = vi.fn()
    const refreshConfig = vi.fn()
    const { result } = renderHook(() => useLlmForm(mockConfig))

    await act(async () => {
      await result.current.handleSave(setMessage, refreshConfig)
    })

    expect(setMessage).toHaveBeenCalledWith({ type: 'error', text: 'Invalid API key' })
  })
})
