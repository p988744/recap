import { describe, it, expect, beforeEach } from 'vitest'
import {
  mockInvoke,
  mockCommandValue,
  mockCommandError,
  resetTauriMock,
} from '@/test/mocks/tauri'
import * as claude from './claude'

// Mock fixtures
const mockClaudeProject = {
  path: '/home/user/.claude/projects/test-project',
  name: 'test-project',
  sessions: [
    {
      id: 'session-1',
      date: '2024-01-15',
      start_time: '2024-01-15T09:00:00Z',
      end_time: '2024-01-15T12:00:00Z',
      hours: 3.0,
      title: 'Feature development',
    },
    {
      id: 'session-2',
      date: '2024-01-15',
      start_time: '2024-01-15T14:00:00Z',
      end_time: '2024-01-15T17:00:00Z',
      hours: 3.0,
      title: 'Bug fixes',
    },
  ],
  total_hours: 6.0,
  total_sessions: 2,
}

const mockClaudeProjects = [
  mockClaudeProject,
  {
    path: '/home/user/.claude/projects/another-project',
    name: 'another-project',
    sessions: [],
    total_hours: 0,
    total_sessions: 0,
  },
]

const mockImportResult = {
  imported: 2,
  work_items_created: 2,
}

const mockSummarizeResult = {
  summary: 'Implemented user authentication feature with JWT tokens',
  success: true,
}

const mockSyncResult = {
  sessions_processed: 5,
  sessions_skipped: 0,
  work_items_created: 3,
  work_items_updated: 2,
}

describe('claude service', () => {
  beforeEach(() => {
    resetTauriMock()
    localStorage.setItem('recap_auth_token', 'test-token')
  })

  describe('listSessions', () => {
    it('should list all Claude Code sessions', async () => {
      mockCommandValue('list_claude_sessions', mockClaudeProjects)

      const result = await claude.listSessions()

      expect(result).toHaveLength(2)
      expect(result[0].name).toBe('test-project')
      expect(result[0].sessions).toHaveLength(2)
      expect(mockInvoke).toHaveBeenCalledWith('list_claude_sessions', { token: 'test-token' })
    })

    it('should return empty list when no sessions found', async () => {
      mockCommandValue('list_claude_sessions', [])

      const result = await claude.listSessions()

      expect(result).toHaveLength(0)
    })

    it('should throw on error', async () => {
      mockCommandError('list_claude_sessions', 'Claude projects directory not found')

      await expect(claude.listSessions()).rejects.toThrow('Claude projects directory not found')
    })
  })

  describe('importSessions', () => {
    it('should import selected sessions as work items', async () => {
      mockCommandValue('import_claude_sessions', mockImportResult)

      const request = {
        session_ids: ['session-1', 'session-2'],
      }
      const result = await claude.importSessions(request)

      expect(result.imported).toBe(2)
      expect(result.work_items_created).toBe(2)
      expect(mockInvoke).toHaveBeenCalledWith('import_claude_sessions', {
        token: 'test-token',
        request,
      })
    })

    it('should handle partial import with skipped sessions', async () => {
      mockCommandValue('import_claude_sessions', {
        imported: 1,
        work_items_created: 1,
      })

      const request = {
        session_ids: ['session-1', 'session-2'],
      }
      const result = await claude.importSessions(request)

      expect(result.imported).toBe(1)
      expect(result.work_items_created).toBe(1)
    })

    it('should throw on invalid session', async () => {
      mockCommandError('import_claude_sessions', 'Session not found')

      const request = {
        session_ids: ['invalid-session'],
      }

      await expect(claude.importSessions(request)).rejects.toThrow('Session not found')
    })
  })

  describe('summarizeSession', () => {
    it('should summarize a session using LLM', async () => {
      mockCommandValue('summarize_claude_session', mockSummarizeResult)

      const request = {
        session_file_path: '/home/user/.claude/projects/test-project/session-1.jsonl',
      }
      const result = await claude.summarizeSession(request)

      expect(result.success).toBe(true)
      expect(result.summary).toContain('authentication')
      expect(mockInvoke).toHaveBeenCalledWith('summarize_claude_session', {
        token: 'test-token',
        request,
      })
    })

    it('should throw when LLM is not configured', async () => {
      mockCommandError('summarize_claude_session', 'LLM API key not configured')

      const request = {
        session_file_path: '/home/user/.claude/projects/test-project/session-1.jsonl',
      }

      await expect(claude.summarizeSession(request)).rejects.toThrow('LLM API key not configured')
    })
  })

  describe('syncProjects', () => {
    it('should sync selected projects to work items', async () => {
      mockCommandValue('sync_claude_projects', mockSyncResult)

      const request = {
        project_paths: ['/home/user/.claude/projects/test-project'],
      }
      const result = await claude.syncProjects(request)

      expect(result.sessions_processed).toBe(5)
      expect(result.work_items_created).toBe(3)
      expect(result.work_items_updated).toBe(2)
      expect(mockInvoke).toHaveBeenCalledWith('sync_claude_projects', {
        token: 'test-token',
        request,
      })
    })

    it('should sync with date range', async () => {
      mockCommandValue('sync_claude_projects', mockSyncResult)

      const request = {
        project_paths: ['/home/user/.claude/projects/test-project'],
      }
      const result = await claude.syncProjects(request)

      expect(result.sessions_processed).toBe(5)
    })

    it('should throw on invalid project path', async () => {
      mockCommandError('sync_claude_projects', 'Project path not found')

      const request = {
        project_paths: ['/invalid/path'],
      }

      await expect(claude.syncProjects(request)).rejects.toThrow('Project path not found')
    })
  })
})
