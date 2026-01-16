import { Save, Loader2, RefreshCw, Clock, CheckCircle2, AlertCircle } from 'lucide-react'
import { Card } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Label } from '@/components/ui/label'
import type { SettingsMessage } from '../hooks/useSettings'
import type { BackgroundSyncStatus } from '@/services/background-sync'

// =============================================================================
// Types
// =============================================================================

interface SyncSectionProps {
  // Config
  enabled: boolean
  setEnabled: (v: boolean) => void
  intervalMinutes: number
  setIntervalMinutes: (v: number) => void
  syncGit: boolean
  setSyncGit: (v: boolean) => void
  syncClaude: boolean
  setSyncClaude: (v: boolean) => void
  syncGitlab: boolean
  setSyncGitlab: (v: boolean) => void
  syncJira: boolean
  setSyncJira: (v: boolean) => void
  // Status
  status: BackgroundSyncStatus | null
  // UI
  loading: boolean
  saving: boolean
  // Actions
  onSave: (setMessage: (msg: SettingsMessage | null) => void) => Promise<void>
  onTriggerSync: (setMessage: (msg: SettingsMessage | null) => void) => Promise<void>
  setMessage: (msg: SettingsMessage | null) => void
}

// =============================================================================
// Helpers
// =============================================================================

function formatTime(isoString: string | null): string {
  if (!isoString) return '-'
  try {
    const date = new Date(isoString)
    return date.toLocaleTimeString('zh-TW', {
      hour: '2-digit',
      minute: '2-digit',
    })
  } catch {
    return '-'
  }
}

function formatDateTime(isoString: string | null): string {
  if (!isoString) return '-'
  try {
    const date = new Date(isoString)
    return date.toLocaleString('zh-TW', {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    })
  } catch {
    return '-'
  }
}

// =============================================================================
// Components
// =============================================================================

function Toggle({
  checked,
  onChange,
  label,
  description,
  disabled,
}: {
  checked: boolean
  onChange: (v: boolean) => void
  label: string
  description?: string
  disabled?: boolean
}) {
  return (
    <label className={`flex items-center gap-3 ${disabled ? 'opacity-50' : 'cursor-pointer'}`}>
      <div className="relative">
        <input
          type="checkbox"
          checked={checked}
          onChange={(e) => onChange(e.target.checked)}
          disabled={disabled}
          className="sr-only peer"
        />
        <div className="w-10 h-5 bg-foreground/15 peer-checked:bg-foreground transition-colors" />
        <div className="absolute top-0.5 left-0.5 w-4 h-4 bg-white transition-transform peer-checked:translate-x-5" />
      </div>
      <div>
        <span className="text-sm text-foreground">{label}</span>
        {description && <p className="text-xs text-muted-foreground">{description}</p>}
      </div>
    </label>
  )
}

function StatusCard({ status }: { status: BackgroundSyncStatus | null }) {
  if (!status) {
    return (
      <div className="p-4 bg-foreground/5 text-sm text-muted-foreground">
        載入中...
      </div>
    )
  }

  const isActive = status.is_running
  const isSyncing = status.is_syncing
  const hasError = status.last_error

  return (
    <div className="space-y-3">
      {/* Status Badge */}
      <div className="flex items-center gap-2">
        {isSyncing ? (
          <>
            <Loader2 className="w-4 h-4 animate-spin text-foreground" />
            <span className="text-sm font-medium">同步中...</span>
          </>
        ) : isActive ? (
          <>
            <CheckCircle2 className="w-4 h-4 text-sage" />
            <span className="text-sm font-medium text-sage">運行中</span>
          </>
        ) : (
          <>
            <AlertCircle className="w-4 h-4 text-muted-foreground" />
            <span className="text-sm text-muted-foreground">已停止</span>
          </>
        )}
      </div>

      {/* Status Details */}
      <div className="grid grid-cols-2 gap-4 text-sm">
        <div>
          <p className="text-muted-foreground">上次同步</p>
          <p className="font-mono">{formatDateTime(status.last_sync_at)}</p>
        </div>
        <div>
          <p className="text-muted-foreground">下次同步</p>
          <p className="font-mono">{formatTime(status.next_sync_at)}</p>
        </div>
      </div>

      {/* Last Result */}
      {status.last_result && (
        <div className="text-sm">
          <p className="text-muted-foreground">上次結果</p>
          <p>{status.last_result}</p>
        </div>
      )}

      {/* Error Message */}
      {hasError && (
        <div className="p-2 bg-destructive/10 text-destructive text-sm border-l-2 border-destructive">
          {status.last_error}
        </div>
      )}
    </div>
  )
}

