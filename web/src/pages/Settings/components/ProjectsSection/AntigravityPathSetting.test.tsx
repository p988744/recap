import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { AntigravityPathSetting } from './AntigravityPathSetting'
import { antigravity } from '@/services/integrations'

vi.mock('@/services/integrations', () => ({
  antigravity: {
    checkApiStatus: vi.fn(),
  },
}))

describe('AntigravityPathSetting', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('should show loading state initially', () => {
    vi.mocked(antigravity.checkApiStatus).mockImplementation(
      () => new Promise(() => {}) // Never resolves
    )

    render(<AntigravityPathSetting />)

    expect(screen.getByText('Antigravity API')).toBeInTheDocument()
    // Loading spinner should be visible
    expect(document.querySelector('.animate-spin')).toBeInTheDocument()
  })

  it('should show healthy status when API is running and healthy', async () => {
    vi.mocked(antigravity.checkApiStatus).mockResolvedValue({
      running: true,
      healthy: true,
      api_url: 'http://localhost:19281',
      session_count: 42,
    })

    render(<AntigravityPathSetting />)

    await waitFor(() => {
      expect(screen.getByText('連線正常')).toBeInTheDocument()
    })

    expect(screen.getByText('http://localhost:19281')).toBeInTheDocument()
    expect(screen.getByText('42')).toBeInTheDocument()
  })

  it('should show disconnected status when API is not running', async () => {
    vi.mocked(antigravity.checkApiStatus).mockResolvedValue({
      running: false,
      healthy: false,
    })

    render(<AntigravityPathSetting />)

    await waitFor(() => {
      expect(screen.getByText('未連線')).toBeInTheDocument()
    })

    expect(screen.getByText(/請先開啟 Antigravity 應用程式/)).toBeInTheDocument()
  })

  it('should show disconnected status when API is running but not healthy', async () => {
    vi.mocked(antigravity.checkApiStatus).mockResolvedValue({
      running: true,
      healthy: false,
    })

    render(<AntigravityPathSetting />)

    await waitFor(() => {
      expect(screen.getByText('未連線')).toBeInTheDocument()
    })
  })

  it('should handle API check failure gracefully', async () => {
    vi.mocked(antigravity.checkApiStatus).mockRejectedValue(
      new Error('Network error')
    )

    render(<AntigravityPathSetting />)

    await waitFor(() => {
      expect(screen.getByText('未連線')).toBeInTheDocument()
    })

    expect(screen.getByText(/請先開啟 Antigravity 應用程式/)).toBeInTheDocument()
  })

  it('should refresh status when refresh button is clicked', async () => {
    vi.mocked(antigravity.checkApiStatus)
      .mockResolvedValueOnce({
        running: false,
        healthy: false,
      })
      .mockResolvedValueOnce({
        running: true,
        healthy: true,
        api_url: 'http://localhost:19281',
        session_count: 10,
      })

    render(<AntigravityPathSetting />)

    // Wait for initial load
    await waitFor(() => {
      expect(screen.getByText('未連線')).toBeInTheDocument()
    })

    // Click refresh button
    const refreshButton = screen.getByTitle('重新檢查')
    fireEvent.click(refreshButton)

    // Wait for status to update
    await waitFor(() => {
      expect(screen.getByText('連線正常')).toBeInTheDocument()
    })

    expect(antigravity.checkApiStatus).toHaveBeenCalledTimes(2)
  })

  it('should show API URL when connected', async () => {
    vi.mocked(antigravity.checkApiStatus).mockResolvedValue({
      running: true,
      healthy: true,
      api_url: 'http://127.0.0.1:19281',
      session_count: 5,
    })

    render(<AntigravityPathSetting />)

    await waitFor(() => {
      expect(screen.getByText('API 端點')).toBeInTheDocument()
    })

    expect(screen.getByText('http://127.0.0.1:19281')).toBeInTheDocument()
  })

  it('should show session count when available', async () => {
    vi.mocked(antigravity.checkApiStatus).mockResolvedValue({
      running: true,
      healthy: true,
      api_url: 'http://localhost:19281',
      session_count: 123,
    })

    render(<AntigravityPathSetting />)

    await waitFor(() => {
      expect(screen.getByText('Session 數')).toBeInTheDocument()
    })

    expect(screen.getByText('123')).toBeInTheDocument()
  })

  it('should not show session count when undefined', async () => {
    vi.mocked(antigravity.checkApiStatus).mockResolvedValue({
      running: true,
      healthy: true,
      api_url: 'http://localhost:19281',
      // session_count is undefined
    })

    render(<AntigravityPathSetting />)

    await waitFor(() => {
      expect(screen.getByText('連線正常')).toBeInTheDocument()
    })

    expect(screen.queryByText('Session 數')).not.toBeInTheDocument()
  })

  it('should disable refresh button while re-checking', async () => {
    let resolveCheck: (value: unknown) => void
    vi.mocked(antigravity.checkApiStatus)
      .mockResolvedValueOnce({
        running: true,
        healthy: true,
        api_url: 'http://localhost:19281',
      })
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveCheck = resolve
          })
      )

    render(<AntigravityPathSetting />)

    // Wait for initial load to complete
    await waitFor(() => {
      expect(screen.getByText('連線正常')).toBeInTheDocument()
    })

    // Click refresh button to start re-checking
    const refreshButton = screen.getByTitle('重新檢查')
    fireEvent.click(refreshButton)

    // Button should be disabled while checking
    expect(refreshButton).toBeDisabled()

    // Resolve the check
    resolveCheck!({
      running: true,
      healthy: true,
      api_url: 'http://localhost:19281',
    })

    await waitFor(() => {
      expect(refreshButton).not.toBeDisabled()
    })
  })
})
