import { describe, it, expect, beforeEach } from 'vitest'
import {
  mockInvoke,
  mockCommandValue,
  mockCommandError,
  resetTauriMock,
} from '@/test/mocks/tauri'
import {
  mockWorkItem,
  mockPaginatedWorkItems,
  mockWorkItemStats,
} from '@/test/fixtures'
import * as workItems from './work-items'

describe('work-items service', () => {
  beforeEach(() => {
    resetTauriMock()
    localStorage.clear()
    localStorage.setItem('recap_auth_token', 'test-token')
  })

  describe('list', () => {
    it('should list work items with default filters', async () => {
      mockCommandValue('list_work_items', mockPaginatedWorkItems)

      const result = await workItems.list()

      expect(result).toEqual(mockPaginatedWorkItems)
      expect(mockInvoke).toHaveBeenCalledWith('list_work_items', {
        token: 'test-token',
        filters: {},
      })
    })

    it('should list work items with filters', async () => {
      mockCommandValue('list_work_items', mockPaginatedWorkItems)

      const filters = {
        source: 'git',
        start_date: '2024-01-01',
        end_date: '2024-01-31',
        page: 1,
        per_page: 20,
      }
      const result = await workItems.list(filters)

      expect(result).toEqual(mockPaginatedWorkItems)
      expect(mockInvoke).toHaveBeenCalledWith('list_work_items', {
        token: 'test-token',
        filters,
      })
    })
  })

  describe('get', () => {
    it('should get a work item by id', async () => {
      mockCommandValue('get_work_item', mockWorkItem)

      const result = await workItems.get('work-item-1')

      expect(result).toEqual(mockWorkItem)
      expect(mockInvoke).toHaveBeenCalledWith('get_work_item', {
        token: 'test-token',
        id: 'work-item-1',
      })
    })

    it('should throw on not found', async () => {
      mockCommandError('get_work_item', 'Work item not found')

      await expect(workItems.get('invalid-id')).rejects.toThrow('Work item not found')
    })
  })

  describe('create', () => {
    it('should create a new work item', async () => {
      mockCommandValue('create_work_item', mockWorkItem)

      const request = {
        title: 'New task',
        hours: 2.0,
        date: '2024-01-15',
        source: 'manual',
      }
      const result = await workItems.create(request)

      expect(result).toEqual(mockWorkItem)
      expect(mockInvoke).toHaveBeenCalledWith('create_work_item', {
        token: 'test-token',
        request,
      })
    })
  })

  describe('update', () => {
    it('should update a work item', async () => {
      const updatedItem = { ...mockWorkItem, title: 'Updated title' }
      mockCommandValue('update_work_item', updatedItem)

      const result = await workItems.update('work-item-1', { title: 'Updated title' })

      expect(result.title).toBe('Updated title')
      expect(mockInvoke).toHaveBeenCalledWith('update_work_item', {
        token: 'test-token',
        id: 'work-item-1',
        request: { title: 'Updated title' },
      })
    })
  })

  describe('remove', () => {
    it('should delete a work item', async () => {
      mockCommandValue('delete_work_item', { message: 'Deleted', count: 1 })

      await workItems.remove('work-item-1')

      expect(mockInvoke).toHaveBeenCalledWith('delete_work_item', {
        token: 'test-token',
        id: 'work-item-1',
      })
    })
  })

  describe('mapToJira', () => {
    it('should map work item to Jira issue', async () => {
      const mappedItem = { ...mockWorkItem, jira_issue_key: 'PROJ-456' }
      mockCommandValue('map_work_item_jira', mappedItem)

      const result = await workItems.mapToJira('work-item-1', 'PROJ-456', 'Issue Title')

      expect(result.jira_issue_key).toBe('PROJ-456')
      expect(mockInvoke).toHaveBeenCalledWith('map_work_item_jira', {
        token: 'test-token',
        work_item_id: 'work-item-1',
        jira_issue_key: 'PROJ-456',
        jira_issue_title: 'Issue Title',
      })
    })
  })

  describe('getStats', () => {
    it('should get work item statistics', async () => {
      mockCommandValue('get_stats_summary', mockWorkItemStats)

      const result = await workItems.getStats()

      expect(result).toEqual(mockWorkItemStats)
      expect(result.total_items).toBe(10)
      expect(result.total_hours).toBe(45.5)
    })

    it('should get stats with date range', async () => {
      mockCommandValue('get_stats_summary', mockWorkItemStats)

      await workItems.getStats({
        start_date: '2024-01-01',
        end_date: '2024-01-31',
      })

      expect(mockInvoke).toHaveBeenCalledWith('get_stats_summary', {
        token: 'test-token',
        query: {
          start_date: '2024-01-01',
          end_date: '2024-01-31',
        },
      })
    })
  })

  describe('getTimeline', () => {
    it('should get timeline data for a date', async () => {
      const mockTimeline = {
        date: '2024-01-15',
        sessions: [],
        total_hours: 8.0,
        total_commits: 5,
      }
      mockCommandValue('get_timeline_data', mockTimeline)

      const result = await workItems.getTimeline('2024-01-15')

      expect(result.date).toBe('2024-01-15')
      expect(mockInvoke).toHaveBeenCalledWith('get_timeline_data', {
        token: 'test-token',
        date: '2024-01-15',
      })
    })
  })
})
