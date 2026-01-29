import { Loader2, CheckCircle2, AlertCircle, RefreshCw } from 'lucide-react'
import { Card } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import type { BackgroundSyncStatus } from '@/services/background-sync'
import { ClaudeIcon } from './icons/ClaudeIcon'
import { GeminiIcon } from './icons/GeminiIcon'

type PhaseState = 'idle' | 'syncing' | 'done'

interface DataSyncStatusProps {
  status: BackgroundSyncStatus | null
  enabled: boolean
  dataSyncState: PhaseState
  summaryState: PhaseState
  onTriggerSync: () => void
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

function SourceSyncRow({
  icon,
  label,
  state,
  colorClass,
}: {
  icon: React.ReactNode
  label: string
  state: PhaseState
  colorClass: string
}) {
  return (
    <div className="flex items-center justify-between py-2">
      <div className={`flex items-center gap-2 ${colorClass}`}>
        {icon}
        <span className="text-sm">{label}</span>
      </div>
      <div className="flex items-center gap-1.5">
        {state === 'syncing' ? (
          <>
            <Loader2 className="w-3.5 h-3.5 animate-spin text-foreground" />
            <span className="text-sm font-medium">同步中</span>
          </>
        ) : state === 'done' ? (
          <>
            <CheckCircle2 className="w-3.5 h-3.5 text-sage" />
            <span className="text-sm text-sage">完成</span>
          </>
        ) : (
          <span className="text-sm text-muted-foreground">待執行</span>
        )}
      </div>
    </div>
  )
}

export function DataSyncStatus({
  status,
  enabled,
  dataSyncState,
  summaryState,
  onTriggerSync,
}: DataSyncStatusProps) {
  if (!status) {
    return (
      <Card className="p-4">
        <div className="text-sm text-muted-foreground">載入中...</div>
      </Card>
    )
  }

  const isSyncing = status.is_syncing
  const isActive = enabled && (status.is_running || !!status.last_sync_at)
  const hasError = status.last_error

  // Determine per-source states based on overall dataSyncState
  // When syncing, Claude Code syncs first, then Antigravity
  const claudeState = dataSyncState
  const antigravityState = dataSyncState === 'syncing' ? 'idle' : dataSyncState

  return (
    <Card className="p-4">
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <h3 className="text-sm font-medium">資料同步</h3>
          {isSyncing ? (
            <span className="inline-flex items-center gap-1 text-xs px-1.5 py-0.5 rounded bg-foreground/10">
              <Loader2 className="w-3 h-3 animate-spin" />
              同步中
            </span>
          ) : isActive ? (
            <span className="inline-flex items-center gap-1 text-xs px-1.5 py-0.5 rounded bg-sage/10 text-sage">
              <CheckCircle2 className="w-3 h-3" />
              運行中
            </span>
          ) : (
            <span className="inline-flex items-center gap-1 text-xs px-1.5 py-0.5 rounded bg-foreground/5 text-muted-foreground">
              <AlertCircle className="w-3 h-3" />
              已停止
            </span>
          )}
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={onTriggerSync}
          disabled={isSyncing}
          className="h-7 px-2"
        >
          {isSyncing ? (
            <Loader2 className="w-3.5 h-3.5 animate-spin" />
          ) : (
            <RefreshCw className="w-3.5 h-3.5" />
          )}
          <span className="ml-1.5 text-xs">立即同步</span>
        </Button>
      </div>

      {/* Per-source sync status */}
      <div className="divide-y divide-border">
        <SourceSyncRow
          icon={<ClaudeIcon className="w-4 h-4" />}
          label="Claude Code"
          state={claudeState}
          colorClass="text-amber-600 dark:text-amber-400"
        />
        <SourceSyncRow
          icon={<GeminiIcon className="w-4 h-4" />}
          label="Antigravity"
          state={antigravityState}
          colorClass="text-blue-600 dark:text-blue-400"
        />
      </div>

      {/* Summary processing */}
      {summaryState !== 'idle' && (
        <div className="mt-2 pt-2 border-t border-border">
          <div className="flex items-center justify-between text-sm">
            <span className="text-muted-foreground">摘要處理</span>
            {summaryState === 'syncing' ? (
              <span className="flex items-center gap-1">
                <Loader2 className="w-3 h-3 animate-spin" />
                處理中
              </span>
            ) : (
              <span className="flex items-center gap-1 text-sage">
                <CheckCircle2 className="w-3 h-3" />
                完成
              </span>
            )}
          </div>
        </div>
      )}

      {/* Time info */}
      <div className="mt-3 pt-3 border-t border-border grid grid-cols-2 gap-3 text-xs">
        <div>
          <p className="text-muted-foreground">上次同步</p>
          <p className="font-mono">{formatDateTime(status.last_sync_at)}</p>
        </div>
        <div>
          <p className="text-muted-foreground">下次同步</p>
          <p className="font-mono">{formatTime(status.next_sync_at)}</p>
        </div>
      </div>

      {/* Last result */}
      {status.last_result && (
        <div className="mt-2 text-xs text-muted-foreground">
          {status.last_result}
        </div>
      )}

      {/* Error */}
      {hasError && (
        <div className="mt-2 p-2 bg-destructive/10 text-destructive text-xs border-l-2 border-destructive">
          {status.last_error}
        </div>
      )}
    </Card>
  )
}
