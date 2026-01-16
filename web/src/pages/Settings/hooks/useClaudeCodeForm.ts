import { useEffect, useState } from 'react'
import { sources as sourcesService, claude } from '@/services'
import type { SourcesResponse, ClaudeProject } from '@/types'
import type { SettingsMessage } from './types'

export function useClaudeCodeForm() {
  const [projects, setProjects] = useState<ClaudeProject[]>([])
  const [loading, setLoading] = useState(false)
  const [selectedProjects, setSelectedProjects] = useState<Set<string>>(() => {
    const saved = localStorage.getItem('recap-selected-claude-projects')
    return saved ? new Set(JSON.parse(saved)) : new Set()
  })
  const [expandedProjects, setExpandedProjects] = useState<Set<string>>(new Set())
  const [importing, setImporting] = useState(false)

  useEffect(() => {
    localStorage.setItem('recap-selected-claude-projects', JSON.stringify(Array.from(selectedProjects)))
  }, [selectedProjects])

  const loadSessions = async (
    sources: SourcesResponse | null,
    setMessage: (msg: SettingsMessage | null) => void,
    refreshSources: () => Promise<SourcesResponse>
  ) => {
    setLoading(true)
    setMessage(null)
    try {
      const projectsList = await claude.listSessions()
      setProjects(projectsList)
      if (projectsList.length > 0) {
        setExpandedProjects(new Set([projectsList[0].path]))
      }

      // Auto-add detected Git repos
      if (projectsList.length > 0 && sources) {
        const existingPaths = sources.git_repos?.map(r => r.path) || []
        const newProjects = projectsList.filter(p => !existingPaths.includes(p.path))

        if (newProjects.length > 0) {
          const addPromises = newProjects.map(p =>
            sourcesService.addGitRepo(p.path).catch(() => null)
          )
          await Promise.all(addPromises)
          await refreshSources()
          setMessage({
            type: 'success',
            text: `已自動新增 ${newProjects.length} 個 Git 倉庫`
          })
        }
      }
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '載入失敗' })
    } finally {
      setLoading(false)
    }
  }

  const toggleExpandProject = (path: string) => {
    setExpandedProjects(prev => {
      const next = new Set(prev)
      if (next.has(path)) next.delete(path)
      else next.add(path)
      return next
    })
  }

  const toggleProjectSelection = (path: string) => {
    setSelectedProjects(prev => {
      const next = new Set(prev)
      if (next.has(path)) next.delete(path)
      else next.add(path)
      return next
    })
  }

  const selectAllProjects = () => {
    setSelectedProjects(new Set(projects.map(p => p.path)))
  }

  const clearSelection = () => {
    setSelectedProjects(new Set())
  }

  const selectedSessionCount = projects
    .filter(p => selectedProjects.has(p.path))
    .reduce((acc, p) => acc + p.sessions.length, 0)

  const handleImport = async (setMessage: (msg: SettingsMessage | null) => void) => {
    if (selectedProjects.size === 0) return
    setImporting(true)
    setMessage(null)
    try {
      const sessionIds = projects
        .filter(p => selectedProjects.has(p.path))
        .flatMap(p => p.sessions.map(s => s.session_id))

      const result = await claude.importSessions({ session_ids: sessionIds })
      setMessage({
        type: 'success',
        text: `已匯入 ${result.imported} 個 session，建立 ${result.work_items_created} 個工作項目`,
      })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '匯入失敗' })
    } finally {
      setImporting(false)
    }
  }

  return {
    projects,
    loading,
    selectedProjects,
    expandedProjects,
    importing,
    selectedSessionCount,
    loadSessions,
    toggleExpandProject,
    toggleProjectSelection,
    selectAllProjects,
    clearSelection,
    handleImport,
  }
}
