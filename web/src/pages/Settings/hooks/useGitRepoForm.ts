import { useState } from 'react'
import { sources as sourcesService } from '@/services'
import type { SourcesResponse } from '@/types'
import type { SettingsMessage } from './types'

export function useGitRepoForm() {
  const [newRepoPath, setNewRepoPath] = useState('')
  const [adding, setAdding] = useState(false)

  const handleAdd = async (
    setMessage: (msg: SettingsMessage | null) => void,
    refreshSources: () => Promise<SourcesResponse>
  ) => {
    if (!newRepoPath.trim()) return
    setAdding(true)
    setMessage(null)
    try {
      await sourcesService.addGitRepo(newRepoPath.trim())
      await refreshSources()
      setNewRepoPath('')
      setMessage({ type: 'success', text: '已新增 Git 倉庫' })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '新增失敗' })
    } finally {
      setAdding(false)
    }
  }

  const handleRemove = async (
    repoId: string,
    setMessage: (msg: SettingsMessage | null) => void,
    refreshSources: () => Promise<SourcesResponse>
  ) => {
    setMessage(null)
    try {
      await sourcesService.removeGitRepo(repoId)
      await refreshSources()
      setMessage({ type: 'success', text: '已移除 Git 倉庫' })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '移除失敗' })
    }
  }

  return {
    newRepoPath,
    setNewRepoPath,
    adding,
    handleAdd,
    handleRemove,
  }
}
