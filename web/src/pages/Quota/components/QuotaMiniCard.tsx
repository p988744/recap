/**
 * QuotaMiniCard Component
 *
 * A compact quota display with progress bar for the overview page.
 * Shows window type, usage percentage, and visual progress indicator.
 */

import { Progress } from '@/components/ui/progress'
import {
  QuotaSnapshot,
  QuotaSettings,
  formatWindowType,
  getAlertLevel,
} from '@/types/quota'
import { cn } from '@/lib/utils'

interface QuotaMiniCardProps {
  snapshot: QuotaSnapshot
  settings: QuotaSettings
}

export function QuotaMiniCard({ snapshot, settings }: QuotaMiniCardProps) {
  const alertLevel = getAlertLevel(snapshot.used_percent, settings)

  // Get color based on alert level
  const getProgressColor = () => {
    switch (alertLevel) {
      case 'critical':
        return 'bg-destructive'
      case 'warning':
        return 'bg-yellow-500'
      default:
        return 'bg-primary'
    }
  }

  const getTextColor = () => {
    switch (alertLevel) {
      case 'critical':
        return 'text-destructive'
      case 'warning':
        return 'text-yellow-600 dark:text-yellow-500'
      default:
        return 'text-foreground'
    }
  }

  return (
    <div className="space-y-1.5">
      <div className="flex items-center justify-between text-xs">
        <span className="text-muted-foreground">
          {formatWindowType(snapshot.window_type)}
        </span>
        <span className={cn('font-medium tabular-nums', getTextColor())}>
          {Math.round(snapshot.used_percent)}%
        </span>
      </div>
      <Progress
        value={snapshot.used_percent}
        className="h-1.5"
        indicatorClassName={getProgressColor()}
      />
    </div>
  )
}

export default QuotaMiniCard
