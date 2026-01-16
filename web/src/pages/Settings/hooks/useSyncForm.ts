import { useCallback, useEffect, useState } from 'react'
import { backgroundSync, tray } from '@/services'
import type { BackgroundSyncConfig, BackgroundSyncStatus } from '@/services/background-sync'
import type { SettingsMessage } from './types'

// =============================================================================
// Types
// =============================================================================

export interface SyncFormState {
  // Config
  enabled: boolean
  intervalMinutes: number
  syncGit: boolean
  syncClaude: boolean
  syncGitlab: boolean
  syncJira: boolean
  // Status
  status: BackgroundSyncStatus | null
  // UI State
  loading: boolean
  saving: boolean
}

// =============================================================================
// Hook
// =============================================================================

export function useSyncForm() {
  const [state, setState] = useState<SyncFormState>({
    enabled: true,
    intervalMinutes: 15,
    syncGit: true,
    syncClaude: true,
    syncGitlab: false,
    syncJira: false,
    status: null,
    loading: true,
    saving: false,
  })

  // Fetch initial config and status
  useEffect(() => {
    async function fetchData() {
      try {
        const [config, status] = await Promise.all([
          backgroundSync.getConfig(),
          backgroundSync.getStatus(),
        ])
        setState((prev) => ({
          ...prev,
          enabled: config.enabled,
          intervalMinutes: config.interval_minutes,
          syncGit: config.sync_git,
          syncClaude: config.sync_claude,
          syncGitlab: config.sync_gitlab,
          syncJira: config.sync_jira,
          status,
          loading: false,
        }))

        // Update tray with current status
        if (status.is_syncing) {
          await tray.setSyncing(true).catch(() => {})
        } else if (status.last_sync_at) {
          await tray.updateSyncStatus(status.last_sync_at, false).catch(() => {})
        }
      } catch (err) {
        console.error('Failed to fetch sync config:', err)
        setState((prev) => ({ ...prev, loading: false }))
      }
    }
    fetchData()
  }, [])

  // Refresh status periodically when syncing
  useEffect(() => {
    if (!state.status?.is_syncing) return

    const interval = setInterval(async () => {
      try {
        const status = await backgroundSync.getStatus()
        setState((prev) => ({ ...prev, status }))

        // Update tray when sync completes
        if (!status.is_syncing) {
          if (status.last_sync_at) {
            await tray.updateSyncStatus(status.last_sync_at, false).catch(() => {})
          } else {
            await tray.setSyncing(false).catch(() => {})
          }
        }
      } catch {
        // Ignore errors
      }
    }, 2000)

    return () => clearInterval(interval)
  }, [state.status?.is_syncing])

  // Setters
  const setEnabled = useCallback((enabled: boolean) => {
    setState((prev) => ({ ...prev, enabled }))
  }, [])

  const setIntervalMinutes = useCallback((intervalMinutes: number) => {
    setState((prev) => ({ ...prev, intervalMinutes }))
  }, [])

  const setSyncGit = useCallback((syncGit: boolean) => {
    setState((prev) => ({ ...prev, syncGit }))
  }, [])

  const setSyncClaude = useCallback((syncClaude: boolean) => {
    setState((prev) => ({ ...prev, syncClaude }))
  }, [])

  const setSyncGitlab = useCallback((syncGitlab: boolean) => {
    setState((prev) => ({ ...prev, syncGitlab }))
  }, [])

  const setSyncJira = useCallback((syncJira: boolean) => {
    setState((prev) => ({ ...prev, syncJira }))
  }, [])

  // Save config
  const handleSave = useCallback(
    async (setMessage: (msg: SettingsMessage | null) => void) => {
      setState((prev) => ({ ...prev, saving: true }))
      try {
        await backgroundSync.updateConfig({
          enabled: state.enabled,
          interval_minutes: state.intervalMinutes,
          sync_git: state.syncGit,
          sync_claude: state.syncClaude,
          sync_gitlab: state.syncGitlab,
          sync_jira: state.syncJira,
        })

        // If enabled, restart the service to apply new settings
        if (state.enabled) {
          await backgroundSync.start()
        } else {
          await backgroundSync.stop()
        }

        // Refresh status
        const status = await backgroundSync.getStatus()
        setState((prev) => ({ ...prev, status }))

        setMessage({ type: 'success', text: '同步設定已儲存' })
      } catch (err) {
        setMessage({
          type: 'error',
          text: err instanceof Error ? err.message : '儲存失敗',
        })
      } finally {
        setState((prev) => ({ ...prev, saving: false }))
      }
    },
    [state.enabled, state.intervalMinutes, state.syncGit, state.syncClaude, state.syncGitlab, state.syncJira]
  )

  // Trigger immediate sync
  const handleTriggerSync = useCallback(
    async (setMessage: (msg: SettingsMessage | null) => void) => {
      try {
        // Update status to show syncing
        setState((prev) => ({
          ...prev,
          status: prev.status ? { ...prev.status, is_syncing: true } : null,
        }))

        // Update tray to show syncing state
        await tray.setSyncing(true).catch(() => {})

        const result = await backgroundSync.triggerSync()

        // Refresh status
        const status = await backgroundSync.getStatus()
        setState((prev) => ({ ...prev, status }))

        // Update tray with last sync time
        if (status.last_sync_at) {
          await tray.updateSyncStatus(status.last_sync_at, false).catch(() => {})
        } else {
          await tray.setSyncing(false).catch(() => {})
        }

        if (result.total_items > 0) {
          setMessage({
            type: 'success',
            text: `已同步 ${result.total_items} 筆工作項目`,
          })
        } else {
          setMessage({ type: 'success', text: '同步完成，無新項目' })
        }
      } catch (err) {
        // Refresh status anyway
        const status = await backgroundSync.getStatus().catch(() => null)
        if (status) {
          setState((prev) => ({ ...prev, status }))
          // Update tray
          if (status.last_sync_at) {
            await tray.updateSyncStatus(status.last_sync_at, false).catch(() => {})
          } else {
            await tray.setSyncing(false).catch(() => {})
          }
        } else {
          await tray.setSyncing(false).catch(() => {})
        }

        setMessage({
          type: 'error',
          text: err instanceof Error ? err.message : '同步失敗',
        })
      }
    },
    []
  )

  // Refresh status
  const refreshStatus = useCallback(async () => {
    try {
      const status = await backgroundSync.getStatus()
      setState((prev) => ({ ...prev, status }))
    } catch {
      // Ignore errors
    }
  }, [])

  return {
    // Config
    enabled: state.enabled,
    setEnabled,
    intervalMinutes: state.intervalMinutes,
    setIntervalMinutes,
    syncGit: state.syncGit,
    setSyncGit,
    syncClaude: state.syncClaude,
    setSyncClaude,
    syncGitlab: state.syncGitlab,
    setSyncGitlab,
    syncJira: state.syncJira,
    setSyncJira,
    // Status
    status: state.status,
    refreshStatus,
    // UI State
    loading: state.loading,
    saving: state.saving,
    // Actions
    handleSave,
    handleTriggerSync,
  }
}
