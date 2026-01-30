import { useState, useEffect } from 'react'
import { projects as projectsService } from '@/services'
import type { ProjectInfo } from '@/types'

interface UseProjectsOptions {
  showHidden?: boolean
}

interface UseProjectsReturn {
  projects: ProjectInfo[]
  isLoading: boolean
  error: string | null
  refetch: () => Promise<void>
}

export function useProjects(options: UseProjectsOptions = {}): UseProjectsReturn {
  const { showHidden = false } = options
  const [projects, setProjects] = useState<ProjectInfo[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const fetchProjects = async () => {
    try {
      setIsLoading(true)
      setError(null)
      const data = await projectsService.listProjects()

      // Filter hidden projects if needed
      const filtered = showHidden
        ? data
        : data.filter(p => !p.hidden)

      // Sort by total hours descending
      filtered.sort((a, b) => b.total_hours - a.total_hours)

      setProjects(filtered)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load projects')
    } finally {
      setIsLoading(false)
    }
  }

  useEffect(() => {
    fetchProjects()
  }, [showHidden])

  return {
    projects,
    isLoading,
    error,
    refetch: fetchProjects,
  }
}
