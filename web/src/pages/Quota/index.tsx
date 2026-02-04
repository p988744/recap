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
import { QuotaChart, QuotaSummaryCard } from './components'
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
    windowType,
    setWindowType,
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
              Quota
            </p>
          </div>
          <h1 className="font-display text-3xl text-foreground tracking-tight">
            Quota Usage
          </h1>
        </header>

        <Card className="border-l-2 border-l-muted">
          <CardContent className="pt-6">
            <p className="text-muted-foreground">
              Claude Code not configured. Run <code className="bg-muted px-1 rounded">claude</code> in
              your terminal to authenticate.
            </p>
          </CardContent>
        </Card>
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
                Quota
              </p>
            </div>
            <h1 className="font-display text-3xl text-foreground tracking-tight">
              Quota Usage
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
            Refresh
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
              Current Usage
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
                  Usage History
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

                {/* Window type filter */}
                <Select value={windowType} onValueChange={setWindowType}>
                  <SelectTrigger className="w-32 h-8 text-xs">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="five_hour">5 Hour</SelectItem>
                    <SelectItem value="seven_day">7 Day</SelectItem>
                    <SelectItem value="seven_day_opus">Opus</SelectItem>
                    <SelectItem value="seven_day_sonnet">Sonnet</SelectItem>
                    <SelectItem value="monthly">Monthly</SelectItem>
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
                    <SelectItem value="1">1 Day</SelectItem>
                    <SelectItem value="3">3 Days</SelectItem>
                    <SelectItem value="7">7 Days</SelectItem>
                    <SelectItem value="14">14 Days</SelectItem>
                    <SelectItem value="30">30 Days</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>
          </CardHeader>
          <CardContent>
            {loading && history.length === 0 ? (
              <div className="flex items-center justify-center h-[300px]">
                <div className="w-5 h-5 border border-border border-t-foreground/60 rounded-full animate-spin" />
              </div>
            ) : history.length > 0 ? (
              <QuotaChart data={history} settings={DEFAULT_QUOTA_SETTINGS} />
            ) : (
              <div className="flex flex-col items-center justify-center h-[300px] text-muted-foreground">
                <History className="w-8 h-8 mb-2 opacity-50" />
                <p>No history data available</p>
                <p className="text-xs mt-1">
                  Quota data will appear here after tracking starts
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
