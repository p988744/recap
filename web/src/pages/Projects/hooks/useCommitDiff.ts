import { useState, useCallback } from 'react'
import { projects as projectsService } from '@/services'
import type { CommitDiffResponse } from '@/types'

interface UseCommitDiffReturn {
  diff: CommitDiffResponse | null
  isLoading: boolean
  error: string | null
  fetchDiff: (projectPath: string, commitHash: string) => Promise<void>
  clearDiff: () => void
}

export function useCommitDiff(): UseCommitDiffReturn {
  const [diff, setDiff] = useState<CommitDiffResponse | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const fetchDiff = useCallback(async (projectPath: string, commitHash: string) => {
    try {
      setIsLoading(true)
      setError(null)
      const data = await projectsService.getCommitDiff(projectPath, commitHash)
      setDiff(data)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load commit diff')
      setDiff(null)
    } finally {
      setIsLoading(false)
    }
  }, [])

  const clearDiff = useCallback(() => {
    setDiff(null)
    setError(null)
  }, [])

  return {
    diff,
    isLoading,
    error,
    fetchDiff,
    clearDiff,
  }
}
