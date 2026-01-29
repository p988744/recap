import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { AntigravityPathSetting } from './AntigravityPathSetting'
import { projects } from '@/services'

vi.mock('@/services', () => ({
  projects: {
    getAntigravitySessionPath: vi.fn(),
    updateAntigravitySessionPath: vi.fn(),
  },
}))

describe('AntigravityPathSetting', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('should render with default path', async () => {
    vi.mocked(projects.getAntigravitySessionPath).mockResolvedValue({
      path: '/Users/test/.gemini/antigravity',
      is_default: true,
    })

    render(<AntigravityPathSetting />)

    await waitFor(() => {
      expect(screen.getByText('/Users/test/.gemini/antigravity')).toBeInTheDocument()
    })

    expect(screen.getByText('預設')).toBeInTheDocument()
    expect(screen.getByText('Antigravity Session 路徑')).toBeInTheDocument()
  })

  it('should render with custom path (non-default)', async () => {
    vi.mocked(projects.getAntigravitySessionPath).mockResolvedValue({
      path: '/custom/path/antigravity',
      is_default: false,
    })

    render(<AntigravityPathSetting />)

    await waitFor(() => {
      expect(screen.getByText('/custom/path/antigravity')).toBeInTheDocument()
    })

    // Should not show "預設" badge
    expect(screen.queryByText('預設')).not.toBeInTheDocument()
  })

  it('should enter edit mode when path is clicked', async () => {
    vi.mocked(projects.getAntigravitySessionPath).mockResolvedValue({
      path: '/Users/test/.gemini/antigravity',
      is_default: true,
    })

    render(<AntigravityPathSetting />)

    await waitFor(() => {
      expect(screen.getByText('/Users/test/.gemini/antigravity')).toBeInTheDocument()
    })

    // Click the path to enter edit mode
    fireEvent.click(screen.getByText('/Users/test/.gemini/antigravity'))

    // Should show input with current value
    const input = screen.getByRole('textbox')
    expect(input).toBeInTheDocument()
    expect(input).toHaveValue('/Users/test/.gemini/antigravity')

    // Should show save and cancel buttons
    expect(screen.getByText('儲存')).toBeInTheDocument()
    expect(screen.getByText('取消')).toBeInTheDocument()
  })

  it('should cancel edit mode', async () => {
    vi.mocked(projects.getAntigravitySessionPath).mockResolvedValue({
      path: '/Users/test/.gemini/antigravity',
      is_default: true,
    })

    render(<AntigravityPathSetting />)

    await waitFor(() => {
      expect(screen.getByText('/Users/test/.gemini/antigravity')).toBeInTheDocument()
    })

    // Enter edit mode
    fireEvent.click(screen.getByText('/Users/test/.gemini/antigravity'))

    // Click cancel
    fireEvent.click(screen.getByText('取消'))

    // Should exit edit mode
    expect(screen.queryByRole('textbox')).not.toBeInTheDocument()
    expect(screen.getByText('/Users/test/.gemini/antigravity')).toBeInTheDocument()
  })

  it('should save new path', async () => {
    vi.mocked(projects.getAntigravitySessionPath).mockResolvedValue({
      path: '/Users/test/.gemini/antigravity',
      is_default: true,
    })
    vi.mocked(projects.updateAntigravitySessionPath).mockResolvedValue('ok')

    render(<AntigravityPathSetting />)

    await waitFor(() => {
      expect(screen.getByText('/Users/test/.gemini/antigravity')).toBeInTheDocument()
    })

    // Enter edit mode
    fireEvent.click(screen.getByText('/Users/test/.gemini/antigravity'))

    // Change value
    const input = screen.getByRole('textbox')
    fireEvent.change(input, { target: { value: '/new/custom/path' } })

    // Update mock to return new path
    vi.mocked(projects.getAntigravitySessionPath).mockResolvedValue({
      path: '/new/custom/path',
      is_default: false,
    })

    // Save
    fireEvent.click(screen.getByText('儲存'))

    await waitFor(() => {
      expect(projects.updateAntigravitySessionPath).toHaveBeenCalledWith('/new/custom/path')
    })
  })

  it('should show reset button for non-default path', async () => {
    vi.mocked(projects.getAntigravitySessionPath).mockResolvedValue({
      path: '/custom/path/antigravity',
      is_default: false,
    })

    render(<AntigravityPathSetting />)

    await waitFor(() => {
      expect(screen.getByText('/custom/path/antigravity')).toBeInTheDocument()
    })

    // Enter edit mode
    fireEvent.click(screen.getByText('/custom/path/antigravity'))

    // Should show reset button
    expect(screen.getByText('重設為預設')).toBeInTheDocument()
  })

  it('should not show reset button for default path', async () => {
    vi.mocked(projects.getAntigravitySessionPath).mockResolvedValue({
      path: '/Users/test/.gemini/antigravity',
      is_default: true,
    })

    render(<AntigravityPathSetting />)

    await waitFor(() => {
      expect(screen.getByText('/Users/test/.gemini/antigravity')).toBeInTheDocument()
    })

    // Enter edit mode
    fireEvent.click(screen.getByText('/Users/test/.gemini/antigravity'))

    // Should NOT show reset button
    expect(screen.queryByText('重設為預設')).not.toBeInTheDocument()
  })

  it('should reset to default path', async () => {
    vi.mocked(projects.getAntigravitySessionPath).mockResolvedValue({
      path: '/custom/path/antigravity',
      is_default: false,
    })
    vi.mocked(projects.updateAntigravitySessionPath).mockResolvedValue('ok')

    render(<AntigravityPathSetting />)

    await waitFor(() => {
      expect(screen.getByText('/custom/path/antigravity')).toBeInTheDocument()
    })

    // Enter edit mode
    fireEvent.click(screen.getByText('/custom/path/antigravity'))

    // Update mock to return default path after reset
    vi.mocked(projects.getAntigravitySessionPath).mockResolvedValue({
      path: '/Users/test/.gemini/antigravity',
      is_default: true,
    })

    // Click reset
    fireEvent.click(screen.getByText('重設為預設'))

    await waitFor(() => {
      expect(projects.updateAntigravitySessionPath).toHaveBeenCalledWith(null)
    })
  })

  it('should show error message on save failure', async () => {
    vi.mocked(projects.getAntigravitySessionPath).mockResolvedValue({
      path: '/Users/test/.gemini/antigravity',
      is_default: true,
    })
    vi.mocked(projects.updateAntigravitySessionPath).mockRejectedValue(
      new Error('Path is not a valid directory')
    )

    render(<AntigravityPathSetting />)

    await waitFor(() => {
      expect(screen.getByText('/Users/test/.gemini/antigravity')).toBeInTheDocument()
    })

    // Enter edit mode
    fireEvent.click(screen.getByText('/Users/test/.gemini/antigravity'))

    // Change to invalid path
    const input = screen.getByRole('textbox')
    fireEvent.change(input, { target: { value: '/invalid/path' } })

    // Save
    fireEvent.click(screen.getByText('儲存'))

    await waitFor(() => {
      expect(screen.getByText('Error: Path is not a valid directory')).toBeInTheDocument()
    })
  })

  it('should save on Enter key press', async () => {
    vi.mocked(projects.getAntigravitySessionPath).mockResolvedValue({
      path: '/Users/test/.gemini/antigravity',
      is_default: true,
    })
    vi.mocked(projects.updateAntigravitySessionPath).mockResolvedValue('ok')

    render(<AntigravityPathSetting />)

    await waitFor(() => {
      expect(screen.getByText('/Users/test/.gemini/antigravity')).toBeInTheDocument()
    })

    // Enter edit mode
    fireEvent.click(screen.getByText('/Users/test/.gemini/antigravity'))

    // Press Enter
    const input = screen.getByRole('textbox')
    fireEvent.keyDown(input, { key: 'Enter' })

    await waitFor(() => {
      expect(projects.updateAntigravitySessionPath).toHaveBeenCalled()
    })
  })

  it('should cancel on Escape key press', async () => {
    vi.mocked(projects.getAntigravitySessionPath).mockResolvedValue({
      path: '/Users/test/.gemini/antigravity',
      is_default: true,
    })

    render(<AntigravityPathSetting />)

    await waitFor(() => {
      expect(screen.getByText('/Users/test/.gemini/antigravity')).toBeInTheDocument()
    })

    // Enter edit mode
    fireEvent.click(screen.getByText('/Users/test/.gemini/antigravity'))

    // Press Escape
    const input = screen.getByRole('textbox')
    fireEvent.keyDown(input, { key: 'Escape' })

    // Should exit edit mode
    expect(screen.queryByRole('textbox')).not.toBeInTheDocument()
  })
})
