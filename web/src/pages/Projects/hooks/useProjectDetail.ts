import { useState, useEffect } from 'react'
import { projects as projectsService } from '@/services'
import type { ProjectDetail } from '@/types'

interface UseProjectDetailReturn {
  detail: ProjectDetail | null
  isLoading: boolean
  error: string | null
  refetch: () => Promise<void>
}

export function useProjectDetail(projectName: string): UseProjectDetailReturn {
  const [detail, setDetail] = useState<ProjectDetail | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const fetchDetail = async () => {
    try {
      setIsLoading(true)
      setError(null)
      const data = await projectsService.getProjectDetail(projectName)
      setDetail(data)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load project detail')
    } finally {
      setIsLoading(false)
    }
  }

  useEffect(() => {
    if (projectName) {
      fetchDetail()
    }
  }, [projectName])

  return {
    detail,
    isLoading,
    error,
    refetch: fetchDetail,
  }
}
