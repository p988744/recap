import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useClaudeCodeForm } from './useClaudeCodeForm'
import { claude } from '@/services'

vi.mock('@/services', () => ({
  sources: {
    addGitRepo: vi.fn(),
  },
  claude: {
    listSessions: vi.fn(),
    importSessions: vi.fn(),
  },
}))

// Mock localStorage
const localStorageMock = {
  getItem: vi.fn(),
  setItem: vi.fn(),
  clear: vi.fn(),
}
Object.defineProperty(window, 'localStorage', { value: localStorageMock })

describe('useClaudeCodeForm', () => {
  const mockSources = {
    mode: 'local',
    git_repos: [{ id: 'repo-1', path: '/home/user/project', name: 'project', valid: true }],
    claude_connected: true,
    claude_path: '/home/user/.claude',
  }

  const mockProjects = [
    {
      path: '/home/user/project-a',
      sessions: [
        { session_id: 'session-1', created_at: '2024-01-01T00:00:00Z', title: 'Session 1' },
        { session_id: 'session-2', created_at: '2024-01-02T00:00:00Z', title: 'Session 2' },
      ],
    },
    {
      path: '/home/user/project-b',
      sessions: [
        { session_id: 'session-3', created_at: '2024-01-03T00:00:00Z', title: 'Session 3' },
      ],
    },
  ]

  beforeEach(() => {
    vi.clearAllMocks()
    localStorageMock.getItem.mockReturnValue(null)
  })

  it('should initialize with default values', () => {
    const { result } = renderHook(() => useClaudeCodeForm())

    expect(result.current.projects).toEqual([])
    expect(result.current.loading).toBe(false)
    expect(result.current.selectedProjects.size).toBe(0)
    expect(result.current.importing).toBe(false)
    expect(result.current.selectedSessionCount).toBe(0)
  })

  it('should restore selected projects from localStorage', () => {
    localStorageMock.getItem.mockReturnValue(JSON.stringify(['/home/user/project-a']))
    const { result } = renderHook(() => useClaudeCodeForm())

    expect(result.current.selectedProjects.has('/home/user/project-a')).toBe(true)
  })

  it('should load sessions successfully', async () => {
    vi.mocked(claude.listSessions).mockResolvedValue(mockProjects)
    const setMessage = vi.fn()
    const refreshSources = vi.fn().mockResolvedValue(mockSources)
    const { result } = renderHook(() => useClaudeCodeForm())

    await act(async () => {
      await result.current.loadSessions(mockSources, setMessage, refreshSources)
    })

    expect(claude.listSessions).toHaveBeenCalled()
    expect(result.current.projects).toEqual(mockProjects)
    expect(result.current.expandedProjects.has('/home/user/project-a')).toBe(true)
  })

  it('should handle load sessions error', async () => {
    vi.mocked(claude.listSessions).mockRejectedValue(new Error('Failed to list sessions'))
    const setMessage = vi.fn()
    const refreshSources = vi.fn()
    const { result } = renderHook(() => useClaudeCodeForm())

    await act(async () => {
      await result.current.loadSessions(mockSources, setMessage, refreshSources)
    })

    expect(setMessage).toHaveBeenCalledWith({ type: 'error', text: 'Failed to list sessions' })
  })

  it('should toggle expand project', async () => {
    vi.mocked(claude.listSessions).mockResolvedValue(mockProjects)
    const { result } = renderHook(() => useClaudeCodeForm())

    await act(async () => {
      await result.current.loadSessions(mockSources, vi.fn(), vi.fn().mockResolvedValue(mockSources))
    })

    // First project should be expanded by default
    expect(result.current.expandedProjects.has('/home/user/project-a')).toBe(true)

    // Collapse it
    act(() => {
      result.current.toggleExpandProject('/home/user/project-a')
    })
    expect(result.current.expandedProjects.has('/home/user/project-a')).toBe(false)

    // Expand it again
    act(() => {
      result.current.toggleExpandProject('/home/user/project-a')
    })
    expect(result.current.expandedProjects.has('/home/user/project-a')).toBe(true)
  })

  it('should toggle project selection', () => {
    const { result } = renderHook(() => useClaudeCodeForm())

    expect(result.current.selectedProjects.has('/home/user/project-a')).toBe(false)

    act(() => {
      result.current.toggleProjectSelection('/home/user/project-a')
    })
    expect(result.current.selectedProjects.has('/home/user/project-a')).toBe(true)

    act(() => {
      result.current.toggleProjectSelection('/home/user/project-a')
    })
    expect(result.current.selectedProjects.has('/home/user/project-a')).toBe(false)
  })

  it('should select all projects', async () => {
    vi.mocked(claude.listSessions).mockResolvedValue(mockProjects)
    const { result } = renderHook(() => useClaudeCodeForm())

    await act(async () => {
      await result.current.loadSessions(mockSources, vi.fn(), vi.fn().mockResolvedValue(mockSources))
    })

    act(() => {
      result.current.selectAllProjects()
    })

    expect(result.current.selectedProjects.size).toBe(2)
    expect(result.current.selectedProjects.has('/home/user/project-a')).toBe(true)
    expect(result.current.selectedProjects.has('/home/user/project-b')).toBe(true)
  })

  it('should clear selection', () => {
    localStorageMock.getItem.mockReturnValue(JSON.stringify(['/home/user/project-a', '/home/user/project-b']))
    const { result } = renderHook(() => useClaudeCodeForm())

    expect(result.current.selectedProjects.size).toBe(2)

    act(() => {
      result.current.clearSelection()
    })

    expect(result.current.selectedProjects.size).toBe(0)
  })

  it('should calculate selected session count', async () => {
    vi.mocked(claude.listSessions).mockResolvedValue(mockProjects)
    localStorageMock.getItem.mockReturnValue(JSON.stringify(['/home/user/project-a']))
    const { result } = renderHook(() => useClaudeCodeForm())

    await act(async () => {
      await result.current.loadSessions(mockSources, vi.fn(), vi.fn().mockResolvedValue(mockSources))
    })

    // project-a has 2 sessions
    expect(result.current.selectedSessionCount).toBe(2)
  })

  it('should import sessions successfully', async () => {
    vi.mocked(claude.listSessions).mockResolvedValue(mockProjects)
    vi.mocked(claude.importSessions).mockResolvedValue({ imported: 2, work_items_created: 3 })
    localStorageMock.getItem.mockReturnValue(JSON.stringify(['/home/user/project-a']))
    const setMessage = vi.fn()
    const { result } = renderHook(() => useClaudeCodeForm())

    await act(async () => {
      await result.current.loadSessions(mockSources, vi.fn(), vi.fn().mockResolvedValue(mockSources))
    })

    await act(async () => {
      await result.current.handleImport(setMessage)
    })

    expect(claude.importSessions).toHaveBeenCalledWith({
      session_ids: ['session-1', 'session-2'],
    })
    expect(setMessage).toHaveBeenCalledWith({
      type: 'success',
      text: '已匯入 2 個 session，建立 3 個工作項目',
    })
  })

  it('should not import when no projects selected', async () => {
    const setMessage = vi.fn()
    const { result } = renderHook(() => useClaudeCodeForm())

    await act(async () => {
      await result.current.handleImport(setMessage)
    })

    expect(claude.importSessions).not.toHaveBeenCalled()
  })

  it('should handle import error', async () => {
    vi.mocked(claude.listSessions).mockResolvedValue(mockProjects)
    vi.mocked(claude.importSessions).mockRejectedValue(new Error('Import failed'))
    localStorageMock.getItem.mockReturnValue(JSON.stringify(['/home/user/project-a']))
    const setMessage = vi.fn()
    const { result } = renderHook(() => useClaudeCodeForm())

    await act(async () => {
      await result.current.loadSessions(mockSources, vi.fn(), vi.fn().mockResolvedValue(mockSources))
    })

    await act(async () => {
      await result.current.handleImport(setMessage)
    })

    expect(setMessage).toHaveBeenCalledWith({ type: 'error', text: 'Import failed' })
  })
})
