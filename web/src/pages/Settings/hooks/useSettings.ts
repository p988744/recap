import { useEffect, useState } from 'react'
import { config as configService, sources as sourcesService } from '@/services'
import type { ConfigResponse, SourcesResponse } from '@/types'
import { useAuth } from '@/lib/auth'
import type { SettingsSection, SettingsMessage } from './types'

export function useSettings() {
  const { user, logout, appStatus, token, isAuthenticated } = useAuth()
  const [activeSection, setActiveSection] = useState<SettingsSection>('profile')
  const [config, setConfig] = useState<ConfigResponse | null>(null)
  const [sources, setSources] = useState<SourcesResponse | null>(null)
  const [loading, setLoading] = useState(true)
  const [message, setMessage] = useState<SettingsMessage | null>(null)

  useEffect(() => {
    if (!isAuthenticated || !token) return

    async function fetchData() {
      try {
        const [configData, sourcesData] = await Promise.all([
          configService.getConfig(),
          sourcesService.getSources(),
        ])
        setConfig(configData)
        setSources(sourcesData)
      } catch (err) {
        console.error('Failed to fetch data:', err)
        setMessage({ type: 'error', text: err instanceof Error ? err.message : '載入設定失敗' })
      } finally {
        setLoading(false)
      }
    }
    fetchData()
  }, [isAuthenticated, token])

  const refreshConfig = async () => {
    const updated = await configService.getConfig()
    setConfig(updated)
    return updated
  }

  const refreshSources = async () => {
    const updated = await sourcesService.getSources()
    setSources(updated)
    return updated
  }

  return {
    // Auth
    user,
    logout,
    appStatus,
    isAuthenticated,
    // State
    activeSection,
    setActiveSection,
    config,
    setConfig,
    sources,
    setSources,
    loading,
    message,
    setMessage,
    // Refresh helpers
    refreshConfig,
    refreshSources,
  }
}

// Re-export types
export type { SettingsSection, SettingsMessage } from './types'

// Re-export all hooks
export { useProfileForm } from './useProfileForm'
export { usePreferencesForm } from './usePreferencesForm'
export { useLlmForm } from './useLlmForm'
export { useJiraForm } from './useJiraForm'
export { useGitLabForm } from './useGitLabForm'
export { useGitRepoForm } from './useGitRepoForm'
export { useClaudeCodeForm } from './useClaudeCodeForm'

// Re-export utilities
export { formatFileSize, formatTimestamp } from './utils'
