import { describe, it, expect, beforeEach } from 'vitest'
import {
  mockInvoke,
  mockCommandValue,
  mockCommandError,
  resetTauriMock,
} from '@/test/mocks/tauri'
import * as antigravity from './antigravity'

// Mock fixtures
const mockAntigravityProject = {
  path: '/home/user/projects/test-project',
  name: 'test-project',
  sessions: [
    {
      session_id: 'ag-session-1',
      task_summary: 'Implement user authentication',
      walkthrough_summary: null,
      cwd: '/home/user/projects/test-project',
      git_branch: 'main',
      first_message: 'Help me add login functionality',
      message_count: 15,
      first_timestamp: '2024-01-15T09:00:00+08:00',
      last_timestamp: '2024-01-15T12:00:00+08:00',
      file_path: '/home/user/.gemini/antigravity/test-project/session-1.jsonl',
      file_size: 1024,
      artifact_count: 5,
      tool_usage: [
        { tool_name: 'read_file', count: 10, details: ['src/auth.rs'] },
        { tool_name: 'edit_file', count: 5, details: ['src/login.rs'] },
      ],
      files_modified: ['src/auth.rs', 'src/login.rs'],
      commands_run: ['cargo test', 'cargo build'],
      user_messages: ['Help me add login functionality'],
    },
    {
      session_id: 'ag-session-2',
      task_summary: 'Fix authentication bug',
      walkthrough_summary: null,
      cwd: '/home/user/projects/test-project',
      git_branch: 'fix/auth',
      first_message: 'The login is not working correctly',
      message_count: 8,
      first_timestamp: '2024-01-15T14:00:00+08:00',
      last_timestamp: '2024-01-15T16:00:00+08:00',
      file_path: '/home/user/.gemini/antigravity/test-project/session-2.jsonl',
      file_size: 512,
      artifact_count: 2,
      tool_usage: [{ tool_name: 'edit_file', count: 3, details: ['src/auth.rs'] }],
      files_modified: ['src/auth.rs'],
      commands_run: ['cargo test'],
      user_messages: ['The login is not working correctly'],
    },
  ],
}

const mockAntigravityProjects = [
  mockAntigravityProject,
  {
    path: '/home/user/projects/another-project',
    name: 'another-project',
    sessions: [],
  },
]

const mockSyncResult = {
  sessions_processed: 5,
  sessions_skipped: 0,
  work_items_created: 3,
  work_items_updated: 2,
}

describe('antigravity service', () => {
  beforeEach(() => {
    resetTauriMock()
    localStorage.setItem('recap_auth_token', 'test-token')
  })

  describe('checkInstalled', () => {
    it('should return true when Antigravity is installed', async () => {
      mockCommandValue('check_antigravity_installed', true)

      const result = await antigravity.checkInstalled()

      expect(result).toBe(true)
      expect(mockInvoke).toHaveBeenCalledWith('check_antigravity_installed', { token: 'test-token' })
    })

    it('should return false when Antigravity is not installed', async () => {
      mockCommandValue('check_antigravity_installed', false)

      const result = await antigravity.checkInstalled()

      expect(result).toBe(false)
    })

    it('should throw on error', async () => {
      mockCommandError('check_antigravity_installed', 'Failed to check Antigravity installation')

      await expect(antigravity.checkInstalled()).rejects.toThrow('Failed to check Antigravity installation')
    })
  })

  describe('listProjects', () => {
    it('should list all Antigravity projects', async () => {
      mockCommandValue('list_antigravity_sessions', mockAntigravityProjects)

      const result = await antigravity.listProjects()

      expect(result).toHaveLength(2)
      expect(result[0].name).toBe('test-project')
      expect(result[0].sessions).toHaveLength(2)
      expect(mockInvoke).toHaveBeenCalledWith('list_antigravity_sessions', { token: 'test-token' })
    })

    it('should return empty list when no projects found', async () => {
      mockCommandValue('list_antigravity_sessions', [])

      const result = await antigravity.listProjects()

      expect(result).toHaveLength(0)
    })

    it('should throw on error', async () => {
      mockCommandError('list_antigravity_sessions', 'Antigravity directory not found')

      await expect(antigravity.listProjects()).rejects.toThrow('Antigravity directory not found')
    })

    it('should return sessions with correct metadata', async () => {
      mockCommandValue('list_antigravity_sessions', [mockAntigravityProject])

      const result = await antigravity.listProjects()

      expect(result[0].sessions[0].session_id).toBe('ag-session-1')
      expect(result[0].sessions[0].task_summary).toBe('Implement user authentication')
      expect(result[0].sessions[0].artifact_count).toBe(5)
      expect(result[0].sessions[0].tool_usage).toHaveLength(2)
    })
  })

  describe('sync', () => {
    it('should sync selected projects to work items', async () => {
      mockCommandValue('sync_antigravity_projects', mockSyncResult)

      const request = {
        project_paths: ['/home/user/projects/test-project'],
      }
      const result = await antigravity.sync(request)

      expect(result.sessions_processed).toBe(5)
      expect(result.work_items_created).toBe(3)
      expect(result.work_items_updated).toBe(2)
      expect(mockInvoke).toHaveBeenCalledWith('sync_antigravity_projects', {
        token: 'test-token',
        request,
      })
    })

    it('should handle skipped sessions', async () => {
      mockCommandValue('sync_antigravity_projects', {
        sessions_processed: 3,
        sessions_skipped: 2,
        work_items_created: 2,
        work_items_updated: 1,
      })

      const request = {
        project_paths: ['/home/user/projects/test-project'],
      }
      const result = await antigravity.sync(request)

      expect(result.sessions_processed).toBe(3)
      expect(result.sessions_skipped).toBe(2)
    })

    it('should throw on invalid project path', async () => {
      mockCommandError('sync_antigravity_projects', 'Project path not found')

      const request = {
        project_paths: ['/invalid/path'],
      }

      await expect(antigravity.sync(request)).rejects.toThrow('Project path not found')
    })

    it('should sync multiple projects', async () => {
      mockCommandValue('sync_antigravity_projects', {
        sessions_processed: 10,
        sessions_skipped: 0,
        work_items_created: 7,
        work_items_updated: 3,
      })

      const request = {
        project_paths: [
          '/home/user/projects/test-project',
          '/home/user/projects/another-project',
        ],
      }
      const result = await antigravity.sync(request)

      expect(result.sessions_processed).toBe(10)
      expect(result.work_items_created).toBe(7)
    })
  })
})
