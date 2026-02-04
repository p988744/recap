import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, act, waitFor } from '@testing-library/react'
import { useCommitDiff } from './useCommitDiff'

// Mock the projects service
vi.mock('@/services', () => ({
  projects: {
    getCommitDiff: vi.fn(),
  },
}))

import { projects } from '@/services'

describe('useCommitDiff', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('should initialize with null diff and no loading/error', () => {
    const { result } = renderHook(() => useCommitDiff())

    expect(result.current.diff).toBeNull()
    expect(result.current.isLoading).toBe(false)
    expect(result.current.error).toBeNull()
  })

  it('should fetch diff successfully', async () => {
    const mockDiff = {
      hash: 'abc123',
      message: 'Test commit',
      author: 'Test Author <test@example.com>',
      date: '2026-01-30T10:00:00Z',
      files: [
        {
          path: 'test.ts',
          status: 'modified' as const,
          old_path: null,
          insertions: 10,
          deletions: 5,
        },
      ],
      diff_text: '+added line\n-deleted line',
      stats: {
        files_changed: 1,
        insertions: 10,
        deletions: 5,
      },
    }

    vi.mocked(projects.getCommitDiff).mockResolvedValue(mockDiff)

    const { result } = renderHook(() => useCommitDiff())

    await act(async () => {
      await result.current.fetchDiff('/path/to/project', 'abc123')
    })

    expect(result.current.diff).toEqual(mockDiff)
    expect(result.current.isLoading).toBe(false)
    expect(result.current.error).toBeNull()
    expect(projects.getCommitDiff).toHaveBeenCalledWith('/path/to/project', 'abc123')
  })

  it('should handle fetch error', async () => {
    const errorMessage = 'Failed to load diff'
    vi.mocked(projects.getCommitDiff).mockRejectedValue(new Error(errorMessage))

    const { result } = renderHook(() => useCommitDiff())

    await act(async () => {
      await result.current.fetchDiff('/path/to/project', 'abc123')
    })

    expect(result.current.diff).toBeNull()
    expect(result.current.isLoading).toBe(false)
    expect(result.current.error).toBe(errorMessage)
  })

  it('should clear diff', async () => {
    const mockDiff = {
      hash: 'abc123',
      message: 'Test commit',
      author: 'Test Author <test@example.com>',
      date: '2026-01-30T10:00:00Z',
      files: [],
      diff_text: null,
      stats: {
        files_changed: 0,
        insertions: 0,
        deletions: 0,
      },
    }

    vi.mocked(projects.getCommitDiff).mockResolvedValue(mockDiff)

    const { result } = renderHook(() => useCommitDiff())

    await act(async () => {
      await result.current.fetchDiff('/path/to/project', 'abc123')
    })

    expect(result.current.diff).toEqual(mockDiff)

    act(() => {
      result.current.clearDiff()
    })

    expect(result.current.diff).toBeNull()
    expect(result.current.error).toBeNull()
  })

  it('should set loading state during fetch', async () => {
    let resolvePromise: (value: unknown) => void
    const pendingPromise = new Promise((resolve) => {
      resolvePromise = resolve
    })

    vi.mocked(projects.getCommitDiff).mockReturnValue(pendingPromise as Promise<never>)

    const { result } = renderHook(() => useCommitDiff())

    act(() => {
      result.current.fetchDiff('/path/to/project', 'abc123')
    })

    // Loading should be true while waiting
    await waitFor(() => {
      expect(result.current.isLoading).toBe(true)
    })

    // Resolve the promise
    await act(async () => {
      resolvePromise!({
        hash: 'abc123',
        message: 'Test',
        author: 'Test',
        date: '2026-01-30',
        files: [],
        diff_text: null,
        stats: { files_changed: 0, insertions: 0, deletions: 0 },
      })
    })

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false)
    })
  })
})
