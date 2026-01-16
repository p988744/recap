import type { ConfigResponse, SourcesResponse, ClaudeProject, GitLabProject, GitLabProjectInfo } from '@/types'
import type { SettingsMessage } from '../../hooks/useSettings'
import { GitRepoCard } from './GitRepoCard'
import { ClaudeCodeCard } from './ClaudeCodeCard'
import { JiraTempoCard } from './JiraTempoCard'
import { GitLabCard } from './GitLabCard'

interface IntegrationsSectionProps {
  // Common
  config: ConfigResponse | null
  sources: SourcesResponse | null
  setSources: (sources: SourcesResponse) => void
  setMessage: (msg: SettingsMessage | null) => void
  refreshConfig: () => Promise<ConfigResponse>
  refreshSources: () => Promise<SourcesResponse>
  // Git repos
  claudeProjects: ClaudeProject[]
  newRepoPath: string
  setNewRepoPath: (v: string) => void
  addingRepo: boolean
  onAddRepo: (
    setMessage: (msg: SettingsMessage | null) => void,
    refreshSources: () => Promise<SourcesResponse>
  ) => Promise<void>
  onRemoveRepo: (
    repoId: string,
    setMessage: (msg: SettingsMessage | null) => void,
    refreshSources: () => Promise<SourcesResponse>
  ) => Promise<void>
  // Claude Code
  claudeLoading: boolean
  selectedClaudeProjects: Set<string>
  expandedClaudeProjects: Set<string>
  importingClaude: boolean
  selectedClaudeSessionCount: number
  onLoadClaudeSessions: (
    sources: SourcesResponse | null,
    setMessage: (msg: SettingsMessage | null) => void,
    refreshSources: () => Promise<SourcesResponse>
  ) => Promise<void>
  onToggleExpandClaude: (path: string) => void
  onToggleSelectionClaude: (path: string) => void
  onSelectAllClaude: () => void
  onClearSelectionClaude: () => void
  onImportClaude: (setMessage: (msg: SettingsMessage | null) => void) => Promise<void>
  // Jira
  jiraUrl: string
  setJiraUrl: (v: string) => void
  jiraAuthType: 'pat' | 'basic'
  setJiraAuthType: (v: 'pat' | 'basic') => void
  jiraToken: string
  setJiraToken: (v: string) => void
  jiraEmail: string
  setJiraEmail: (v: string) => void
  tempoToken: string
  setTempoToken: (v: string) => void
  showJiraToken: boolean
  setShowJiraToken: (v: boolean) => void
  savingJira: boolean
  testingJira: boolean
  onSaveJira: (
    setMessage: (msg: SettingsMessage | null) => void,
    refreshConfig: () => Promise<ConfigResponse>
  ) => Promise<void>
  onTestJira: (setMessage: (msg: SettingsMessage | null) => void) => Promise<void>
  // GitLab
  gitlabUrl: string
  setGitlabUrl: (v: string) => void
  gitlabToken: string
  setGitlabToken: (v: string) => void
  showGitlabToken: boolean
  setShowGitlabToken: (v: boolean) => void
  savingGitlab: boolean
  testingGitlab: boolean
  gitlabProjects: GitLabProject[]
  gitlabSearchResults: GitLabProjectInfo[]
  gitlabSearch: string
  setGitlabSearch: (v: string) => void
  searchingGitlab: boolean
  syncingGitlab: boolean
  onSaveGitlab: (
    setMessage: (msg: SettingsMessage | null) => void,
    refreshConfig: () => Promise<ConfigResponse>
  ) => Promise<void>
  onTestGitlab: (setMessage: (msg: SettingsMessage | null) => void) => Promise<void>
  onSearchGitlab: (setMessage: (msg: SettingsMessage | null) => void) => Promise<void>
  onAddGitlabProject: (projectId: number, setMessage: (msg: SettingsMessage | null) => void) => Promise<void>
  onRemoveGitlabProject: (id: string, setMessage: (msg: SettingsMessage | null) => void) => Promise<void>
  onSyncGitlab: (setMessage: (msg: SettingsMessage | null) => void) => Promise<void>
  onRemoveGitlabConfig: (
    setMessage: (msg: SettingsMessage | null) => void,
    refreshConfig: () => Promise<ConfigResponse>
  ) => Promise<void>
}

