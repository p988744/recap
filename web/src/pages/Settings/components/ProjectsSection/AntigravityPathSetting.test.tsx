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
      () => new Promise(() => {}) // never resolves
    )

    render(<AntigravityPathSetting />)

    expect(screen.getByText('Antigravity API')).toBeInTheDocument()
  })

  it('should show healthy status when API is running', async () => {
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

  it('should show disconnected status on API error', async () => {
    vi.mocked(antigravity.checkApiStatus).mockRejectedValue(
      new Error('Connection refused')
    )

    render(<AntigravityPathSetting />)

    await waitFor(() => {
      expect(screen.getByText('未連線')).toBeInTheDocument()
    })
  })

  it('should show healthy but without session count when not provided', async () => {
    vi.mocked(antigravity.checkApiStatus).mockResolvedValue({
      running: true,
      healthy: true,
      api_url: 'http://localhost:3000',
    })

    render(<AntigravityPathSetting />)

    await waitFor(() => {
      expect(screen.getByText('連線正常')).toBeInTheDocument()
    })

    expect(screen.getByText('http://localhost:3000')).toBeInTheDocument()
    // Session count should not be shown
    expect(screen.queryByText('Session 數')).not.toBeInTheDocument()
  })

  it('should re-check status when refresh button is clicked', async () => {
    vi.mocked(antigravity.checkApiStatus).mockResolvedValue({
      running: false,
      healthy: false,
    })

    render(<AntigravityPathSetting />)

    await waitFor(() => {
      expect(screen.getByText('未連線')).toBeInTheDocument()
    })

    // Now mock healthy response
    vi.mocked(antigravity.checkApiStatus).mockResolvedValue({
      running: true,
      healthy: true,
      api_url: 'http://localhost:3000',
    })

    // Click refresh button
    fireEvent.click(screen.getByTitle('重新檢查'))

    await waitFor(() => {
      expect(screen.getByText('連線正常')).toBeInTheDocument()
    })

    expect(antigravity.checkApiStatus).toHaveBeenCalledTimes(2)
  })

  it('should show hint text when not connected', async () => {
    vi.mocked(antigravity.checkApiStatus).mockResolvedValue({
      running: true,
      healthy: false,
    })

    render(<AntigravityPathSetting />)

    await waitFor(() => {
      expect(screen.getByText('未連線')).toBeInTheDocument()
    })

    expect(screen.getByText(/請先開啟 Antigravity 應用程式/)).toBeInTheDocument()
  })
})
