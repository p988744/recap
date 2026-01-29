/**
 * App-level sync hook & context.
 *
 * Manages the background sync service lifecycle and initial sync.
 * Lives at the Layout level so it works regardless of which page is active.
 * Pages consume sync state via useSyncContext() — they never trigger sync directly.
 *
 * Two-phase sync:
 *   Phase 1 — 資料同步: discover Claude sessions → import work items (autoSync)
 *   Phase 2 — 摘要處理: capture snapshots + run LLM compaction (triggerSync)
 *
 * Single source of truth: backend BackgroundSyncStatus via getStatus().
 */
import { createContext, useContext, useEffect, useCallback, useState, useRef } from 'react'
import { listen } from '@tauri-apps/api/event'
import { backgroundSync, tray, notification } from '@/services'
import type { BackgroundSyncStatus, SyncProgress } from '@/services/background-sync'

// =============================================================================
// Context
// =============================================================================

export interface SyncContextValue {
  /** Phase 1: data sync state */
  dataSyncState: 'idle' | 'syncing' | 'done'
  /** Phase 2: summary/compaction state */
  summaryState: 'idle' | 'syncing' | 'done'
  /** Backend sync status (single source of truth for timing & config) */
  backendStatus: BackgroundSyncStatus | null
  /** Detailed sync progress */
  syncProgress: SyncProgress | null
  /** Transient info message after sync (auto-clears) */
  syncInfo: string
  /** Trigger a full sync (work items + snapshots + compaction) */
  performFullSync: () => Promise<void>
  /** Refresh backend status (call after config changes) */
  refreshStatus: () => Promise<void>
}

const SyncContext = createContext<SyncContextValue | null>(null)

export const SyncProvider = SyncContext.Provider

/**
 * Consume app-level sync state from any page.
 * Must be used inside a SyncProvider (Layout).
 */
export function useSyncContext(): SyncContextValue {
  const ctx = useContext(SyncContext)
  if (!ctx) {
    throw new Error('useSyncContext must be used within a SyncProvider')
  }
  return ctx
}

// =============================================================================
// Hook (used only by Layout)
// =============================================================================

/** Poll interval for backend status (30 seconds) */
const STATUS_POLL_INTERVAL = 30_000

export function useAppSync(isAuthenticated: boolean, token: string | null): SyncContextValue {
  const [dataSyncState, setDataSyncState] = useState<'idle' | 'syncing' | 'done'>('idle')
  const [summaryState, setSummaryState] = useState<'idle' | 'syncing' | 'done'>('idle')
  const [backendStatus, setBackendStatus] = useState<BackgroundSyncStatus | null>(null)
  const [syncProgress, setSyncProgress] = useState<SyncProgress | null>(null)
  const [syncInfo, setSyncInfo] = useState('')
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null)

  // Fetch backend status
  const refreshStatus = useCallback(async () => {
    try {
      const status = await backgroundSync.getStatus()
      setBackendStatus(status)
    } catch {
      // Ignore errors
    }
  }, [])

  // Full sync: Uses unified sync with progress
  const performFullSync = useCallback(async () => {
    if (!isAuthenticated || !token) return

    await tray.setSyncing(true).catch(() => {})
    setSyncProgress(null)

    try {
      // Use unified sync with progress
      setDataSyncState('syncing')
      setSummaryState('syncing')

      const result = await backgroundSync.triggerSyncWithProgress((progress) => {
        setSyncProgress(progress)

        // Update phase states based on progress
        if (progress.phase === 'sources') {
          setDataSyncState('syncing')
        } else if (progress.phase === 'snapshots' || progress.phase === 'compaction') {
          setDataSyncState('done')
          setSummaryState('syncing')
        } else if (progress.phase === 'complete') {
          setDataSyncState('done')
          setSummaryState('done')
        }
      })

      setDataSyncState('done')
      setSummaryState('done')

      const failedResults = result.results.filter((r) => !r.success)
      if (failedResults.length > 0) {
        const errorSources = failedResults.map((r) => r.source).join(', ')
        await notification.sendSyncNotification(false, `${errorSources} 同步失敗`).catch(() => {})
      }

      // Surface sync info for UI
      if (result.total_items > 0) {
        setSyncInfo(`同步完成，共處理 ${result.total_items} 筆資料`)
        setTimeout(() => setSyncInfo(''), 4000)
      }

      // Refresh from backend (single source of truth)
      await refreshStatus()
      const status = await backgroundSync.getStatus().catch(() => null)
      if (status?.last_sync_at) {
        await tray.updateSyncStatus(status.last_sync_at, false).catch(() => {})
      }

      // Clear progress after a short delay
      setTimeout(() => setSyncProgress(null), 1000)
    } catch {
      setDataSyncState('done')
      setSummaryState('done')
      setSyncProgress(null)
      await tray.setSyncing(false).catch(() => {})
    }
  }, [isAuthenticated, token, refreshStatus])

  // Start background sync service when authenticated
  useEffect(() => {
    if (!isAuthenticated || !token) return

    backgroundSync.start().catch((err) => {
      console.warn('Failed to start background sync:', err)
    })

    return () => {
      backgroundSync.stop().catch(() => {})
    }
  }, [isAuthenticated, token])

  // Initial sync on first mount
  useEffect(() => {
    if (dataSyncState === 'idle') {
      performFullSync()
    }
  }, []) // eslint-disable-line react-hooks/exhaustive-deps

  // Periodic status polling to stay in sync with backend
  useEffect(() => {
    if (!isAuthenticated) return

    // Fetch immediately
    refreshStatus()

    pollRef.current = setInterval(refreshStatus, STATUS_POLL_INTERVAL)

    return () => {
      if (pollRef.current) {
        clearInterval(pollRef.current)
        pollRef.current = null
      }
    }
  }, [isAuthenticated, refreshStatus])

  // Listen for tray "Sync Now" event
  useEffect(() => {
    const isSyncing = dataSyncState === 'syncing' || summaryState === 'syncing'
    const unlisten = listen('tray-sync-now', () => {
      if (!isSyncing) {
        performFullSync()
      }
    })

    return () => {
      unlisten.then((fn) => fn())
    }
  }, [dataSyncState, summaryState, performFullSync])

  return { dataSyncState, summaryState, backendStatus, syncProgress, syncInfo, performFullSync, refreshStatus }
}
