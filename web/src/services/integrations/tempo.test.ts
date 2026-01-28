import { describe, it, expect, beforeEach } from 'vitest'
import {
  mockInvoke,
  mockCommandValue,
  mockCommandError,
  resetTauriMock,
} from '@/test/mocks/tauri'
import * as tempo from './tempo'

// Mock fixtures
const mockTempoSuccess = {
  success: true,
  message: 'Connection successful',
}

const mockValidateIssueResponse = {
  valid: true,
  issue_key: 'PROJ-123',
  summary: 'Implement feature X',
  message: 'Issue found',
}

const mockSyncWorklogsResponse = {
  success: true,
  total_entries: 5,
  successful: 5,
  failed: 0,
  results: [],
  dry_run: false,
}

const mockWorklogEntryResponse = {
  id: 'worklog-1',
  issue_key: 'PROJ-123',
  date: '2024-01-15',
  minutes: 240,
  hours: 4.0,
  description: 'Feature development',
  status: 'success',
}

const mockWorklogs = [
  {
    id: 'worklog-1',
    issue_key: 'PROJ-123',
    hours: 4.0,
    date: '2024-01-15',
    description: 'Feature development',
  },
  {
    id: 'worklog-2',
    issue_key: 'PROJ-124',
    hours: 3.5,
    date: '2024-01-15',
    description: 'Code review',
  },
]

describe('tempo service', () => {
  beforeEach(() => {
    resetTauriMock()
    localStorage.setItem('recap_auth_token', 'test-token')
  })

  describe('testConnection', () => {
    it('should test Jira/Tempo connection successfully', async () => {
      mockCommandValue('test_tempo_connection', mockTempoSuccess)

      const result = await tempo.testConnection()

      expect(result.success).toBe(true)
      expect(result.message).toBe('Connection successful')
      expect(mockInvoke).toHaveBeenCalledWith('test_tempo_connection', { token: 'test-token' })
    })

    it('should return failure on invalid credentials', async () => {
      mockCommandValue('test_tempo_connection', {
        success: false,
        message: 'Invalid credentials',
      })

      const result = await tempo.testConnection()

      expect(result.success).toBe(false)
      expect(result.message).toBe('Invalid credentials')
    })

    it('should throw on connection error', async () => {
      mockCommandError('test_tempo_connection', 'Network error')

      await expect(tempo.testConnection()).rejects.toThrow('Network error')
    })
  })

  describe('validateIssue', () => {
    it('should validate a valid Jira issue key', async () => {
      mockCommandValue('validate_jira_issue', mockValidateIssueResponse)

      const result = await tempo.validateIssue('PROJ-123')

      expect(result.valid).toBe(true)
      expect(result.issue_key).toBe('PROJ-123')
      expect(result.summary).toBe('Implement feature X')
      expect(mockInvoke).toHaveBeenCalledWith('validate_jira_issue', {
        token: 'test-token',
        issueKey: 'PROJ-123',
      })
    })

    it('should return invalid for non-existent issue', async () => {
      mockCommandValue('validate_jira_issue', {
        valid: false,
        issue_key: 'PROJ-999',
        error: 'Issue not found',
      })

      const result = await tempo.validateIssue('PROJ-999')

      expect(result.valid).toBe(false)
    })

    it('should throw on invalid issue key format', async () => {
      mockCommandError('validate_jira_issue', 'Invalid issue key format')

      await expect(tempo.validateIssue('invalid')).rejects.toThrow('Invalid issue key format')
    })
  })

  describe('syncWorklogs', () => {
    it('should sync multiple worklogs to Tempo', async () => {
      mockCommandValue('sync_worklogs_to_tempo', mockSyncWorklogsResponse)

      const request = {
        entries: [
          { issue_key: 'PROJ-1', date: '2024-01-15', minutes: 240, description: 'Work 1' },
          { issue_key: 'PROJ-2', date: '2024-01-15', minutes: 180, description: 'Work 2' },
        ],
      }
      const result = await tempo.syncWorklogs(request)

      expect(result.successful).toBe(5)
      expect(result.failed).toBe(0)
      expect(mockInvoke).toHaveBeenCalledWith('sync_worklogs_to_tempo', {
        token: 'test-token',
        request,
      })
    })

    it('should handle partial sync with failures', async () => {
      mockCommandValue('sync_worklogs_to_tempo', {
        success: true,
        total_entries: 5,
        successful: 3,
        failed: 2,
        results: [],
        dry_run: false,
      })

      const request = {
        entries: [
          { issue_key: 'PROJ-1', date: '2024-01-15', minutes: 240, description: 'Work 1' },
        ],
      }
      const result = await tempo.syncWorklogs(request)

      expect(result.successful).toBe(3)
      expect(result.failed).toBe(2)
    })

    it('should throw when Jira is not configured', async () => {
      mockCommandError('sync_worklogs_to_tempo', 'Jira not configured')

      const request = {
        entries: [{ issue_key: 'PROJ-1', date: '2024-01-15', minutes: 240, description: 'Work' }],
      }

      await expect(tempo.syncWorklogs(request)).rejects.toThrow('Jira not configured')
    })
  })

  describe('uploadWorklog', () => {
    it('should upload a single worklog entry', async () => {
      mockCommandValue('upload_single_worklog', mockWorklogEntryResponse)

      const request = {
        issue_key: 'PROJ-123',
        date: '2024-01-15',
        minutes: 240,
        description: 'Feature development',
      }
      const result = await tempo.uploadWorklog(request)

      expect(result.issue_key).toBe('PROJ-123')
      expect(result.hours).toBe(4.0)
      expect(result.status).toBe('success')
      expect(mockInvoke).toHaveBeenCalledWith('upload_single_worklog', {
        token: 'test-token',
        request,
      })
    })

    it('should throw on invalid issue', async () => {
      mockCommandError('upload_single_worklog', 'Issue not found')

      const request = {
        issue_key: 'INVALID-999',
        date: '2024-01-15',
        minutes: 240,
        description: 'Work',
      }

      await expect(tempo.uploadWorklog(request)).rejects.toThrow('Issue not found')
    })

    it('should throw on invalid hours', async () => {
      mockCommandError('upload_single_worklog', 'Hours must be positive')

      const request = {
        issue_key: 'PROJ-123',
        date: '2024-01-15',
        minutes: -60,
        description: 'Work',
      }

      await expect(tempo.uploadWorklog(request)).rejects.toThrow('Hours must be positive')
    })
  })

  describe('getWorklogs', () => {
    it('should get worklogs from Tempo for a date range', async () => {
      mockCommandValue('get_tempo_worklogs', mockWorklogs)

      const request = {
        date_from: '2024-01-01',
        date_to: '2024-01-31',
      }
      const result = await tempo.getWorklogs(request)

      expect(result).toHaveLength(2)
      expect(mockInvoke).toHaveBeenCalledWith('get_tempo_worklogs', {
        token: 'test-token',
        request,
      })
    })

    it('should return empty array when no worklogs found', async () => {
      mockCommandValue('get_tempo_worklogs', [])

      const request = {
        date_from: '2024-01-01',
        date_to: '2024-01-01',
      }
      const result = await tempo.getWorklogs(request)

      expect(result).toHaveLength(0)
    })
  })
})
