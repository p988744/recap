import { useCallback, useEffect, useMemo, useState } from 'react'
import { backgroundSync } from '@/services'
import type { BackgroundSyncStatus } from '@/services/background-sync'
import type { SettingsMessage } from './types'
import { useSyncContext } from '@/hooks/useAppSync'

// =============================================================================
// Types
// =============================================================================

export interface SyncFormState {
  // Config
  enabled: boolean
  intervalMinutes: number
  compactionIntervalHours: number
  autoGenerateSummaries: boolean
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
    compactionIntervalHours: 6,
    autoGenerateSummaries: true,
    loading: true,
    saving: false,
  })

  // App-level sync (lifecycle, tray state, triggering, shared status)
  const {
    performFullSync,
    backendStatus,
    refreshStatus,
    dataSyncState,
    summaryState,
    syncProgress,
  } = useSyncContext()

  // Merge frontend phase states into backend status for UI
  const isSyncing = dataSyncState === 'syncing' || summaryState === 'syncing'
  const status = useMemo<BackgroundSyncStatus | null>(() => {
    if (!backendStatus) {
      if (isSyncing) {
        return {
          is_running: false,
          is_syncing: true,
          is_compacting: false,
          last_sync_at: null,
          last_compaction_at: null,
          next_sync_at: null,
          next_compaction_at: null,
          last_result: null,
          last_error: null,
        }
      }
      return null
    }
    return {
      ...backendStatus,
      is_syncing: backendStatus.is_syncing || isSyncing,
    }
  }, [backendStatus, isSyncing])

  // Fetch initial config
  useEffect(() => {
    async function fetchData() {
      try {
        const config = await backgroundSync.getConfig()
        setState((prev) => ({
          ...prev,
          enabled: config.enabled,
          intervalMinutes: config.interval_minutes,
          compactionIntervalHours: config.compaction_interval_hours,
          autoGenerateSummaries: config.auto_generate_summaries,
          loading: false,
        }))
      } catch (err) {
        console.error('Failed to fetch sync config:', err)
        setState((prev) => ({ ...prev, loading: false }))
      }
    }
    fetchData()
  }, [])

  // Setters
  const setEnabled = useCallback((enabled: boolean) => {
    setState((prev) => ({ ...prev, enabled }))
  }, [])

  const setIntervalMinutes = useCallback((intervalMinutes: number) => {
    setState((prev) => ({ ...prev, intervalMinutes }))
  }, [])

  const setCompactionIntervalHours = useCallback((compactionIntervalHours: number) => {
    setState((prev) => ({ ...prev, compactionIntervalHours }))
  }, [])

  const setAutoGenerateSummaries = useCallback((autoGenerateSummaries: boolean) => {
    setState((prev) => ({ ...prev, autoGenerateSummaries }))
  }, [])

  // Save config (backend handles restart/stop internally via update_config)
  const handleSave = useCallback(
    async (setMessage: (msg: SettingsMessage | null) => void) => {
      setState((prev) => ({ ...prev, saving: true }))
      try {
        await backgroundSync.updateConfig({
          enabled: state.enabled,
          interval_minutes: state.intervalMinutes,
          compaction_interval_hours: state.compactionIntervalHours,
          sync_git: true,
          sync_claude: true,
          sync_gitlab: false,
          sync_jira: false,
          auto_generate_summaries: state.autoGenerateSummaries,
        })

        // Refresh shared status so sidebar updates too
        await refreshStatus()

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
    [state.enabled, state.intervalMinutes, state.compactionIntervalHours, state.autoGenerateSummaries, refreshStatus]
  )

  // Trigger immediate sync via app-level sync
  const handleTriggerSync = useCallback(
    async (setMessage: (msg: SettingsMessage | null) => void) => {
      try {
        await performFullSync()
        // performFullSync already calls refreshStatus internally
        setMessage({ type: 'success', text: '同步完成' })
      } catch (err) {
        setMessage({
          type: 'error',
          text: err instanceof Error ? err.message : '同步失敗',
        })
      }
    },
    [performFullSync]
  )

  return {
    // Config
    enabled: state.enabled,
    setEnabled,
    intervalMinutes: state.intervalMinutes,
    setIntervalMinutes,
    compactionIntervalHours: state.compactionIntervalHours,
    setCompactionIntervalHours,
    autoGenerateSummaries: state.autoGenerateSummaries,
    setAutoGenerateSummaries,
    // Status (merged: backend + frontend phase states)
    status,
    // Phase states (for split display)
    dataSyncState,
    summaryState,
    // Detailed progress
    syncProgress,
    // UI State
    loading: state.loading,
    saving: state.saving,
    // Actions
    handleSave,
    handleTriggerSync,
  }
}
