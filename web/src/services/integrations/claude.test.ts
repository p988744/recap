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
  imported_count: 2,
  skipped_count: 0,
  message: 'Successfully imported 2 sessions',
}

const mockSummarizeResult = {
  session_id: 'session-1',
  summary: 'Implemented user authentication feature with JWT tokens',
  key_points: ['Added login endpoint', 'Implemented JWT validation', 'Added logout functionality'],
}

const mockSyncResult = {
  synced_count: 5,
  created_count: 3,
  updated_count: 2,
  message: 'Synced 5 work items from Claude Code',
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
      expect(result[0].total_hours).toBe(6.0)
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
        project_paths: ['/home/user/.claude/projects/test-project'],
        session_ids: ['session-1', 'session-2'],
      }
      const result = await claude.importSessions(request)

      expect(result.imported_count).toBe(2)
      expect(result.skipped_count).toBe(0)
      expect(mockInvoke).toHaveBeenCalledWith('import_claude_sessions', {
        token: 'test-token',
        request,
      })
    })

    it('should handle partial import with skipped sessions', async () => {
      mockCommandValue('import_claude_sessions', {
        imported_count: 1,
        skipped_count: 1,
        message: 'Imported 1 session, skipped 1 duplicate',
      })

      const request = {
        project_paths: ['/home/user/.claude/projects/test-project'],
        session_ids: ['session-1', 'session-2'],
      }
      const result = await claude.importSessions(request)

      expect(result.imported_count).toBe(1)
      expect(result.skipped_count).toBe(1)
    })

    it('should throw on invalid session', async () => {
      mockCommandError('import_claude_sessions', 'Session not found')

      const request = {
        project_paths: ['/invalid/path'],
        session_ids: ['invalid-session'],
      }

      await expect(claude.importSessions(request)).rejects.toThrow('Session not found')
    })
  })

  describe('summarizeSession', () => {
    it('should summarize a session using LLM', async () => {
      mockCommandValue('summarize_claude_session', mockSummarizeResult)

      const request = {
        project_path: '/home/user/.claude/projects/test-project',
        session_id: 'session-1',
      }
      const result = await claude.summarizeSession(request)

      expect(result.session_id).toBe('session-1')
      expect(result.summary).toContain('authentication')
      expect(result.key_points).toHaveLength(3)
      expect(mockInvoke).toHaveBeenCalledWith('summarize_claude_session', {
        token: 'test-token',
        request,
      })
    })

    it('should throw when LLM is not configured', async () => {
      mockCommandError('summarize_claude_session', 'LLM API key not configured')

      const request = {
        project_path: '/home/user/.claude/projects/test-project',
        session_id: 'session-1',
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

      expect(result.synced_count).toBe(5)
      expect(result.created_count).toBe(3)
      expect(result.updated_count).toBe(2)
      expect(mockInvoke).toHaveBeenCalledWith('sync_claude_projects', {
        token: 'test-token',
        request,
      })
    })

    it('should sync with date range', async () => {
      mockCommandValue('sync_claude_projects', mockSyncResult)

      const request = {
        project_paths: ['/home/user/.claude/projects/test-project'],
        start_date: '2024-01-01',
        end_date: '2024-01-31',
      }
      const result = await claude.syncProjects(request)

      expect(result.synced_count).toBe(5)
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
