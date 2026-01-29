import { User, Bot, Settings, Sparkles, FolderGit2, Plug } from 'lucide-react'
import {
  useSettings,
  useProfileForm,
  usePreferencesForm,
  useLlmForm,
  useSyncForm,
  useJiraForm,
} from './hooks/useSettings'
import { ProfileSection } from './components/ProfileSection'
import { ProjectsSection } from './components/ProjectsSection'
import { SyncSection } from './components/SyncSection'
import { ExportSection } from './components/ExportSection'
import { AiSection } from './components/AiSection'
import { AboutSection } from './components/AboutSection'

const sections = [
  { id: 'profile' as const, label: '帳號', icon: User },
  { id: 'projects' as const, label: '專案', icon: FolderGit2 },
  { id: 'sync' as const, label: '系統設定', icon: Settings },
  { id: 'export' as const, label: '整合', icon: Plug },
  { id: 'ai' as const, label: 'AI 助手', icon: Sparkles },
  { id: 'about' as const, label: '關於', icon: Bot },
]

export function SettingsPage() {
  const settings = useSettings()
  const profileForm = useProfileForm(settings.user)
  const preferencesForm = usePreferencesForm(settings.config)
  const llmForm = useLlmForm(settings.config)
  const syncForm = useSyncForm()
  const jiraForm = useJiraForm(settings.config)

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
      <aside className="w-48 shrink-0 sticky top-10 self-start">
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

        {/* Profile & Account Section */}
        {settings.activeSection === 'profile' && (
          <ProfileSection
            {...profileForm}
            onSave={profileForm.handleSave}
            setMessage={settings.setMessage}
            user={settings.user}
            onLogout={settings.logout}
          />
        )}

        {/* Projects Section */}
        {settings.activeSection === 'projects' && (
          <ProjectsSection
            syncStatus={syncForm.status}
            syncEnabled={syncForm.enabled}
            dataSyncState={syncForm.dataSyncState}
            summaryState={syncForm.summaryState}
            onTriggerSync={() => syncForm.handleTriggerSync(settings.setMessage)}
          />
        )}

        {/* Sync + Preferences Section */}
        {settings.activeSection === 'sync' && (
          <SyncSection
            enabled={syncForm.enabled}
            setEnabled={syncForm.setEnabled}
            intervalMinutes={syncForm.intervalMinutes}
            setIntervalMinutes={syncForm.setIntervalMinutes}
            loading={syncForm.loading}
            saving={syncForm.saving}
            onSave={syncForm.handleSave}
            dailyHours={preferencesForm.dailyHours}
            setDailyHours={preferencesForm.setDailyHours}
            normalizeHours={preferencesForm.normalizeHours}
            setNormalizeHours={preferencesForm.setNormalizeHours}
            timezone={preferencesForm.timezone}
            setTimezone={preferencesForm.setTimezone}
            weekStartDay={preferencesForm.weekStartDay}
            setWeekStartDay={preferencesForm.setWeekStartDay}
            savingPreferences={preferencesForm.saving}
            onSavePreferences={preferencesForm.handleSave}
            setMessage={settings.setMessage}
          />
        )}

        {/* Export Section */}
        {settings.activeSection === 'export' && (
          <ExportSection
            config={settings.config}
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
            showToken={jiraForm.showToken}
            setShowToken={jiraForm.setShowToken}
            saving={jiraForm.saving}
            testing={jiraForm.testing}
            onSave={jiraForm.handleSave}
            onTest={jiraForm.handleTest}
            setMessage={settings.setMessage}
            refreshConfig={settings.refreshConfig}
          />
        )}

        {/* AI Section */}
        {settings.activeSection === 'ai' && (
          <AiSection
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
