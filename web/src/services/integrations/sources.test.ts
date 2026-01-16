import { describe, it, expect, beforeEach } from 'vitest'
import {
  mockInvoke,
  mockCommandValue,
  mockCommandError,
  resetTauriMock,
} from '@/test/mocks/tauri'
import * as sources from './sources'

// Mock fixtures
const mockSourcesResponse = {
  git_repos: [
    { id: 'repo-1', path: '/home/user/project-a', name: 'project-a' },
    { id: 'repo-2', path: '/home/user/project-b', name: 'project-b' },
  ],
  source_mode: 'git',
  claude_projects_dir: '/home/user/.claude/projects',
}

const mockAddGitRepoResponse = {
  id: 'repo-3',
  path: '/home/user/new-project',
  name: 'new-project',
}

const mockSourceModeResponse = {
  mode: 'claude',
  message: 'Source mode updated',
}

describe('sources service', () => {
  beforeEach(() => {
    resetTauriMock()
    localStorage.setItem('recap_auth_token', 'test-token')
  })

  describe('getSources', () => {
    it('should return data sources configuration', async () => {
      mockCommandValue('get_sources', mockSourcesResponse)

      const result = await sources.getSources()

      expect(result).toEqual(mockSourcesResponse)
      expect(result.git_repos).toHaveLength(2)
      expect(result.source_mode).toBe('git')
      expect(mockInvoke).toHaveBeenCalledWith('get_sources', { token: 'test-token' })
    })

    it('should throw on error', async () => {
      mockCommandError('get_sources', 'Failed to load sources')

      await expect(sources.getSources()).rejects.toThrow('Failed to load sources')
    })
  })

  describe('addGitRepo', () => {
    it('should add a local Git repository', async () => {
      mockCommandValue('add_git_repo', mockAddGitRepoResponse)

      const result = await sources.addGitRepo('/home/user/new-project')

      expect(result.id).toBe('repo-3')
      expect(result.name).toBe('new-project')
      expect(mockInvoke).toHaveBeenCalledWith('add_git_repo', {
        token: 'test-token',
        path: '/home/user/new-project',
      })
    })

    it('should throw on invalid path', async () => {
      mockCommandError('add_git_repo', 'Path is not a valid Git repository')

      await expect(sources.addGitRepo('/invalid/path')).rejects.toThrow(
        'Path is not a valid Git repository'
      )
    })

    it('should throw on duplicate repository', async () => {
      mockCommandError('add_git_repo', 'Repository already exists')

      await expect(sources.addGitRepo('/home/user/project-a')).rejects.toThrow(
        'Repository already exists'
      )
    })
  })

  describe('removeGitRepo', () => {
    it('should remove a local Git repository', async () => {
      mockCommandValue('remove_git_repo', { message: 'Repository removed' })

      const result = await sources.removeGitRepo('repo-1')

      expect(result.message).toBe('Repository removed')
      expect(mockInvoke).toHaveBeenCalledWith('remove_git_repo', {
        token: 'test-token',
        repo_id: 'repo-1',
      })
    })

    it('should throw on non-existent repository', async () => {
      mockCommandError('remove_git_repo', 'Repository not found')

      await expect(sources.removeGitRepo('non-existent')).rejects.toThrow('Repository not found')
    })
  })

  describe('setSourceMode', () => {
    it('should set source mode to claude', async () => {
      mockCommandValue('set_source_mode', mockSourceModeResponse)

      const result = await sources.setSourceMode('claude')

      expect(result.mode).toBe('claude')
      expect(mockInvoke).toHaveBeenCalledWith('set_source_mode', {
        token: 'test-token',
        mode: 'claude',
      })
    })

    it('should set source mode to git', async () => {
      mockCommandValue('set_source_mode', { mode: 'git', message: 'Source mode updated' })

      const result = await sources.setSourceMode('git')

      expect(result.mode).toBe('git')
    })

    it('should throw on invalid mode', async () => {
      mockCommandError('set_source_mode', 'Invalid source mode')

      await expect(sources.setSourceMode('invalid')).rejects.toThrow('Invalid source mode')
    })
  })
})
