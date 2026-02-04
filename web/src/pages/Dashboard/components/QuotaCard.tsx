/**
 * QuotaCard component for Dashboard
 *
 * Displays current Claude Code quota usage with progress bars
 * and alert indicators for warning/critical levels.
 */

import { useState, useEffect } from 'react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Progress } from '@/components/ui/progress'
import { RefreshCw, AlertTriangle, AlertCircle } from 'lucide-react'
import { quota } from '@/services'
import type { QuotaSnapshot, QuotaSettings, AlertLevel } from '@/types/quota'
import { getAlertLevel, formatWindowType, formatResetTime } from '@/types/quota'
import { cn } from '@/lib/utils'

const LOG_PREFIX = '[QuotaCard]'

const DEFAULT_SETTINGS: QuotaSettings = {
  interval_minutes: 15,
  warning_threshold: 80,
  critical_threshold: 95,
  notifications_enabled: true,
}

export function QuotaCard() {
  const [snapshots, setSnapshots] = useState<QuotaSnapshot[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [providerAvailable, setProviderAvailable] = useState(true)

  const fetchQuota = async () => {
    console.log(`${LOG_PREFIX} Fetching quota...`)
    setLoading(true)
    setError(null)

    try {
      const result = await quota.getCurrentQuota()
      console.log(`${LOG_PREFIX} Quota fetched:`, result)
      setSnapshots(result.snapshots)
      setProviderAvailable(result.provider_available)
    } catch (err) {
      console.error(`${LOG_PREFIX} Error:`, err)
      setError(err instanceof Error ? err.message : 'Failed to fetch quota')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    fetchQuota()
  }, [])

  const getAlertColor = (level: AlertLevel) => {
    switch (level) {
      case 'critical':
        return 'text-red-500'
      case 'warning':
        return 'text-yellow-500'
      default:
        return 'text-green-500'
    }
  }

  const getProgressColor = (level: AlertLevel) => {
    switch (level) {
      case 'critical':
        return '[&>div]:bg-red-500'
      case 'warning':
        return '[&>div]:bg-yellow-500'
      default:
        return '[&>div]:bg-green-500'
    }
  }

  const getAlertIcon = (level: AlertLevel) => {
    switch (level) {
      case 'critical':
        return <AlertCircle className="h-4 w-4 text-red-500" />
      case 'warning':
        return <AlertTriangle className="h-4 w-4 text-yellow-500" />
      default:
        return null
    }
  }

  // Filter Claude snapshots
  const claudeSnapshots = snapshots.filter((s) => s.provider === 'claude')
  const primarySnapshot = claudeSnapshots.find((s) => s.window_type === 'five_hour')

  if (!providerAvailable) {
    return (
      <Card className="border-l-2 border-l-muted">
        <CardHeader className="flex flex-row items-center justify-between pb-2">
          <CardTitle className="text-sm font-medium">Quota Usage</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">
            Claude Code not configured. Run `claude` to authenticate.
          </p>
        </CardContent>
      </Card>
    )
  }

  return (
    <Card className="border-l-2 border-l-warm/60">
      <CardHeader className="flex flex-row items-center justify-between pb-2">
        <CardTitle className="text-sm font-medium">Quota Usage</CardTitle>
        <Button
          variant="ghost"
          size="sm"
          onClick={fetchQuota}
          disabled={loading}
          className="h-8 w-8 p-0"
        >
          <RefreshCw className={cn('h-4 w-4', loading && 'animate-spin')} />
        </Button>
      </CardHeader>
      <CardContent>
        {error && <p className="text-sm text-red-500 mb-2">{error}</p>}

        {claudeSnapshots.length === 0 && !loading && !error && (
          <p className="text-sm text-muted-foreground">No quota data available</p>
        )}

        <div className="space-y-3">
          {claudeSnapshots.map((snapshot) => {
            const level = getAlertLevel(snapshot.used_percent, DEFAULT_SETTINGS)
            return (
              <div key={`${snapshot.provider}-${snapshot.window_type}`}>
                <div className="flex items-center justify-between mb-1">
                  <span className="text-sm font-medium flex items-center gap-1">
                    {formatWindowType(snapshot.window_type)}
                    {getAlertIcon(level)}
                  </span>
                  <span className={cn('text-sm font-bold', getAlertColor(level))}>
                    {snapshot.used_percent.toFixed(0)}%
                  </span>
                </div>
                <Progress
                  value={snapshot.used_percent}
                  className={cn('h-2', getProgressColor(level))}
                />
                <p className="text-xs text-muted-foreground mt-1">
                  Resets in {formatResetTime(snapshot.resets_at)}
                </p>
              </div>
            )
          })}
        </div>

        {primarySnapshot?.extra_credits_used != null && (
          <div className="mt-3 pt-3 border-t">
            <p className="text-xs text-muted-foreground">
              Extra credits: ${primarySnapshot.extra_credits_used?.toFixed(2)} / $
              {primarySnapshot.extra_credits_limit?.toFixed(2)}
            </p>
          </div>
        )}
      </CardContent>
    </Card>
  )
}
