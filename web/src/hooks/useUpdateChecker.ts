/**
 * App update checker hook & context.
 *
 * Periodically checks for app updates via the Tauri updater plugin.
 * Lives at the Layout level; pages consume state via useUpdateChecker().
 *
 * State machine:
 *   idle → checking → up-to-date / available / error
 *   available → downloading → ready
 */
import { createContext, useContext, useEffect, useCallback, useState, useRef } from 'react'
import { updater } from '@/services'
import type { Update, DownloadProgress } from '@/services/updater'

// =============================================================================
// Types
// =============================================================================

export type UpdateStatus =
  | 'idle'
  | 'checking'
  | 'up-to-date'
  | 'available'
  | 'downloading'
  | 'ready'
  | 'error'

export interface UpdateState {
  status: UpdateStatus
  version: string | null
  notes: string | null
  progress: DownloadProgress | null
  error: string | null
  lastCheckedAt: string | null
}

export interface UpdateContextValue extends UpdateState {
  checkForUpdate: () => Promise<void>
  downloadAndInstall: () => Promise<void>
  relaunchApp: () => Promise<void>
}

// =============================================================================
// Context
// =============================================================================

const UpdateContext = createContext<UpdateContextValue | null>(null)

export const UpdateProvider = UpdateContext.Provider

/**
 * Consume update checker state from any page.
 * Must be used inside an UpdateProvider (Layout).
 */
export function useUpdateChecker(): UpdateContextValue {
  const ctx = useContext(UpdateContext)
  if (!ctx) {
    throw new Error('useUpdateChecker must be used within an UpdateProvider')
  }
  return ctx
}

// =============================================================================
// Constants
// =============================================================================

const STORAGE_KEY = 'recap_update_last_checked_at'
const CHECK_INTERVAL = 24 * 60 * 60 * 1000 // 24 hours
const INITIAL_DELAY = 30_000 // 30 seconds after mount

// =============================================================================
// Hook (used only by Layout)
// =============================================================================

export function useUpdateCheckerState(): UpdateContextValue {
  const [state, setState] = useState<UpdateState>({
    status: 'idle',
    version: null,
    notes: null,
    progress: null,
    error: null,
    lastCheckedAt: localStorage.getItem(STORAGE_KEY),
  })

  // Refs for non-serializable Update object and concurrency guard
  const updateRef = useRef<Update | null>(null)
  const checkingRef = useRef(false)

  const checkForUpdate = useCallback(async () => {
    if (checkingRef.current) return
    checkingRef.current = true

    setState((prev) => ({ ...prev, status: 'checking', error: null }))

    try {
      const update = await updater.checkForUpdate()
      const now = new Date().toISOString()
      localStorage.setItem(STORAGE_KEY, now)

      if (update) {
        updateRef.current = update
        setState((prev) => ({
          ...prev,
          status: 'available',
          version: update.version,
          notes: update.body ?? null,
          lastCheckedAt: now,
        }))
      } else {
        updateRef.current = null
        setState((prev) => ({
          ...prev,
          status: 'up-to-date',
          version: null,
          notes: null,
          lastCheckedAt: now,
        }))
      }
    } catch (err) {
      setState((prev) => ({
        ...prev,
        status: 'error',
        error: err instanceof Error ? err.message : String(err),
      }))
    } finally {
      checkingRef.current = false
    }
  }, [])

  const downloadAndInstall = useCallback(async () => {
    const update = updateRef.current
    if (!update) return

    setState((prev) => ({ ...prev, status: 'downloading', progress: null, error: null }))

    try {
      await updater.downloadAndInstall(update, (progress) => {
        setState((prev) => ({ ...prev, progress }))
      })
      setState((prev) => ({ ...prev, status: 'ready', progress: null }))
    } catch (err) {
      setState((prev) => ({
        ...prev,
        status: 'error',
        error: err instanceof Error ? err.message : String(err),
        progress: null,
      }))
    }
  }, [])

  const relaunchApp = useCallback(async () => {
    await updater.relaunchApp()
  }, [])

  // Periodic check: 30s after mount, then every 24h
  useEffect(() => {
    const lastChecked = localStorage.getItem(STORAGE_KEY)
    const elapsed = lastChecked ? Date.now() - new Date(lastChecked).getTime() : Infinity

    // If enough time has passed, schedule first check after INITIAL_DELAY
    const firstDelay = elapsed >= CHECK_INTERVAL ? INITIAL_DELAY : Math.max(0, CHECK_INTERVAL - elapsed)

    const initialTimer = setTimeout(() => {
      checkForUpdate()
    }, firstDelay)

    const intervalTimer = setInterval(() => {
      checkForUpdate()
    }, CHECK_INTERVAL)

    return () => {
      clearTimeout(initialTimer)
      clearInterval(intervalTimer)
    }
  }, [checkForUpdate])

  return {
    ...state,
    checkForUpdate,
    downloadAndInstall,
    relaunchApp,
  }
}
