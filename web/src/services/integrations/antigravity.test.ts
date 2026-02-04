import { describe, it, expect, beforeEach } from 'vitest'
import {
  mockInvoke,
  mockCommandValue,
  mockCommandError,
  resetTauriMock,
} from '@/test/mocks/tauri'
import * as antigravity from './antigravity'

// Mock fixtures - updated to match new HTTP API response structure
const mockAntigravityProject = {
  path: '/home/user/projects/test-project',
  name: 'test-project',
  sessions: [
    {
      session_id: 'ag-session-1',
      summary: 'Implement user authentication',
      cwd: '/home/user/projects/test-project',
      git_branch: 'main',
      git_repo: 'user/test-project',
      step_count: 150,
      first_timestamp: '2024-01-15T09:00:00+08:00',
      last_timestamp: '2024-01-15T12:00:00+08:00',
      status: 'CASCADE_RUN_STATUS_IDLE',
    },
    {
      session_id: 'ag-session-2',
      summary: 'Fix authentication bug',
      cwd: '/home/user/projects/test-project',
      git_branch: 'fix/auth',
      git_repo: 'user/test-project',
      step_count: 80,
      first_timestamp: '2024-01-15T14:00:00+08:00',
      last_timestamp: '2024-01-15T16:00:00+08:00',
      status: 'CASCADE_RUN_STATUS_IDLE',
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
    it('should return true when Antigravity is running', async () => {
      mockCommandValue('check_antigravity_installed', true)

      const result = await antigravity.checkInstalled()

      expect(result).toBe(true)
      expect(mockInvoke).toHaveBeenCalledWith('check_antigravity_installed', { token: 'test-token' })
    })

    it('should return false when Antigravity is not running', async () => {
      mockCommandValue('check_antigravity_installed', false)

      const result = await antigravity.checkInstalled()

      expect(result).toBe(false)
    })

    it('should throw on error', async () => {
      mockCommandError('check_antigravity_installed', 'Failed to check Antigravity process')

      await expect(antigravity.checkInstalled()).rejects.toThrow('Failed to check Antigravity process')
    })
  })

  describe('listProjects', () => {
    it('should list all Antigravity projects from API', async () => {
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

    it('should throw when Antigravity is not running', async () => {
      mockCommandError('list_antigravity_sessions', 'Antigravity is not running. Please start the Antigravity app.')

      await expect(antigravity.listProjects()).rejects.toThrow('Antigravity is not running')
    })

    it('should return sessions with correct metadata', async () => {
      mockCommandValue('list_antigravity_sessions', [mockAntigravityProject])

      const result = await antigravity.listProjects()

      expect(result[0].sessions[0].session_id).toBe('ag-session-1')
      expect(result[0].sessions[0].summary).toBe('Implement user authentication')
      expect(result[0].sessions[0].step_count).toBe(150)
      expect(result[0].sessions[0].git_branch).toBe('main')
      expect(result[0].sessions[0].git_repo).toBe('user/test-project')
      expect(result[0].sessions[0].status).toBe('CASCADE_RUN_STATUS_IDLE')
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

    it('should throw when Antigravity is not running', async () => {
      mockCommandError('sync_antigravity_projects', 'Antigravity is not running. Please start the Antigravity app.')

      const request = {
        project_paths: ['/invalid/path'],
      }

      await expect(antigravity.sync(request)).rejects.toThrow('Antigravity is not running')
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
