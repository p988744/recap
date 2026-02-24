import { vi, describe, it, expect, beforeEach } from 'vitest'
import {
  mockInvoke,
  mockCommandValue,
  mockCommandError,
  resetTauriMock,
} from '@/test/mocks/tauri'
import type {
  BackgroundSyncConfig,
  BackgroundSyncStatus,
  TriggerSyncResponse,
  SyncProgress,
} from './background-sync'
import * as backgroundSync from './background-sync'

// Mock @tauri-apps/api/event
const mockUnlisten = vi.fn()
const mockListen = vi.fn()

vi.mock('@tauri-apps/api/event', () => ({
  listen: (...args: unknown[]) => mockListen(...args),
}))

// Fixtures
const mockConfig: BackgroundSyncConfig = {
  enabled: true,
  interval_minutes: 15,
  compaction_interval_minutes: 60,
  sync_git: true,
  sync_claude: true,
  sync_antigravity: false,
  sync_gitlab: false,
  sync_jira: false,
  auto_generate_summaries: true,
  summary_max_chars: 500,
  summary_reasoning_effort: 'medium',
  summary_prompt: null,
}

const mockStatus: BackgroundSyncStatus = {
  is_running: true,
  is_syncing: false,
  is_compacting: false,
  syncing_started_at: null,
  last_sync_at: '2024-01-15T10:00:00Z',
  last_compaction_at: '2024-01-15T09:00:00Z',
  next_sync_at: '2024-01-15T10:15:00Z',
  next_compaction_at: '2024-01-15T11:00:00Z',
  last_result: 'Success',
  last_error: null,
}

const mockTriggerResponse: TriggerSyncResponse = {
  results: [
    {
      source: 'git',
      success: true,
      items_synced: 5,
      projects_scanned: 2,
      items_created: 3,
      error: null,
    },
  ],
  total_items: 5,
}

