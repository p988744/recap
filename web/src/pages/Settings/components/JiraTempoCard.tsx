import {
  Link2,
  CheckCircle2,
  XCircle,
  Save,
  Loader2,
  Eye,
  EyeOff,
} from 'lucide-react'
import { Card } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import type { ConfigResponse } from '@/types'
import type { SettingsMessage } from '../hooks/useSettings'

interface JiraTempoCardProps {
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

export function JiraTempoCard({
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
}: JiraTempoCardProps) {
  return (
    <Card className="p-6">
      <div className="flex items-center gap-3 mb-6">
        <div className="w-10 h-10 rounded-lg bg-blue-500/10 flex items-center justify-center">
          <Link2 className="w-5 h-5 text-blue-600" strokeWidth={1.5} />
        </div>
        <div className="flex-1">
          <h3 className="font-medium text-foreground">Jira / Tempo</h3>
          <p className="text-xs text-muted-foreground">工時記錄與同步</p>
        </div>
        {config?.jira_configured ? (
          <span className="flex items-center gap-1.5 text-xs text-sage">
            <CheckCircle2 className="w-3.5 h-3.5" strokeWidth={1.5} />
            已連接
          </span>
        ) : (
          <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <XCircle className="w-3.5 h-3.5" strokeWidth={1.5} />
            未設定
          </span>
        )}
      </div>

      <div className="space-y-4">
        <div>
          <Label htmlFor="jira-url" className="mb-2 block text-xs">Jira URL</Label>
          <Input
            id="jira-url"
            type="url"
            value={jiraUrl}
            onChange={(e) => setJiraUrl(e.target.value)}
            placeholder="https://your-company.atlassian.net"
          />
        </div>

        <div>
          <Label className="mb-2 block text-xs">認證方式</Label>
          <div className="flex items-center gap-4">
            <label className="flex items-center gap-2 cursor-pointer">
              <input
                type="radio"
                name="auth-type"
                checked={jiraAuthType === 'pat'}
                onChange={() => setJiraAuthType('pat')}
                className="w-4 h-4 accent-foreground"
              />
              <span className="text-sm">PAT</span>
            </label>
            <label className="flex items-center gap-2 cursor-pointer">
              <input
                type="radio"
                name="auth-type"
                checked={jiraAuthType === 'basic'}
                onChange={() => setJiraAuthType('basic')}
                className="w-4 h-4 accent-foreground"
              />
              <span className="text-sm">Basic Auth</span>
            </label>
          </div>
        </div>

        {jiraAuthType === 'basic' && (
          <div>
            <Label htmlFor="jira-email" className="mb-2 block text-xs">Email</Label>
            <Input
              id="jira-email"
              type="email"
              value={jiraEmail}
              onChange={(e) => setJiraEmail(e.target.value)}
              placeholder="your-email@company.com"
            />
          </div>
        )}

        <div>
          <Label htmlFor="jira-token" className="mb-2 block text-xs">
            {jiraAuthType === 'pat' ? 'Personal Access Token' : 'API Token'}
          </Label>
          <div className="relative">
            <Input
              id="jira-token"
              type={showToken ? 'text' : 'password'}
              value={jiraToken}
              onChange={(e) => setJiraToken(e.target.value)}
              placeholder={config?.jira_configured ? '••••••••（已設定）' : '輸入 Token'}
              className="pr-10"
            />
            <button
              type="button"
              onClick={() => setShowToken(!showToken)}
              className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
            >
              {showToken ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
            </button>
          </div>
        </div>

        <div>
          <Label htmlFor="tempo-token" className="mb-2 block text-xs">
            Tempo API Token <span className="text-muted-foreground">(選用)</span>
          </Label>
          <Input
            id="tempo-token"
            type="password"
            value={tempoToken}
            onChange={(e) => setTempoToken(e.target.value)}
            placeholder={config?.tempo_configured ? '••••••••（已設定）' : '留空使用 Jira worklog'}
          />
        </div>

        <div className="flex items-center gap-3 pt-4 border-t border-border">
          <Button variant="outline" onClick={() => onSave(setMessage, refreshConfig)} disabled={saving}>
            {saving ? <Loader2 className="w-4 h-4 animate-spin" /> : <Save className="w-4 h-4" />}
            儲存
          </Button>
          <Button variant="ghost" onClick={() => onTest(setMessage)} disabled={testing || !config?.jira_configured}>
            {testing ? <Loader2 className="w-4 h-4 animate-spin" /> : <Link2 className="w-4 h-4" />}
            測試連線
          </Button>
        </div>
      </div>
    </Card>
  )
}
