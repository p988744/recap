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
    // Loading spinner should be present
    expect(document.querySelector('.animate-spin')).toBeInTheDocument()
  })

  it('should show connected status when API is healthy', async () => {
    vi.mocked(antigravity.checkApiStatus).mockResolvedValue({
      running: true,
      healthy: true,
      api_url: 'http://localhost:3000',
      session_count: 5,
    })

    render(<AntigravityPathSetting />)

    await waitFor(() => {
      expect(screen.getByText('連線正常')).toBeInTheDocument()
    })

    expect(screen.getByText('http://localhost:3000')).toBeInTheDocument()
    expect(screen.getByText('5')).toBeInTheDocument()
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

  it('should show disconnected status when API check fails', async () => {
    vi.mocked(antigravity.checkApiStatus).mockRejectedValue(
      new Error('Network error')
    )

    render(<AntigravityPathSetting />)

    await waitFor(() => {
      expect(screen.getByText('未連線')).toBeInTheDocument()
    })
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
        api_url: 'http://localhost:3000',
      })

    render(<AntigravityPathSetting />)

    // Wait for initial load
    await waitFor(() => {
      expect(screen.getByText('未連線')).toBeInTheDocument()
    })

    // Click refresh button
    const refreshButton = screen.getByTitle('重新檢查')
    fireEvent.click(refreshButton)

    // Should now show connected
    await waitFor(() => {
      expect(screen.getByText('連線正常')).toBeInTheDocument()
    })

    expect(antigravity.checkApiStatus).toHaveBeenCalledTimes(2)
  })

  it('should show running but unhealthy status', async () => {
    vi.mocked(antigravity.checkApiStatus).mockResolvedValue({
      running: true,
      healthy: false,
    })

    render(<AntigravityPathSetting />)

    await waitFor(() => {
      expect(screen.getByText('未連線')).toBeInTheDocument()
    })
  })
})
