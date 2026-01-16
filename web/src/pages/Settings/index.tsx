import { useEffect } from 'react'
import { User, Link2, Bot, Settings } from 'lucide-react'
import { Cloud } from 'lucide-react'
import {
  useSettings,
  useProfileForm,
  usePreferencesForm,
  useLlmForm,
  useJiraForm,
  useGitLabForm,
  useGitRepoForm,
  useClaudeCodeForm,
  type SettingsSection,
} from './hooks/useSettings'
import { ProfileSection } from './components/ProfileSection'
import { AccountSection } from './components/AccountSection'
import { IntegrationsSection } from './components/IntegrationsSection'
import { PreferencesSection } from './components/PreferencesSection'
import { AboutSection } from './components/AboutSection'

const sections = [
  { id: 'profile' as const, label: '個人資料', icon: User },
  { id: 'account' as const, label: '帳號', icon: Cloud },
  { id: 'integrations' as const, label: '整合服務', icon: Link2 },
  { id: 'preferences' as const, label: '偏好設定', icon: Settings },
  { id: 'about' as const, label: '關於', icon: Bot },
]

export function SettingsPage() {
  const settings = useSettings()
  const profileForm = useProfileForm(settings.user)
  const preferencesForm = usePreferencesForm(settings.config)
  const llmForm = useLlmForm(settings.config)
  const jiraForm = useJiraForm(settings.config)
  const gitlabForm = useGitLabForm(settings.config)
  const gitRepoForm = useGitRepoForm()
  const claudeCodeForm = useClaudeCodeForm()

  // Auto-load Claude sessions and GitLab projects when viewing integrations
  useEffect(() => {
    if (!settings.isAuthenticated || settings.activeSection !== 'integrations') {
      return
    }
    if (claudeCodeForm.projects.length === 0 && !claudeCodeForm.loading) {
      claudeCodeForm.loadSessions(settings.sources, settings.setMessage, settings.refreshSources)
    }
    if (settings.config?.gitlab_configured && gitlabForm.projects.length === 0) {
      gitlabForm.loadProjects()
    }
  }, [settings.activeSection, settings.config?.gitlab_configured, settings.isAuthenticated])

  if (settings.loading) {
    return (
      <div className="flex items-center justify-center h-96">
        <div className="w-6 h-6 border border-border border-t-charcoal/60 rounded-full animate-spin" />
      </div>
    )
  }

  return (
    <div className="flex gap-8">
      {/* Sidebar Navigation */}
      <aside className="w-48 shrink-0">
        <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-4">
          設定
        </p>
        <nav className="space-y-1">
          {sections.map((section) => (
            <button
              key={section.id}
              onClick={() => {
                settings.setActiveSection(section.id)
                settings.setMessage(null)
              }}
              className={`w-full flex items-center gap-3 px-3 py-2 text-sm transition-colors ${
                settings.activeSection === section.id
                  ? 'text-foreground bg-foreground/5 border-l-2 border-foreground -ml-px'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
            >
              <section.icon className="w-4 h-4" strokeWidth={1.5} />
              {section.label}
            </button>
          ))}
        </nav>
      </aside>

      {/* Content */}
      <main className="flex-1 max-w-2xl">
        {/* Message */}
        {settings.message && (
          <div className={`mb-6 p-3 text-sm ${
            settings.message.type === 'success'
              ? 'bg-sage/10 text-sage border-l-2 border-sage'
              : 'bg-destructive/10 text-destructive border-l-2 border-destructive'
          }`}>
            {settings.message.text}
          </div>
        )}

        {/* Profile Section */}
        {settings.activeSection === 'profile' && (
          <ProfileSection
            {...profileForm}
            onSave={profileForm.handleSave}
            setMessage={settings.setMessage}
          />
        )}

        {/* Account Section */}
        {settings.activeSection === 'account' && (
          <AccountSection
            user={settings.user}
            appStatus={settings.appStatus}
            onLogout={settings.logout}
          />
        )}

        {/* Integrations Section */}
        {settings.activeSection === 'integrations' && (
          <IntegrationsSection
            config={settings.config}
            sources={settings.sources}
            setSources={settings.setSources}
            setMessage={settings.setMessage}
            refreshConfig={settings.refreshConfig}
            refreshSources={settings.refreshSources}
            // Git repos
            claudeProjects={claudeCodeForm.projects}
            newRepoPath={gitRepoForm.newRepoPath}
            setNewRepoPath={gitRepoForm.setNewRepoPath}
            addingRepo={gitRepoForm.adding}
            onAddRepo={gitRepoForm.handleAdd}
            onRemoveRepo={gitRepoForm.handleRemove}
            // Claude Code
            claudeLoading={claudeCodeForm.loading}
            selectedClaudeProjects={claudeCodeForm.selectedProjects}
            expandedClaudeProjects={claudeCodeForm.expandedProjects}
            importingClaude={claudeCodeForm.importing}
            selectedClaudeSessionCount={claudeCodeForm.selectedSessionCount}
            onLoadClaudeSessions={claudeCodeForm.loadSessions}
            onToggleExpandClaude={claudeCodeForm.toggleExpandProject}
            onToggleSelectionClaude={claudeCodeForm.toggleProjectSelection}
            onSelectAllClaude={claudeCodeForm.selectAllProjects}
            onClearSelectionClaude={claudeCodeForm.clearSelection}
            onImportClaude={claudeCodeForm.handleImport}
            // Jira
            jiraUrl={jiraForm.jiraUrl}
            setJiraUrl={jiraForm.setJiraUrl}
            jiraAuthType={jiraForm.jiraAuthType}
            setJiraAuthType={jiraForm.setJiraAuthType}
            jiraToken={jiraForm.jiraToken}
            setJiraToken={jiraForm.setJiraToken}
            jiraEmail={jiraForm.jiraEmail}
            setJiraEmail={jiraForm.setJiraEmail}
            tempoToken={jiraForm.tempoToken}
            setTempoToken={jiraForm.setTempoToken}
            showJiraToken={jiraForm.showToken}
            setShowJiraToken={jiraForm.setShowToken}
            savingJira={jiraForm.saving}
            testingJira={jiraForm.testing}
            onSaveJira={jiraForm.handleSave}
            onTestJira={jiraForm.handleTest}
            // GitLab
            gitlabUrl={gitlabForm.gitlabUrl}
            setGitlabUrl={gitlabForm.setGitlabUrl}
            gitlabToken={gitlabForm.gitlabToken}
            setGitlabToken={gitlabForm.setGitlabToken}
            showGitlabToken={gitlabForm.showToken}
            setShowGitlabToken={gitlabForm.setShowToken}
            savingGitlab={gitlabForm.saving}
            testingGitlab={gitlabForm.testing}
            gitlabProjects={gitlabForm.projects}
            gitlabSearchResults={gitlabForm.searchResults}
            gitlabSearch={gitlabForm.search}
            setGitlabSearch={gitlabForm.setSearch}
            searchingGitlab={gitlabForm.searching}
            syncingGitlab={gitlabForm.syncing}
            onSaveGitlab={gitlabForm.handleSave}
            onTestGitlab={gitlabForm.handleTest}
            onSearchGitlab={gitlabForm.handleSearch}
            onAddGitlabProject={gitlabForm.handleAddProject}
            onRemoveGitlabProject={gitlabForm.handleRemoveProject}
            onSyncGitlab={gitlabForm.handleSync}
            onRemoveGitlabConfig={gitlabForm.handleRemoveConfig}
          />
        )}

        {/* Preferences Section */}
        {settings.activeSection === 'preferences' && (
          <PreferencesSection
            dailyHours={preferencesForm.dailyHours}
            setDailyHours={preferencesForm.setDailyHours}
            normalizeHours={preferencesForm.normalizeHours}
            setNormalizeHours={preferencesForm.setNormalizeHours}
            savingPreferences={preferencesForm.saving}
            onSavePreferences={preferencesForm.handleSave}
            config={settings.config}
            llmProvider={llmForm.llmProvider}
            llmModel={llmForm.llmModel}
            setLlmModel={llmForm.setLlmModel}
            llmApiKey={llmForm.llmApiKey}
            setLlmApiKey={llmForm.setLlmApiKey}
            llmBaseUrl={llmForm.llmBaseUrl}
            setLlmBaseUrl={llmForm.setLlmBaseUrl}
            showLlmKey={llmForm.showLlmKey}
            setShowLlmKey={llmForm.setShowLlmKey}
            savingLlm={llmForm.saving}
            onProviderChange={llmForm.handleProviderChange}
            onSaveLlm={llmForm.handleSave}
            setMessage={settings.setMessage}
            refreshConfig={settings.refreshConfig}
          />
        )}

        {/* About Section */}
        {settings.activeSection === 'about' && <AboutSection />}
      </main>
    </div>
  )
}
