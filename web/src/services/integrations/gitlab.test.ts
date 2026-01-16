import { describe, it, expect, beforeEach } from 'vitest'
import {
  mockInvoke,
  mockCommandValue,
  mockCommandError,
  resetTauriMock,
} from '@/test/mocks/tauri'
import * as gitlab from './gitlab'

// Mock fixtures
const mockGitLabStatus = {
  configured: true,
  gitlab_url: 'https://gitlab.example.com',
}

const mockGitLabProject = {
  id: 'project-1',
  user_id: 'user-1',
  gitlab_project_id: 123,
  name: 'Test Project',
  path_with_namespace: 'team/test-project',
  gitlab_url: 'https://gitlab.example.com',
  default_branch: 'main',
  enabled: true,
  created_at: '2024-01-01T00:00:00Z',
}

const mockGitLabProjects = [
  mockGitLabProject,
  {
    id: 'project-2',
    user_id: 'user-1',
    gitlab_project_id: 456,
    name: 'Another Project',
    path_with_namespace: 'team/another-project',
    gitlab_url: 'https://gitlab.example.com',
    default_branch: 'main',
    enabled: true,
    created_at: '2024-01-01T00:00:00Z',
  },
]

const mockGitLabProjectInfo = {
  id: 789,
  name: 'Search Result',
  path_with_namespace: 'team/search-result',
  web_url: 'https://gitlab.example.com/team/search-result',
}

const mockSyncResponse = {
  synced_commits: 3,
  synced_merge_requests: 2,
  work_items_created: 5,
}

describe('gitlab service', () => {
  beforeEach(() => {
    resetTauriMock()
    localStorage.setItem('recap_auth_token', 'test-token')
  })

  describe('getStatus', () => {
    it('should return GitLab configuration status', async () => {
      mockCommandValue('get_gitlab_status', mockGitLabStatus)

      const result = await gitlab.getStatus()

      expect(result.configured).toBe(true)
      expect(result.gitlab_url).toBe('https://gitlab.example.com')
      expect(mockInvoke).toHaveBeenCalledWith('get_gitlab_status', { token: 'test-token' })
    })

    it('should return unconfigured status', async () => {
      mockCommandValue('get_gitlab_status', { configured: false })

      const result = await gitlab.getStatus()

      expect(result.configured).toBe(false)
    })
  })

  describe('configure', () => {
    it('should configure GitLab', async () => {
      mockCommandValue('configure_gitlab', { message: 'GitLab configured successfully' })

      const request = {
        gitlab_url: 'https://gitlab.example.com',
        gitlab_pat: 'glpat-xxx',
      }
      const result = await gitlab.configure(request)

      expect(result.message).toBe('GitLab configured successfully')
      expect(mockInvoke).toHaveBeenCalledWith('configure_gitlab', {
        token: 'test-token',
        request,
      })
    })

    it('should throw on invalid token', async () => {
      mockCommandError('configure_gitlab', 'Invalid GitLab token')

      const request = { gitlab_url: 'https://gitlab.example.com', gitlab_pat: 'invalid' }

      await expect(gitlab.configure(request)).rejects.toThrow('Invalid GitLab token')
    })
  })

  describe('removeConfig', () => {
    it('should remove GitLab configuration', async () => {
      mockCommandValue('remove_gitlab_config', { message: 'GitLab configuration removed' })

      const result = await gitlab.removeConfig()

      expect(result.message).toBe('GitLab configuration removed')
      expect(mockInvoke).toHaveBeenCalledWith('remove_gitlab_config', { token: 'test-token' })
    })
  })

  describe('listProjects', () => {
    it('should list tracked GitLab projects', async () => {
      mockCommandValue('list_gitlab_projects', mockGitLabProjects)

      const result = await gitlab.listProjects()

      expect(result).toHaveLength(2)
      expect(result[0].name).toBe('Test Project')
      expect(result[1].name).toBe('Another Project')
    })

    it('should return empty list when no projects tracked', async () => {
      mockCommandValue('list_gitlab_projects', [])

      const result = await gitlab.listProjects()

      expect(result).toHaveLength(0)
    })
  })

  describe('addProject', () => {
    it('should add a GitLab project to track', async () => {
      mockCommandValue('add_gitlab_project', mockGitLabProject)

      const request = { gitlab_project_id: 123 }
      const result = await gitlab.addProject(request)

      expect(result.gitlab_project_id).toBe(123)
      expect(result.name).toBe('Test Project')
      expect(mockInvoke).toHaveBeenCalledWith('add_gitlab_project', {
        token: 'test-token',
        request,
      })
    })

    it('should throw on project not found', async () => {
      mockCommandError('add_gitlab_project', 'Project not found')

      const request = { gitlab_project_id: 999 }

      await expect(gitlab.addProject(request)).rejects.toThrow('Project not found')
    })

    it('should throw on duplicate project', async () => {
      mockCommandError('add_gitlab_project', 'Project already tracked')

      const request = { gitlab_project_id: 123 }

      await expect(gitlab.addProject(request)).rejects.toThrow('Project already tracked')
    })
  })

  describe('removeProject', () => {
    it('should remove a GitLab project from tracking', async () => {
      mockCommandValue('remove_gitlab_project', { message: 'Project removed' })

      const result = await gitlab.removeProject('project-1')

      expect(result.message).toBe('Project removed')
      expect(mockInvoke).toHaveBeenCalledWith('remove_gitlab_project', {
        token: 'test-token',
        id: 'project-1',
      })
    })

    it('should throw on project not found', async () => {
      mockCommandError('remove_gitlab_project', 'Project not found')

      await expect(gitlab.removeProject('non-existent')).rejects.toThrow('Project not found')
    })
  })

  describe('sync', () => {
    it('should sync GitLab data to work items', async () => {
      mockCommandValue('sync_gitlab', mockSyncResponse)

      const result = await gitlab.sync()

      expect(result.work_items_created).toBe(5)
      expect(result.synced_commits).toBe(3)
      expect(mockInvoke).toHaveBeenCalledWith('sync_gitlab', {
        token: 'test-token',
        request: {},
      })
    })

    it('should sync with date range', async () => {
      mockCommandValue('sync_gitlab', mockSyncResponse)

      const request = { start_date: '2024-01-01', end_date: '2024-01-31' }
      const result = await gitlab.sync(request)

      expect(result.work_items_created).toBe(5)
      expect(mockInvoke).toHaveBeenCalledWith('sync_gitlab', {
        token: 'test-token',
        request,
      })
    })
  })

  describe('searchProjects', () => {
    it('should search GitLab projects', async () => {
      mockCommandValue('search_gitlab_projects', [mockGitLabProjectInfo])

      const result = await gitlab.searchProjects({ search: 'test' })

      expect(result).toHaveLength(1)
      expect(result[0].name).toBe('Search Result')
      expect(mockInvoke).toHaveBeenCalledWith('search_gitlab_projects', {
        token: 'test-token',
        request: { search: 'test' },
      })
    })

    it('should return empty list when no matches', async () => {
      mockCommandValue('search_gitlab_projects', [])

      const result = await gitlab.searchProjects({ search: 'nonexistent' })

      expect(result).toHaveLength(0)
    })

    it('should search without query to get all projects', async () => {
      mockCommandValue('search_gitlab_projects', [mockGitLabProjectInfo])

      const result = await gitlab.searchProjects()

      expect(result).toHaveLength(1)
      expect(mockInvoke).toHaveBeenCalledWith('search_gitlab_projects', {
        token: 'test-token',
        request: {},
      })
    })
  })
})
