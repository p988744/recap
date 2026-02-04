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

      // Sort by latest activity date (most recent first)
      filtered.sort((a, b) => {
        const dateA = a.latest_date || '0000-00-00'
        const dateB = b.latest_date || '0000-00-00'
        return dateB.localeCompare(dateA)
      })

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
