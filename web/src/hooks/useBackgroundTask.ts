/**
 * Background Task Hook & Context
 *
 * Manages long-running background tasks like recompaction.
 * Shared between the task initiator (DangerZoneSection) and
 * the sidebar status display (Layout).
 */
import { createContext, useContext, useState, useCallback, useRef } from 'react'
import type { RecompactProgress } from '@/services/danger-zone'

// =============================================================================
// Types
// =============================================================================

export interface BackgroundTaskState {
  /** Unique identifier for the task type */
  taskType: 'recompact' | null
  /** Whether the task is currently running */
  isRunning: boolean
  /** Current progress */
  progress: RecompactProgress | null
  /** Error message if task failed */
  error: string | null
}

export interface BackgroundTaskContextValue {
  /** Current task state */
  task: BackgroundTaskState
  /** Start a background task */
  startTask: (taskType: 'recompact') => void
  /** Update task progress */
  updateProgress: (progress: RecompactProgress) => void
  /** Complete the task */
  completeTask: () => void
  /** Set task error */
  setTaskError: (error: string) => void
  /** Clear the task */
  clearTask: () => void
}

// =============================================================================
// Context
// =============================================================================

const BackgroundTaskContext = createContext<BackgroundTaskContextValue | null>(null)

export const BackgroundTaskProvider = BackgroundTaskContext.Provider

/**
 * Consume background task state from any component.
 * Must be used inside a BackgroundTaskProvider (Layout).
 */
export function useBackgroundTask(): BackgroundTaskContextValue {
  const ctx = useContext(BackgroundTaskContext)
  if (!ctx) {
    throw new Error('useBackgroundTask must be used within a BackgroundTaskProvider')
  }
  return ctx
}

// =============================================================================
// Hook (used by Layout)
// =============================================================================

const initialState: BackgroundTaskState = {
  taskType: null,
  isRunning: false,
  progress: null,
  error: null,
}

export function useBackgroundTaskState(): BackgroundTaskContextValue {
  const [task, setTask] = useState<BackgroundTaskState>(initialState)
  const taskRef = useRef<BackgroundTaskState>(task)
  taskRef.current = task

  const startTask = useCallback((taskType: 'recompact') => {
    setTask({
      taskType,
      isRunning: true,
      progress: null,
      error: null,
    })
  }, [])

  const updateProgress = useCallback((progress: RecompactProgress) => {
    setTask(prev => ({
      ...prev,
      progress,
    }))
  }, [])

  const completeTask = useCallback(() => {
    setTask(prev => ({
      ...prev,
      isRunning: false,
    }))
    // Auto-clear after 3 seconds if task is complete
    setTimeout(() => {
      if (!taskRef.current.isRunning && taskRef.current.progress?.phase === 'complete') {
        setTask(initialState)
      }
    }, 3000)
  }, [])

  const setTaskError = useCallback((error: string) => {
    setTask(prev => ({
      ...prev,
      isRunning: false,
      error,
    }))
  }, [])

  const clearTask = useCallback(() => {
    setTask(initialState)
  }, [])

  return {
    task,
    startTask,
    updateProgress,
    completeTask,
    setTaskError,
    clearTask,
  }
}

// =============================================================================
// Helper Labels
// =============================================================================

export const taskTypeLabels: Record<string, string> = {
  recompact: '重新計算摘要',
}

export const phaseLabels: Record<RecompactProgress['phase'], string> = {
  counting: '統計現有摘要',
  scanning: '掃描快照',
  hourly: '處理小時摘要',
  daily: '處理每日摘要',
  monthly: '處理月度摘要',
  complete: '完成',
}
