import { Loader2, CheckCircle2, AlertCircle, RefreshCw } from 'lucide-react'
import { Card } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Progress } from '@/components/ui/progress'
import type { BackgroundSyncStatus, SyncProgress } from '@/services/background-sync'
import { ClaudeIcon } from './icons/ClaudeIcon'
import { GeminiIcon } from './icons/GeminiIcon'

type PhaseState = 'idle' | 'syncing' | 'done'

interface DataSyncStatusProps {
  status: BackgroundSyncStatus | null
  enabled: boolean
  dataSyncState: PhaseState
  summaryState: PhaseState
  syncProgress: SyncProgress | null
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

const phaseLabels: Record<SyncProgress['phase'], string> = {
  sources: '同步資料來源',
  snapshots: '捕獲快照',
  compaction: '處理摘要',
  summaries: '生成時間軸摘要',
  complete: '完成',
}

export function DataSyncStatus({
  status,
  enabled,
  dataSyncState,
  summaryState,
  syncProgress,
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

  // Calculate progress percentage
  const progressPercent = syncProgress
    ? syncProgress.total > 0
      ? Math.round((syncProgress.current / syncProgress.total) * 100)
      : syncProgress.phase === 'complete' ? 100 : 0
    : 0

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

      {/* Progress bar during sync */}
      {syncProgress && syncProgress.phase !== 'complete' && (
        <div className="mt-3 pt-3 border-t border-border space-y-2">
          <div className="flex items-center justify-between text-xs">
            <span className="text-muted-foreground">
              {phaseLabels[syncProgress.phase]}
              {syncProgress.current_source && `: ${syncProgress.current_source}`}
            </span>
            <span className="font-mono text-foreground">
              {syncProgress.current}/{syncProgress.total}
            </span>
          </div>
          <Progress value={progressPercent} className="h-1.5" />
          <p className="text-xs text-muted-foreground truncate">
            {syncProgress.message}
          </p>
        </div>
      )}

      {/* Summary processing (when no detailed progress) */}
      {!syncProgress && summaryState !== 'idle' && (
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

// =============================================================================
// Data Compaction Status Component
// =============================================================================

type CompactionPhase = 'idle' | 'hourly' | 'timeline' | 'done'

interface CompactionResult {
  hourly_compacted: number
  daily_compacted: number
  weekly_compacted: number
  monthly_compacted: number
  /** Latest date that was compacted (YYYY-MM-DD format) */
  latest_compacted_date: string | null
}

interface DataCompactionStatusProps {
  status: BackgroundSyncStatus | null
  enabled: boolean
  autoGenerateSummaries: boolean
  compactionPhase?: CompactionPhase
  compactionResult?: CompactionResult | null
  onTriggerCompaction?: () => void
}

export function DataCompactionStatus({
  status,
  enabled,
  autoGenerateSummaries,
  compactionPhase = 'idle',
  compactionResult = null,
  onTriggerCompaction,
}: DataCompactionStatusProps) {
  if (!status || !autoGenerateSummaries) {
    return null
  }

  const isCompacting = status.is_compacting || (compactionPhase !== 'idle' && compactionPhase !== 'done')
  const isActive = enabled && autoGenerateSummaries
  const hasCompletedBefore = !!status.last_compaction_at

  // Determine phase states
  // Priority: manual phase > backend is_compacting > completed history > idle
  const hourlyState: PhaseState =
    compactionPhase === 'hourly' ? 'syncing' :
    compactionPhase === 'timeline' || compactionPhase === 'done' ? 'done' :
    status.is_compacting ? 'syncing' :
    hasCompletedBefore ? 'done' : 'idle'

  const timelineState: PhaseState =
    compactionPhase === 'timeline' ? 'syncing' :
    compactionPhase === 'done' ? 'done' :
    status.is_compacting ? 'syncing' :
    hasCompletedBefore ? 'done' : 'idle'

  // Format date to user-friendly format (e.g., "1月15日")
  const formatCompactedDate = (dateStr: string | null): string | null => {
    if (!dateStr) return null
    try {
      const date = new Date(dateStr)
      const month = date.getMonth() + 1
      const day = date.getDate()
      return `${month}月${day}日`
    } catch {
      return null
    }
  }

  // Format result message
  const getResultMessage = () => {
    if (!compactionResult) return null
    const { hourly_compacted, daily_compacted, weekly_compacted, monthly_compacted, latest_compacted_date } = compactionResult
    const total = hourly_compacted + daily_compacted + weekly_compacted + monthly_compacted
    if (total === 0) return '沒有需要壓縮的資料'

    const parts: string[] = []
    if (hourly_compacted > 0) parts.push(`${hourly_compacted} 個小時`)
    if (daily_compacted > 0) parts.push(`${daily_compacted} 個每日`)
    if (weekly_compacted > 0) parts.push(`${weekly_compacted} 個每週`)
    if (monthly_compacted > 0) parts.push(`${monthly_compacted} 個月度`)

    const formattedDate = formatCompactedDate(latest_compacted_date)
    if (formattedDate) {
      return `已壓縮到 ${formattedDate} 的資料（${parts.join('、')} 摘要）`
    }
    return `已壓縮 ${parts.join('、')} 摘要`
  }

  return (
    <Card className="p-4">
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <h3 className="text-sm font-medium">資料壓縮</h3>
          {isCompacting ? (
            <span className="inline-flex items-center gap-1 text-xs px-1.5 py-0.5 rounded bg-foreground/10">
              <Loader2 className="w-3 h-3 animate-spin" />
              壓縮中
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
        {onTriggerCompaction && (
          <Button
            variant="ghost"
            size="sm"
            onClick={onTriggerCompaction}
            disabled={isCompacting}
            className="h-7 px-2"
          >
            {isCompacting ? (
              <Loader2 className="w-3.5 h-3.5 animate-spin" />
            ) : (
              <RefreshCw className="w-3.5 h-3.5" />
            )}
            <span className="ml-1.5 text-xs">立即壓縮</span>
          </Button>
        )}
      </div>

      {/* Compaction phases - using divide-y like DataSyncStatus */}
      <div className="divide-y divide-border">
        <div className="flex items-center justify-between py-2">
          <div className="flex items-center gap-2 text-foreground">
            <span className="text-sm">每小時 → 每日摘要</span>
          </div>
          <div className="flex items-center gap-1.5">
            {hourlyState === 'syncing' ? (
              <>
                <Loader2 className="w-3.5 h-3.5 animate-spin text-foreground" />
                <span className="text-sm font-medium">處理中</span>
              </>
            ) : hourlyState === 'done' ? (
              <>
                <CheckCircle2 className="w-3.5 h-3.5 text-sage" />
                <span className="text-sm text-sage">完成</span>
              </>
            ) : (
              <span className="text-sm text-muted-foreground">尚未執行</span>
            )}
          </div>
        </div>
        <div className="flex items-center justify-between py-2">
          <div className="flex items-center gap-2 text-foreground">
            <span className="text-sm">時間軸摘要（週/月/季/年）</span>
          </div>
          <div className="flex items-center gap-1.5">
            {timelineState === 'syncing' ? (
              <>
                <Loader2 className="w-3.5 h-3.5 animate-spin text-foreground" />
                <span className="text-sm font-medium">處理中</span>
              </>
            ) : timelineState === 'done' ? (
              <>
                <CheckCircle2 className="w-3.5 h-3.5 text-sage" />
                <span className="text-sm text-sage">完成</span>
              </>
            ) : hourlyState === 'syncing' ? (
              <span className="text-sm text-muted-foreground">等待中</span>
            ) : (
              <span className="text-sm text-muted-foreground">尚未執行</span>
            )}
          </div>
        </div>
      </div>

      {/* Time info - matching DataSyncStatus layout */}
      <div className="mt-3 pt-3 border-t border-border grid grid-cols-2 gap-3 text-xs">
        <div>
          <p className="text-muted-foreground">上次壓縮</p>
          <p className="font-mono">{formatDateTime(status.last_compaction_at)}</p>
        </div>
        <div>
          <p className="text-muted-foreground">下次壓縮</p>
          <p className="font-mono">{formatTime(status.next_compaction_at)}</p>
        </div>
      </div>

      {/* Last result - like DataSyncStatus */}
      {compactionResult && compactionPhase === 'done' && (
        <div className="mt-2 text-xs text-muted-foreground">
          {getResultMessage()}
        </div>
      )}
    </Card>
  )
}
