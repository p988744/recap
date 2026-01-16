import { createContext, useContext, useEffect, type ReactNode } from 'react'
import type { ConfigResponse, SourcesResponse } from '@/types'
import type { SettingsMessage } from '../hooks'
import { useJiraForm } from '../hooks/useJiraForm'
import { useGitLabForm } from '../hooks/useGitLabForm'
import { useGitRepoForm } from '../hooks/useGitRepoForm'
import { useClaudeCodeForm } from '../hooks/useClaudeCodeForm'

interface IntegrationsContextValue {
  // Common
  config: ConfigResponse | null
  sources: SourcesResponse | null
  setSources: (sources: SourcesResponse) => void
  setMessage: (msg: SettingsMessage | null) => void
  refreshConfig: () => Promise<ConfigResponse>
  refreshSources: () => Promise<SourcesResponse>
  isAuthenticated: boolean
  // Git repos
  gitRepo: ReturnType<typeof useGitRepoForm>
  // Claude Code
  claudeCode: ReturnType<typeof useClaudeCodeForm>
  // Jira
  jira: ReturnType<typeof useJiraForm>
  // GitLab
  gitlab: ReturnType<typeof useGitLabForm>
}

const IntegrationsContext = createContext<IntegrationsContextValue | null>(null)

interface IntegrationsProviderProps {
  children: ReactNode
  config: ConfigResponse | null
  sources: SourcesResponse | null
  setSources: (sources: SourcesResponse) => void
  setMessage: (msg: SettingsMessage | null) => void
  refreshConfig: () => Promise<ConfigResponse>
  refreshSources: () => Promise<SourcesResponse>
  isAuthenticated: boolean
}

export function IntegrationsProvider({
  children,
  config,
  sources,
  setSources,
  setMessage,
  refreshConfig,
  refreshSources,
  isAuthenticated,
}: IntegrationsProviderProps) {
  const gitRepo = useGitRepoForm()
  const claudeCode = useClaudeCodeForm()
  const jira = useJiraForm(config)
  const gitlab = useGitLabForm(config)

  // Auto-load Claude sessions and GitLab projects
  useEffect(() => {
    if (!isAuthenticated) return

    if (claudeCode.projects.length === 0 && !claudeCode.loading) {
      claudeCode.loadSessions(sources, setMessage, refreshSources)
    }
    if (config?.gitlab_configured && gitlab.projects.length === 0) {
      gitlab.loadProjects()
    }
  }, [config?.gitlab_configured, isAuthenticated])

  return (
    <IntegrationsContext.Provider
      value={{
        config,
        sources,
        setSources,
        setMessage,
        refreshConfig,
        refreshSources,
        isAuthenticated,
        gitRepo,
        claudeCode,
        jira,
        gitlab,
      }}
    >
      {children}
    </IntegrationsContext.Provider>
  )
}

export function useIntegrations() {
  const context = useContext(IntegrationsContext)
  if (!context) {
    throw new Error('useIntegrations must be used within IntegrationsProvider')
  }
  return context
}