// =============================================================================
// Main Component
// =============================================================================

export function SyncSection({
  enabled,
  setEnabled,
  intervalMinutes,
  setIntervalMinutes,
  syncGit,
  setSyncGit,
  syncClaude,
  setSyncClaude,
  syncGitlab,
  setSyncGitlab,
  syncJira,
  setSyncJira,
  status,
  loading,
  saving,
  onSave,
  onTriggerSync,
  setMessage,
}: SyncSectionProps) {
  if (loading) {
    return (
      <section className="animate-fade-up opacity-0 delay-1">
        <h2 className="font-display text-2xl text-foreground mb-6">背景同步</h2>
        <div className="flex items-center justify-center h-48">
          <Loader2 className="w-6 h-6 animate-spin text-muted-foreground" />
        </div>
      </section>
    )
  }

  return (
    <section className="animate-fade-up opacity-0 delay-1">
      <h2 className="font-display text-2xl text-foreground mb-6">背景同步</h2>

      {/* Status Card */}
      <Card className="p-6 mb-6">
        <div className="flex items-center justify-between mb-4">
          <h3 className="font-medium">同步狀態</h3>
          <Button
            variant="outline"
            size="sm"
            onClick={() => onTriggerSync(setMessage)}
            disabled={status?.is_syncing}
          >
            {status?.is_syncing ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <RefreshCw className="w-4 h-4" />
            )}
            立即同步
          </Button>
        </div>
        <StatusCard status={status} />
      </Card>

      {/* Configuration Card */}
      <Card className="p-6">
        <h3 className="font-medium mb-4">同步設定</h3>

        <div className="space-y-6">
          {/* Enable Toggle */}
          <Toggle
            checked={enabled}
            onChange={setEnabled}
            label="啟用背景同步"
            description="自動定時同步工作項目"
          />

          {/* Interval Selection */}
          <div className={enabled ? '' : 'opacity-50 pointer-events-none'}>
            <Label className="mb-2 block">同步間隔</Label>
            <div className="flex items-center gap-2">
              <Clock className="w-4 h-4 text-muted-foreground" />
              <select
                value={intervalMinutes}
                onChange={(e) => setIntervalMinutes(Number(e.target.value))}
                className="px-3 py-2 bg-background border border-border text-sm focus:outline-none focus:ring-1 focus:ring-foreground"
                disabled={!enabled}
              >
                <option value={5}>每 5 分鐘</option>
                <option value={15}>每 15 分鐘</option>
                <option value={30}>每 30 分鐘</option>
                <option value={60}>每小時</option>
              </select>
            </div>
          </div>

          {/* Source Toggles */}
          <div className={enabled ? '' : 'opacity-50 pointer-events-none'}>
            <Label className="mb-3 block">同步來源</Label>
            <div className="space-y-3">
              <Toggle
                checked={syncClaude}
                onChange={setSyncClaude}
                label="Claude Code"
                description="同步 Claude Code 工作階段"
                disabled={!enabled}
              />
              <Toggle
                checked={syncGit}
                onChange={setSyncGit}
                label="本地 Git"
                description="同步本地 Git 提交記錄"
                disabled={!enabled}
              />
              <Toggle
                checked={syncGitlab}
                onChange={setSyncGitlab}
                label="GitLab"
                description="同步 GitLab 提交記錄（需先設定連線）"
                disabled={!enabled}
              />
              <Toggle
                checked={syncJira}
                onChange={setSyncJira}
                label="Jira / Tempo"
                description="同步 Jira 工作記錄（需先設定連線）"
                disabled={!enabled}
              />
            </div>
          </div>

          {/* Save Button */}
          <div className="pt-4 border-t border-border">
            <Button onClick={() => onSave(setMessage)} disabled={saving}>
              {saving ? (
                <Loader2 className="w-4 h-4 animate-spin" />
              ) : (
                <Save className="w-4 h-4" />
              )}
              {saving ? '儲存中...' : '儲存設定'}
            </Button>
          </div>
        </div>
      </Card>
    </section>
  )
}
