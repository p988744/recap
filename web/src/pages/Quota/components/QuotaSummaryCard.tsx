/**
 * QuotaSummaryCard component
 *
 * Displays a single quota snapshot with progress bar and status.
 */

import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Progress } from '@/components/ui/progress'
import { AlertTriangle, AlertCircle } from 'lucide-react'
import type { QuotaSnapshot, QuotaSettings, AlertLevel } from '@/types/quota'
import { getAlertLevel, formatWindowType, formatResetTime } from '@/types/quota'
import { cn } from '@/lib/utils'

interface QuotaSummaryCardProps {
  snapshot: QuotaSnapshot
  settings: QuotaSettings
}

export function QuotaSummaryCard({ snapshot, settings }: QuotaSummaryCardProps) {
  const level = getAlertLevel(snapshot.used_percent, settings)

  const getAlertColor = (alertLevel: AlertLevel) => {
    switch (alertLevel) {
      case 'critical':
        return 'text-red-500'
      case 'warning':
        return 'text-yellow-500'
      default:
        return 'text-green-500'
    }
  }

  const getProgressColor = (alertLevel: AlertLevel) => {
    switch (alertLevel) {
      case 'critical':
        return '[&>div]:bg-red-500'
      case 'warning':
        return '[&>div]:bg-yellow-500'
      default:
        return '[&>div]:bg-green-500'
    }
  }

  const getBorderColor = (alertLevel: AlertLevel) => {
    switch (alertLevel) {
      case 'critical':
        return 'border-l-red-500'
      case 'warning':
        return 'border-l-yellow-500'
      default:
        return 'border-l-green-500'
    }
  }

  const getAlertIcon = (alertLevel: AlertLevel) => {
    switch (alertLevel) {
      case 'critical':
        return <AlertCircle className="h-4 w-4 text-red-500" />
      case 'warning':
        return <AlertTriangle className="h-4 w-4 text-yellow-500" />
      default:
        return null
    }
  }

  const providerLabel = snapshot.provider === 'claude' ? 'Claude' : 'Antigravity'

  return (
    <Card className={cn('border-l-2', getBorderColor(level))}>
      <CardHeader className="pb-2">
        <CardTitle className="text-sm font-medium flex items-center justify-between">
          <span className="flex items-center gap-2">
            {providerLabel} - {formatWindowType(snapshot.window_type)}
            {getAlertIcon(level)}
          </span>
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-2">
          <div className="flex items-baseline justify-between">
            <span className={cn('text-3xl font-bold', getAlertColor(level))}>
              {snapshot.used_percent.toFixed(0)}%
            </span>
          </div>
          <Progress
            value={snapshot.used_percent}
            className={cn('h-2', getProgressColor(level))}
          />
          {snapshot.resets_at && (
            <p className="text-xs text-muted-foreground">
              Resets in {formatResetTime(snapshot.resets_at)}
            </p>
          )}
          {snapshot.extra_credits_used != null && (
            <p className="text-xs text-muted-foreground pt-1 border-t">
              Extra credits: ${snapshot.extra_credits_used.toFixed(2)} / $
              {snapshot.extra_credits_limit?.toFixed(2)}
            </p>
          )}
        </div>
      </CardContent>
    </Card>
  )
}
