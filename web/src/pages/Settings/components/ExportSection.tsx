import type { ConfigResponse } from '@/types'
import type { SettingsMessage } from '../hooks/types'
import { JiraTempoCard } from './IntegrationsSection/JiraTempoCard'

interface ExportSectionProps {
  config: ConfigResponse | null
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
  showToken: boolean
  setShowToken: (v: boolean) => void
  saving: boolean
  testing: boolean
  onSave: (
    setMessage: (msg: SettingsMessage | null) => void,
    refreshConfig: () => Promise<ConfigResponse>
  ) => Promise<void>
  onTest: (setMessage: (msg: SettingsMessage | null) => void) => Promise<void>
  setMessage: (msg: SettingsMessage | null) => void
  refreshConfig: () => Promise<ConfigResponse>
}

export function ExportSection({
  config,
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
  showToken,
  setShowToken,
  saving,
  testing,
  onSave,
  onTest,
  setMessage,
  refreshConfig,
}: ExportSectionProps) {
  return (
    <div className="space-y-8">
      <div>
        <h2 className="font-display text-lg text-foreground tracking-tight">
          整合 (Integrations)
        </h2>
        <p className="text-sm text-muted-foreground mt-1">
          連接外部平台，同步工作紀錄
        </p>
      </div>

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
        showToken={showToken}
        setShowToken={setShowToken}
        saving={saving}
        testing={testing}
        onSave={onSave}
        onTest={onTest}
        setMessage={setMessage}
        refreshConfig={refreshConfig}
      />
    </div>
  )
}
