/**
 * App-level sync hook.
 *
 * Manages the background sync service lifecycle and initial sync.
 * Lives at the Layout level so it works regardless of which page is active.
 */
import { useEffect, useCallback, useState } from 'react'
import { listen } from '@tauri-apps/api/event'
import { backgroundSync, sync, tray, notification } from '@/services'

export function useAppSync(isAuthenticated: boolean, token: string | null) {
  const [syncState, setSyncState] = useState<'idle' | 'syncing' | 'done'>('idle')

  // Full sync: auto_sync (work items) + triggerSync (snapshots + compaction)
  const performFullSync = useCallback(async () => {
    if (!isAuthenticated || !token) return
    setSyncState('syncing')

    await tray.setSyncing(true).catch(() => {})

    try {
      // Phase 1: Sync Claude sessions → work items
      const result = await sync.autoSync()

      const failedResults = result.results.filter((r) => !r.success)
      if (failedResults.length > 0) {
        const errorSources = failedResults.map((r) => r.source).join(', ')
        await notification.sendSyncNotification(false, `${errorSources} 同步失敗`).catch(() => {})
      }

      // Phase 2: Capture snapshots + run compaction (for worklog data)
      await backgroundSync.triggerSync().catch((err) => {
        console.warn('Background sync trigger failed:', err)
      })

      const now = new Date()
      await tray.updateSyncStatus(now.toISOString(), false).catch(() => {})
    } catch {
      await tray.setSyncing(false).catch(() => {})
    } finally {
      setSyncState('done')
    }
  }, [isAuthenticated, token])

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
    if (syncState === 'idle') {
      performFullSync()
    }
  }, []) // eslint-disable-line react-hooks/exhaustive-deps

  // Listen for tray "Sync Now" event
  useEffect(() => {
    const unlisten = listen('tray-sync-now', () => {
      if (syncState !== 'syncing') {
        setSyncState('idle')
        performFullSync()
      }
    })

    return () => {
      unlisten.then((fn) => fn())
    }
  }, [syncState, performFullSync])

  return { syncState, performFullSync }
}