export function IntegrationsSection({
  config,
  sources,
  setSources,
  setMessage,
  refreshConfig,
  refreshSources,
  // Git repos
  claudeProjects,
  newRepoPath,
  setNewRepoPath,
  addingRepo,
  onAddRepo,
  onRemoveRepo,
  // Claude Code
  claudeLoading,
  selectedClaudeProjects,
  expandedClaudeProjects,
  importingClaude,
  selectedClaudeSessionCount,
  onLoadClaudeSessions,
  onToggleExpandClaude,
  onToggleSelectionClaude,
  onSelectAllClaude,
  onClearSelectionClaude,
  onImportClaude,
  // Jira
  jiraUrl,
  setJiraUrl,
  jiraAuthType,
  setJiraAuthType,
  jiraToken,
  setJiraToken,
  jiraEmail,
  setJiraEmail,
  tempoToken,
  setTempoToken,
  showJiraToken,
  setShowJiraToken,
  savingJira,
  testingJira,
  onSaveJira,
  onTestJira,
  // GitLab
  gitlabUrl,
  setGitlabUrl,
  gitlabToken,
  setGitlabToken,
  showGitlabToken,
  setShowGitlabToken,
  savingGitlab,
  testingGitlab,
  gitlabProjects,
  gitlabSearchResults,
  gitlabSearch,
  setGitlabSearch,
  searchingGitlab,
  syncingGitlab,
  onSaveGitlab,
  onTestGitlab,
  onSearchGitlab,
  onAddGitlabProject,
  onRemoveGitlabProject,
  onSyncGitlab,
  onRemoveGitlabConfig,
}: IntegrationsSectionProps) {
  return (
    <section className="space-y-8 animate-fade-up opacity-0 delay-1">
      <h2 className="font-display text-2xl text-foreground">整合服務</h2>

      <GitRepoCard
        sources={sources}
        claudeProjects={claudeProjects}
        newRepoPath={newRepoPath}
        setNewRepoPath={setNewRepoPath}
        adding={addingRepo}
        onAdd={onAddRepo}
        onRemove={onRemoveRepo}
        setMessage={setMessage}
        refreshSources={refreshSources}
        setSources={setSources}
      />

      <ClaudeCodeCard
        projects={claudeProjects}
        loading={claudeLoading}
        selectedProjects={selectedClaudeProjects}
        expandedProjects={expandedClaudeProjects}
        importing={importingClaude}
        selectedSessionCount={selectedClaudeSessionCount}
        onLoadSessions={onLoadClaudeSessions}
        onToggleExpand={onToggleExpandClaude}
        onToggleSelection={onToggleSelectionClaude}
        onSelectAll={onSelectAllClaude}
        onClearSelection={onClearSelectionClaude}
        onImport={onImportClaude}
        sources={sources}
        setMessage={setMessage}
        refreshSources={refreshSources}
      />

      <JiraTempoCard
        config={config}
        jiraUrl={jiraUrl}
        setJiraUrl={setJiraUrl}
        jiraAuthType={jiraAuthType}
        setJiraAuthType={setJiraAuthType}
        jiraToken={jiraToken}
        setJiraToken={setJiraToken}
        jiraEmail={jiraEmail}
        setJiraEmail={setJiraEmail}
        tempoToken={tempoToken}
        setTempoToken={setTempoToken}
        showToken={showJiraToken}
        setShowToken={setShowJiraToken}
        saving={savingJira}
        testing={testingJira}
        onSave={onSaveJira}
        onTest={onTestJira}
        setMessage={setMessage}
        refreshConfig={refreshConfig}
      />

      <GitLabCard
        config={config}
        gitlabUrl={gitlabUrl}
        setGitlabUrl={setGitlabUrl}
        gitlabToken={gitlabToken}
        setGitlabToken={setGitlabToken}
        showToken={showGitlabToken}
        setShowToken={setShowGitlabToken}
        saving={savingGitlab}
        testing={testingGitlab}
        projects={gitlabProjects}
        searchResults={gitlabSearchResults}
        search={gitlabSearch}
        setSearch={setGitlabSearch}
        searching={searchingGitlab}
        syncing={syncingGitlab}
        onSave={onSaveGitlab}
        onTest={onTestGitlab}
        onSearch={onSearchGitlab}
        onAddProject={onAddGitlabProject}
        onRemoveProject={onRemoveGitlabProject}
        onSync={onSyncGitlab}
        onRemoveConfig={onRemoveGitlabConfig}
        setMessage={setMessage}
        refreshConfig={refreshConfig}
      />
    </section>
  )
}
