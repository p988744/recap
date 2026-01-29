import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { ClaudePathSetting } from './ClaudePathSetting'
import { projects } from '@/services'

vi.mock('@/services', () => ({
  projects: {
    getClaudeSessionPath: vi.fn(),
    updateClaudeSessionPath: vi.fn(),
  },
}))

describe('ClaudePathSetting', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('should render with default path', async () => {
    vi.mocked(projects.getClaudeSessionPath).mockResolvedValue({
      path: '/Users/test/.claude',
      is_default: true,
    })

    render(<ClaudePathSetting />)

    await waitFor(() => {
      expect(screen.getByText('/Users/test/.claude')).toBeInTheDocument()
    })

    expect(screen.getByText('預設')).toBeInTheDocument()
    expect(screen.getByText('Claude Code Session 路徑')).toBeInTheDocument()
  })

  it('should render with custom path (non-default)', async () => {
    vi.mocked(projects.getClaudeSessionPath).mockResolvedValue({
      path: '/custom/path/claude',
      is_default: false,
    })

    render(<ClaudePathSetting />)

    await waitFor(() => {
      expect(screen.getByText('/custom/path/claude')).toBeInTheDocument()
    })

    expect(screen.queryByText('預設')).not.toBeInTheDocument()
  })

  it('should enter edit mode when path is clicked', async () => {
    vi.mocked(projects.getClaudeSessionPath).mockResolvedValue({
      path: '/Users/test/.claude',
      is_default: true,
    })

    render(<ClaudePathSetting />)

    await waitFor(() => {
      expect(screen.getByText('/Users/test/.claude')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByText('/Users/test/.claude'))

    const input = screen.getByRole('textbox')
    expect(input).toBeInTheDocument()
    expect(input).toHaveValue('/Users/test/.claude')
    expect(screen.getByText('儲存')).toBeInTheDocument()
    expect(screen.getByText('取消')).toBeInTheDocument()
  })

  it('should cancel edit mode', async () => {
    vi.mocked(projects.getClaudeSessionPath).mockResolvedValue({
      path: '/Users/test/.claude',
      is_default: true,
    })

    render(<ClaudePathSetting />)

    await waitFor(() => {
      expect(screen.getByText('/Users/test/.claude')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByText('/Users/test/.claude'))
    fireEvent.click(screen.getByText('取消'))

    expect(screen.queryByRole('textbox')).not.toBeInTheDocument()
    expect(screen.getByText('/Users/test/.claude')).toBeInTheDocument()
  })

  it('should save new path', async () => {
    vi.mocked(projects.getClaudeSessionPath).mockResolvedValue({
      path: '/Users/test/.claude',
      is_default: true,
    })
    vi.mocked(projects.updateClaudeSessionPath).mockResolvedValue('ok')

    render(<ClaudePathSetting />)

    await waitFor(() => {
      expect(screen.getByText('/Users/test/.claude')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByText('/Users/test/.claude'))

    const input = screen.getByRole('textbox')
    fireEvent.change(input, { target: { value: '/new/custom/path' } })

    vi.mocked(projects.getClaudeSessionPath).mockResolvedValue({
      path: '/new/custom/path',
      is_default: false,
    })

    fireEvent.click(screen.getByText('儲存'))

    await waitFor(() => {
      expect(projects.updateClaudeSessionPath).toHaveBeenCalledWith('/new/custom/path')
    })
  })

  it('should show reset button for non-default path', async () => {
    vi.mocked(projects.getClaudeSessionPath).mockResolvedValue({
      path: '/custom/path/claude',
      is_default: false,
    })

    render(<ClaudePathSetting />)

    await waitFor(() => {
      expect(screen.getByText('/custom/path/claude')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByText('/custom/path/claude'))

    expect(screen.getByText('重設為預設')).toBeInTheDocument()
  })

  it('should not show reset button for default path', async () => {
    vi.mocked(projects.getClaudeSessionPath).mockResolvedValue({
      path: '/Users/test/.claude',
      is_default: true,
    })

    render(<ClaudePathSetting />)

    await waitFor(() => {
      expect(screen.getByText('/Users/test/.claude')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByText('/Users/test/.claude'))

    expect(screen.queryByText('重設為預設')).not.toBeInTheDocument()
  })

  it('should reset to default path', async () => {
    vi.mocked(projects.getClaudeSessionPath).mockResolvedValue({
      path: '/custom/path/claude',
      is_default: false,
    })
    vi.mocked(projects.updateClaudeSessionPath).mockResolvedValue('ok')

    render(<ClaudePathSetting />)

    await waitFor(() => {
      expect(screen.getByText('/custom/path/claude')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByText('/custom/path/claude'))

    vi.mocked(projects.getClaudeSessionPath).mockResolvedValue({
      path: '/Users/test/.claude',
      is_default: true,
    })

    fireEvent.click(screen.getByText('重設為預設'))

    await waitFor(() => {
      expect(projects.updateClaudeSessionPath).toHaveBeenCalledWith(null)
    })
  })

  it('should show error message on save failure', async () => {
    vi.mocked(projects.getClaudeSessionPath).mockResolvedValue({
      path: '/Users/test/.claude',
      is_default: true,
    })
    vi.mocked(projects.updateClaudeSessionPath).mockRejectedValue(
      new Error('Path is not a valid directory')
    )

    render(<ClaudePathSetting />)

    await waitFor(() => {
      expect(screen.getByText('/Users/test/.claude')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByText('/Users/test/.claude'))

    const input = screen.getByRole('textbox')
    fireEvent.change(input, { target: { value: '/invalid/path' } })

    fireEvent.click(screen.getByText('儲存'))

    await waitFor(() => {
      expect(screen.getByText('Error: Path is not a valid directory')).toBeInTheDocument()
    })
  })

  it('should save on Enter key press', async () => {
    vi.mocked(projects.getClaudeSessionPath).mockResolvedValue({
      path: '/Users/test/.claude',
      is_default: true,
    })
    vi.mocked(projects.updateClaudeSessionPath).mockResolvedValue('ok')

    render(<ClaudePathSetting />)

    await waitFor(() => {
      expect(screen.getByText('/Users/test/.claude')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByText('/Users/test/.claude'))

    const input = screen.getByRole('textbox')
    fireEvent.keyDown(input, { key: 'Enter' })

    await waitFor(() => {
      expect(projects.updateClaudeSessionPath).toHaveBeenCalled()
    })
  })

  it('should cancel on Escape key press', async () => {
    vi.mocked(projects.getClaudeSessionPath).mockResolvedValue({
      path: '/Users/test/.claude',
      is_default: true,
    })

    render(<ClaudePathSetting />)

    await waitFor(() => {
      expect(screen.getByText('/Users/test/.claude')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByText('/Users/test/.claude'))

    const input = screen.getByRole('textbox')
    fireEvent.keyDown(input, { key: 'Escape' })

    expect(screen.queryByRole('textbox')).not.toBeInTheDocument()
  })
})
