import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { usePreferencesForm } from './usePreferencesForm'
import { config as configService } from '@/services'

vi.mock('@/services', () => ({
  config: {
    updateConfig: vi.fn(),
  },
}))

describe('usePreferencesForm', () => {
  const mockConfig = {
    daily_work_hours: 8,
    normalize_hours: true,
    timezone: null as string | null,
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

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('should initialize with default values when config is null', () => {
    const { result } = renderHook(() => usePreferencesForm(null))

    expect(result.current.dailyHours).toBe(8)
    expect(result.current.normalizeHours).toBe(true)
    expect(result.current.timezone).toBe(null)
    expect(result.current.weekStartDay).toBe(1)
  })

  it('should initialize with config values when config is provided', () => {
    const customConfig = { ...mockConfig, daily_work_hours: 6, normalize_hours: false }
    const { result } = renderHook(() => usePreferencesForm(customConfig))

    expect(result.current.dailyHours).toBe(6)
    expect(result.current.normalizeHours).toBe(false)
  })

  it('should update daily hours', () => {
    const { result } = renderHook(() => usePreferencesForm(mockConfig))

    act(() => {
      result.current.setDailyHours(10)
    })

    expect(result.current.dailyHours).toBe(10)
  })

  it('should toggle normalize hours', () => {
    const { result } = renderHook(() => usePreferencesForm(mockConfig))

    act(() => {
      result.current.setNormalizeHours(false)
    })

    expect(result.current.normalizeHours).toBe(false)
  })

  it('should call configService.updateConfig on save', async () => {
    vi.mocked(configService.updateConfig).mockResolvedValue({ message: 'success' })
    const setMessage = vi.fn()
    const { result } = renderHook(() => usePreferencesForm(mockConfig))

    await act(async () => {
      await result.current.handleSave(setMessage)
    })

    expect(configService.updateConfig).toHaveBeenCalledWith({
      daily_work_hours: 8,
      normalize_hours: true,
      timezone: '',
      week_start_day: 1,
    })
    expect(setMessage).toHaveBeenCalledWith({ type: 'success', text: '偏好設定已儲存' })
  })

  it('should handle save error', async () => {
    vi.mocked(configService.updateConfig).mockRejectedValue(new Error('Save failed'))
    const setMessage = vi.fn()
    const { result } = renderHook(() => usePreferencesForm(mockConfig))

    await act(async () => {
      await result.current.handleSave(setMessage)
    })

    expect(setMessage).toHaveBeenCalledWith({ type: 'error', text: 'Save failed' })
  })

  it('should initialize timezone from config', () => {
    const configWithTz = { ...mockConfig, timezone: 'Asia/Taipei' }
    const { result } = renderHook(() => usePreferencesForm(configWithTz))

    expect(result.current.timezone).toBe('Asia/Taipei')
  })

  it('should initialize week start day from config', () => {
    const configWithDay = { ...mockConfig, week_start_day: 0 }
    const { result } = renderHook(() => usePreferencesForm(configWithDay))

    expect(result.current.weekStartDay).toBe(0)
  })

  it('should save timezone and week_start_day', async () => {
    vi.mocked(configService.updateConfig).mockResolvedValue({ message: 'success' })
    const setMessage = vi.fn()
    const configWithSettings = { ...mockConfig, timezone: 'Asia/Tokyo', week_start_day: 0 }
    const { result } = renderHook(() => usePreferencesForm(configWithSettings))

    await act(async () => {
      await result.current.handleSave(setMessage)
    })

    expect(configService.updateConfig).toHaveBeenCalledWith({
      daily_work_hours: 8,
      normalize_hours: true,
      timezone: 'Asia/Tokyo',
      week_start_day: 0,
    })
  })
})
