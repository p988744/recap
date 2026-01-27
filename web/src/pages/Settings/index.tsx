import { User, Bot, Settings, Sparkles, RefreshCw, FolderGit2 } from 'lucide-react'
import { Cloud } from 'lucide-react'
import {
  useSettings,
  useProfileForm,
  usePreferencesForm,
  useLlmForm,
  useSyncForm,
} from './hooks/useSettings'
import { ProfileSection } from './components/ProfileSection'
import { AccountSection } from './components/AccountSection'
import { ProjectsSection } from './components/ProjectsSection'
import { SyncSection } from './components/SyncSection'
import { AiSection } from './components/AiSection'
import { PreferencesSection } from './components/PreferencesSection'
import { AboutSection } from './components/AboutSection'

const sections = [
  { id: 'profile' as const, label: '個人資料', icon: User },
  { id: 'account' as const, label: '帳號', icon: Cloud },
  { id: 'projects' as const, label: '專案', icon: FolderGit2 },
  { id: 'sync' as const, label: '背景同步', icon: RefreshCw },
  { id: 'ai' as const, label: 'AI 助手', icon: Sparkles },
  { id: 'preferences' as const, label: '偏好設定', icon: Settings },
  { id: 'about' as const, label: '關於', icon: Bot },
]

export function SettingsPage() {
  const settings = useSettings()
  const profileForm = useProfileForm(settings.user)
  const preferencesForm = usePreferencesForm(settings.config)
  const llmForm = useLlmForm(settings.config)
  const syncForm = useSyncForm()

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

        {/* Projects Section */}
        {settings.activeSection === 'projects' && (
          <ProjectsSection />
        )}

        {/* Sync Section */}
        {settings.activeSection === 'sync' && (
          <SyncSection
            enabled={syncForm.enabled}
            setEnabled={syncForm.setEnabled}
            intervalMinutes={syncForm.intervalMinutes}
            setIntervalMinutes={syncForm.setIntervalMinutes}
            syncGit={syncForm.syncGit}
            setSyncGit={syncForm.setSyncGit}
            syncClaude={syncForm.syncClaude}
            setSyncClaude={syncForm.setSyncClaude}
            syncGitlab={syncForm.syncGitlab}
            setSyncGitlab={syncForm.setSyncGitlab}
            syncJira={syncForm.syncJira}
            setSyncJira={syncForm.setSyncJira}
            status={syncForm.status}
            loading={syncForm.loading}
            saving={syncForm.saving}
            onSave={syncForm.handleSave}
            onTriggerSync={syncForm.handleTriggerSync}
            setMessage={settings.setMessage}
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

        {/* Preferences Section */}
        {settings.activeSection === 'preferences' && (
          <PreferencesSection
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

        {/* About Section */}
        {settings.activeSection === 'about' && <AboutSection />}
      </main>
    </div>
  )
}
