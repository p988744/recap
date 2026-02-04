/**
 * Quota Page
 *
 * Dedicated page for viewing Claude Code quota usage history.
 * Displays current quota summary cards and a history chart with filters.
 */

import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { RefreshCw, Gauge, History } from 'lucide-react'
import { useQuotaPage, DEFAULT_QUOTA_SETTINGS } from './hooks'
import { QuotaChart, QuotaSummaryCard, ClaudeAuthConfig, QuotaStats } from './components'
import { cn } from '@/lib/utils'

export function QuotaPage() {
  const {
    currentQuota,
    providerAvailable,
    history,
    loading,
    error,
    provider,
    setProvider,
    days,
    setDays,
    refresh,
  } = useQuotaPage()

  // Filter snapshots by provider for summary cards
  const claudeSnapshots = currentQuota.filter((s) => s.provider === 'claude')
  const antigravitySnapshots = currentQuota.filter((s) => s.provider === 'antigravity')

  if (!providerAvailable) {
    return (
      <div className="space-y-6">
        <header className="animate-fade-up opacity-0 delay-1">
          <div className="flex items-center gap-2 mb-2">
            <Gauge className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
            <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
              配額
            </p>
          </div>
          <h1 className="font-display text-3xl text-foreground tracking-tight">
            配額使用量
          </h1>
        </header>

        <Card className="border-l-2 border-l-muted">
          <CardContent className="pt-6 space-y-3">
            <p className="text-muted-foreground">
              尚未設定 Claude Code OAuth 認證。
            </p>
            <p className="text-sm text-muted-foreground">
              此功能需要 Claude Max 訂閱用戶的 OAuth token。請在終端機執行{' '}
              <code className="bg-muted px-1.5 py-0.5 rounded text-foreground">claude /login</code>{' '}
              進行認證，或在下方手動輸入 Token。
            </p>
          </CardContent>
        </Card>

        {/* Manual OAuth token configuration */}
        <ClaudeAuthConfig onAuthStatusChange={refresh} />
      </div>
    )
  }

  return (
    <div className="space-y-8">
      {/* Header */}
      <header className="animate-fade-up opacity-0 delay-1">
        <div className="flex items-center justify-between">
          <div>
            <div className="flex items-center gap-2 mb-2">
              <Gauge className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
              <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
                配額
              </p>
            </div>
            <h1 className="font-display text-3xl text-foreground tracking-tight">
              配額使用量
            </h1>
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={refresh}
            disabled={loading}
            className="gap-2"
          >
            <RefreshCw className={cn('h-4 w-4', loading && 'animate-spin')} />
            重新整理
          </Button>
        </div>
      </header>

      {/* Error display */}
      {error && (
        <div className="p-4 border-l-2 border-l-destructive bg-destructive/5 text-destructive text-sm animate-fade-up">
          {error}
        </div>
      )}

      {/* Loading state */}
      {loading && currentQuota.length === 0 && (
        <div className="flex items-center justify-center h-48">
          <div className="w-6 h-6 border border-border border-t-foreground/60 rounded-full animate-spin" />
        </div>
      )}

      {/* Current Quota Summary */}
      {currentQuota.length > 0 && (
        <section className="animate-fade-up opacity-0 delay-2">
          <div className="flex items-center gap-2 mb-4">
            <Gauge className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
            <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
              目前用量
            </h2>
          </div>

          {/* Claude snapshots */}
          {claudeSnapshots.length > 0 && (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-4">
              {claudeSnapshots.map((snapshot) => (
                <QuotaSummaryCard
                  key={`${snapshot.provider}-${snapshot.window_type}`}
                  snapshot={snapshot}
                  settings={DEFAULT_QUOTA_SETTINGS}
                />
              ))}
            </div>
          )}

          {/* Antigravity snapshots (if any) */}
          {antigravitySnapshots.length > 0 && (
            <>
              <p className="text-xs text-muted-foreground mb-2 mt-4">Antigravity</p>
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
                {antigravitySnapshots.map((snapshot) => (
                  <QuotaSummaryCard
                    key={`${snapshot.provider}-${snapshot.window_type}`}
                    snapshot={snapshot}
                    settings={DEFAULT_QUOTA_SETTINGS}
                  />
                ))}
              </div>
            </>
          )}
        </section>
      )}

      {/* History Chart */}
      <section className="animate-fade-up opacity-0 delay-3">
        <Card>
          <CardHeader>
            <div className="flex items-center justify-between flex-wrap gap-4">
              <div className="flex items-center gap-2">
                <History className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
                <CardTitle className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground font-normal">
                  使用歷史
                </CardTitle>
              </div>
              <div className="flex gap-2 flex-wrap">
                {/* Provider filter */}
                <Select value={provider} onValueChange={setProvider}>
                  <SelectTrigger className="w-32 h-8 text-xs">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="claude">Claude</SelectItem>
                    <SelectItem value="antigravity">Antigravity</SelectItem>
                  </SelectContent>
                </Select>

                {/* Days filter */}
                <Select
                  value={days.toString()}
                  onValueChange={(v) => setDays(parseInt(v, 10))}
                >
                  <SelectTrigger className="w-28 h-8 text-xs">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="1">1 天</SelectItem>
                    <SelectItem value="3">3 天</SelectItem>
                    <SelectItem value="7">7 天</SelectItem>
                    <SelectItem value="14">14 天</SelectItem>
                    <SelectItem value="30">30 天</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>
          </CardHeader>
          <CardContent className="space-y-4">
            {/* Statistics */}
            {currentQuota.length > 0 && (
              <QuotaStats
                currentQuota={currentQuota}
                historyData={history}
              />
            )}

            {/* Chart */}
            {loading && history.length === 0 ? (
              <div className="flex items-center justify-center h-[300px]">
                <div className="w-5 h-5 border border-border border-t-foreground/60 rounded-full animate-spin" />
              </div>
            ) : history.length > 0 ? (
              <QuotaChart
                data={history}
                settings={DEFAULT_QUOTA_SETTINGS}
                currentQuota={currentQuota}
              />
            ) : (
              <div className="flex flex-col items-center justify-center h-[300px] text-muted-foreground">
                <History className="w-8 h-8 mb-2 opacity-50" />
                <p>尚無歷史資料</p>
                <p className="text-xs mt-1">
                  開始追蹤後，配額資料將顯示在此
                </p>
              </div>
            )}
          </CardContent>
        </Card>
      </section>
    </div>
  )
}

export default QuotaPage