describe('background-sync service', () => {
  beforeEach(() => {
    resetTauriMock()
    mockListen.mockReset()
    mockUnlisten.mockReset()
    mockListen.mockResolvedValue(mockUnlisten)
    localStorage.setItem('recap_auth_token', 'test-token')
  })

  describe('getConfig', () => {
    it('should return background sync configuration', async () => {
      mockCommandValue('get_background_sync_config', mockConfig)

      const result = await backgroundSync.getConfig()

      expect(result).toEqual(mockConfig)
      expect(mockInvoke).toHaveBeenCalledWith('get_background_sync_config', {
        token: 'test-token',
      })
    })

    it('should throw on error', async () => {
      mockCommandError('get_background_sync_config', 'Failed to load config')

      await expect(backgroundSync.getConfig()).rejects.toThrow('Failed to load config')
    })
  })

  describe('updateConfig', () => {
    it('should update configuration and return updated config', async () => {
      const updatedConfig = { ...mockConfig, interval_minutes: 30 }
      mockCommandValue('update_background_sync_config', updatedConfig)

      const result = await backgroundSync.updateConfig({ interval_minutes: 30 })

      expect(result).toEqual(updatedConfig)
      expect(mockInvoke).toHaveBeenCalledWith('update_background_sync_config', {
        token: 'test-token',
        config: { interval_minutes: 30 },
      })
    })

    it('should throw on invalid config', async () => {
      mockCommandError('update_background_sync_config', 'Invalid interval')

      await expect(backgroundSync.updateConfig({ interval_minutes: -1 })).rejects.toThrow(
        'Invalid interval'
      )
    })
  })

  describe('getStatus', () => {
    it('should return background sync status', async () => {
      mockCommandValue('get_background_sync_status', mockStatus)

      const result = await backgroundSync.getStatus()

      expect(result).toEqual(mockStatus)
      expect(result.is_running).toBe(true)
      expect(mockInvoke).toHaveBeenCalledWith('get_background_sync_status', {
        token: 'test-token',
      })
    })

    it('should throw on error', async () => {
      mockCommandError('get_background_sync_status', 'Service unavailable')

      await expect(backgroundSync.getStatus()).rejects.toThrow('Service unavailable')
    })
  })

  describe('start', () => {
    it('should start the background sync service', async () => {
      mockCommandValue('start_background_sync', undefined)

      await backgroundSync.start()

      expect(mockInvoke).toHaveBeenCalledWith('start_background_sync', {
        token: 'test-token',
      })
    })

    it('should throw on error', async () => {
      mockCommandError('start_background_sync', 'Already running')

      await expect(backgroundSync.start()).rejects.toThrow('Already running')
    })
  })

  describe('stop', () => {
    it('should stop the background sync service', async () => {
      mockCommandValue('stop_background_sync', undefined)

      await backgroundSync.stop()

      expect(mockInvoke).toHaveBeenCalledWith('stop_background_sync', {
        token: 'test-token',
      })
    })

    it('should throw on error', async () => {
      mockCommandError('stop_background_sync', 'Not running')

      await expect(backgroundSync.stop()).rejects.toThrow('Not running')
    })
  })

  describe('cancelSync', () => {
    it('should cancel a stuck sync and return true', async () => {
      mockCommandValue('cancel_background_sync', true)

      const result = await backgroundSync.cancelSync()

      expect(result).toBe(true)
      expect(mockInvoke).toHaveBeenCalledWith('cancel_background_sync', {
        token: 'test-token',
      })
    })

    it('should return false when no sync was running', async () => {
      mockCommandValue('cancel_background_sync', false)

      const result = await backgroundSync.cancelSync()

      expect(result).toBe(false)
    })

    it('should throw on error', async () => {
      mockCommandError('cancel_background_sync', 'Cancel failed')

      await expect(backgroundSync.cancelSync()).rejects.toThrow('Cancel failed')
    })
  })

  describe('triggerSync', () => {
    it('should trigger an immediate sync', async () => {
      mockCommandValue('trigger_background_sync', mockTriggerResponse)

      const result = await backgroundSync.triggerSync()

      expect(result).toEqual(mockTriggerResponse)
      expect(result.total_items).toBe(5)
      expect(mockInvoke).toHaveBeenCalledWith('trigger_background_sync', {
        token: 'test-token',
      })
    })

    it('should throw on error', async () => {
      mockCommandError('trigger_background_sync', 'Sync already in progress')

      await expect(backgroundSync.triggerSync()).rejects.toThrow('Sync already in progress')
    })
  })

  describe('triggerSyncWithProgress', () => {
    it('should trigger sync and set up progress listener', async () => {
      mockCommandValue('trigger_sync_with_progress', mockTriggerResponse)

      const onProgress = vi.fn()
      const result = await backgroundSync.triggerSyncWithProgress(onProgress)

      expect(result).toEqual(mockTriggerResponse)
      expect(mockListen).toHaveBeenCalledWith('sync-progress', expect.any(Function))
      expect(mockUnlisten).toHaveBeenCalled()
    })

    it('should forward progress events to callback', async () => {
      mockCommandValue('trigger_sync_with_progress', mockTriggerResponse)

      let capturedCallback: ((event: { payload: SyncProgress }) => void) | undefined
      mockListen.mockImplementation(
        (_event: string, callback: (event: { payload: SyncProgress }) => void) => {
          capturedCallback = callback
          return Promise.resolve(mockUnlisten)
        }
      )

      const onProgress = vi.fn()
      const resultPromise = backgroundSync.triggerSyncWithProgress(onProgress)

      // Simulate progress event
      const progressEvent: SyncProgress = {
        phase: 'sources',
        current_source: 'git',
        current: 1,
        total: 3,
        message: 'Syncing git commits',
      }

      if (capturedCallback) {
        capturedCallback({ payload: progressEvent })
      }

      await resultPromise

      expect(onProgress).toHaveBeenCalledWith(progressEvent)
    })

    it('should work without onProgress callback', async () => {
      mockCommandValue('trigger_sync_with_progress', mockTriggerResponse)

      const result = await backgroundSync.triggerSyncWithProgress()

      expect(result).toEqual(mockTriggerResponse)
      expect(mockListen).not.toHaveBeenCalled()
      expect(mockUnlisten).not.toHaveBeenCalled()
    })

    it('should clean up listener even on error', async () => {
      mockCommandError('trigger_sync_with_progress', 'Sync failed')

      const onProgress = vi.fn()

      await expect(backgroundSync.triggerSyncWithProgress(onProgress)).rejects.toThrow(
        'Sync failed'
      )

      expect(mockUnlisten).toHaveBeenCalled()
    })

    it('should not call unlisten if listener was not set up', async () => {
      mockCommandError('trigger_sync_with_progress', 'Sync failed')

      await expect(backgroundSync.triggerSyncWithProgress()).rejects.toThrow('Sync failed')

      expect(mockUnlisten).not.toHaveBeenCalled()
    })
  })
})
