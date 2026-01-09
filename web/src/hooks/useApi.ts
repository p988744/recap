import { useState, useEffect, useCallback } from 'react'

interface UseApiOptions<T> {
  immediate?: boolean
  onSuccess?: (data: T) => void
  onError?: (error: Error) => void
}

interface UseApiResult<T> {
  data: T | null
  loading: boolean
  error: Error | null
  execute: () => Promise<T | null>
  reset: () => void
}

export function useApi<T>(
  apiCall: () => Promise<T>,
  options: UseApiOptions<T> = {}
): UseApiResult<T> {
  const { immediate = true, onSuccess, onError } = options
  const [data, setData] = useState<T | null>(null)
  const [loading, setLoading] = useState(immediate)
  const [error, setError] = useState<Error | null>(null)

  const execute = useCallback(async () => {
    setLoading(true)
    setError(null)

    try {
      const result = await apiCall()
      setData(result)
      onSuccess?.(result)
      return result
    } catch (err) {
      const error = err instanceof Error ? err : new Error(String(err))
      setError(error)
      onError?.(error)
      return null
    } finally {
      setLoading(false)
    }
  }, [apiCall, onSuccess, onError])

  const reset = useCallback(() => {
    setData(null)
    setError(null)
    setLoading(false)
  }, [])

  useEffect(() => {
    if (immediate) {
      execute()
    }
  }, []) // eslint-disable-line react-hooks/exhaustive-deps

  return { data, loading, error, execute, reset }
}

// Polling hook for live updates
export function usePolling<T>(
  apiCall: () => Promise<T>,
  interval: number,
  enabled: boolean = true
) {
  const [data, setData] = useState<T | null>(null)
  const [error, setError] = useState<Error | null>(null)

  useEffect(() => {
    if (!enabled) return

    const fetchData = async () => {
      try {
        const result = await apiCall()
        setData(result)
        setError(null)
      } catch (err) {
        setError(err instanceof Error ? err : new Error(String(err)))
      }
    }

    fetchData()
    const id = setInterval(fetchData, interval)

    return () => clearInterval(id)
  }, [apiCall, interval, enabled])

  return { data, error }
}
