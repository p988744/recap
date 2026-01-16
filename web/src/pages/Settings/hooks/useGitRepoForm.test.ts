import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useGitRepoForm } from './useGitRepoForm'
import { sources as sourcesService } from '@/services'

vi.mock('@/services', () => ({
  sources: {
    addGitRepo: vi.fn(),
    removeGitRepo: vi.fn(),
  },
}))

describe('useGitRepoForm', () => {
  const mockSourcesResponse = {
    mode: 'git',
    git_repos: [
      { id: 'repo-1', path: '/home/user/project-a', name: 'project-a', valid: true },
    ],
    claude_connected: false,
    claude_path: '',
  }

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('should initialize with empty path', () => {
    const { result } = renderHook(() => useGitRepoForm())

    expect(result.current.newRepoPath).toBe('')
    expect(result.current.adding).toBe(false)
  })

  it('should update repo path', () => {
    const { result } = renderHook(() => useGitRepoForm())

    act(() => {
      result.current.setNewRepoPath('/home/user/new-repo')
    })

    expect(result.current.newRepoPath).toBe('/home/user/new-repo')
  })

  it('should not add empty path', async () => {
    const setMessage = vi.fn()
    const refreshSources = vi.fn()
    const { result } = renderHook(() => useGitRepoForm())

    await act(async () => {
      await result.current.handleAdd(setMessage, refreshSources)
    })

    expect(sourcesService.addGitRepo).not.toHaveBeenCalled()
  })

  it('should add git repo successfully', async () => {
    vi.mocked(sourcesService.addGitRepo).mockResolvedValue({
      success: true,
      message: 'Added',
      repo: { id: 'repo-2', path: '/home/user/new-repo', name: 'new-repo', valid: true },
    })
    const setMessage = vi.fn()
    const refreshSources = vi.fn().mockResolvedValue(mockSourcesResponse)
    const { result } = renderHook(() => useGitRepoForm())

    act(() => {
      result.current.setNewRepoPath('/home/user/new-repo')
    })

    await act(async () => {
      await result.current.handleAdd(setMessage, refreshSources)
    })

    expect(sourcesService.addGitRepo).toHaveBeenCalledWith('/home/user/new-repo')
    expect(refreshSources).toHaveBeenCalled()
    expect(setMessage).toHaveBeenCalledWith({ type: 'success', text: '已新增 Git 倉庫' })
    expect(result.current.newRepoPath).toBe('')
  })

  it('should handle add error', async () => {
    vi.mocked(sourcesService.addGitRepo).mockRejectedValue(new Error('Not a git repository'))
    const setMessage = vi.fn()
    const refreshSources = vi.fn()
    const { result } = renderHook(() => useGitRepoForm())

    act(() => {
      result.current.setNewRepoPath('/invalid/path')
    })

    await act(async () => {
      await result.current.handleAdd(setMessage, refreshSources)
    })

    expect(setMessage).toHaveBeenCalledWith({ type: 'error', text: 'Not a git repository' })
  })

  it('should remove git repo successfully', async () => {
    vi.mocked(sourcesService.removeGitRepo).mockResolvedValue({ success: true, message: 'Removed' })
    const setMessage = vi.fn()
    const refreshSources = vi.fn().mockResolvedValue(mockSourcesResponse)
    const { result } = renderHook(() => useGitRepoForm())

    await act(async () => {
      await result.current.handleRemove('repo-1', setMessage, refreshSources)
    })

    expect(sourcesService.removeGitRepo).toHaveBeenCalledWith('repo-1')
    expect(refreshSources).toHaveBeenCalled()
    expect(setMessage).toHaveBeenCalledWith({ type: 'success', text: '已移除 Git 倉庫' })
  })

  it('should handle remove error', async () => {
    vi.mocked(sourcesService.removeGitRepo).mockRejectedValue(new Error('Remove failed'))
    const setMessage = vi.fn()
    const refreshSources = vi.fn()
    const { result } = renderHook(() => useGitRepoForm())

    await act(async () => {
      await result.current.handleRemove('repo-1', setMessage, refreshSources)
    })

    expect(setMessage).toHaveBeenCalledWith({ type: 'error', text: 'Remove failed' })
  })
})
