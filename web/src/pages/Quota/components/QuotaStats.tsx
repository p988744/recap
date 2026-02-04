/**
 * QuotaStats component
 *
 * Displays usage statistics for quota data including
 * average and maximum usage per period.
 */

import { useMemo } from 'react'
import { TrendingUp, TrendingDown, BarChart3, Activity } from 'lucide-react'
import type { QuotaSnapshot } from '@/types/quota'
import { cn } from '@/lib/utils'

interface QuotaStatsProps {
  data: QuotaSnapshot[]
  windowType: string
}

interface StatCardProps {
  label: string
  value: string
  subValue?: string
  icon: React.ReactNode
  trend?: 'up' | 'down' | 'neutral'
}

function StatCard({ label, value, subValue, icon, trend }: StatCardProps) {
  return (
    <div className="flex items-center gap-3 p-3 rounded-lg bg-muted/30">
      <div className="p-2 rounded-md bg-background">
        {icon}
      </div>
      <div className="flex-1 min-w-0">
        <p className="text-xs text-muted-foreground truncate">{label}</p>
        <div className="flex items-center gap-2">
          <p className="text-lg font-semibold tabular-nums">{value}</p>
          {trend && trend !== 'neutral' && (
            <span className={cn(
              'text-xs',
              trend === 'up' ? 'text-red-500' : 'text-green-500'
            )}>
              {trend === 'up' ? <TrendingUp className="w-3 h-3" /> : <TrendingDown className="w-3 h-3" />}
            </span>
          )}
        </div>
        {subValue && (
          <p className="text-[10px] text-muted-foreground">{subValue}</p>
        )}
      </div>
    </div>
  )
}

export function QuotaStats({ data, windowType }: QuotaStatsProps) {
  const stats = useMemo(() => {
    if (data.length === 0) {
      return null
    }

    const values = data.map((d) => d.used_percent)
    const avg = values.reduce((a, b) => a + b, 0) / values.length
    const max = Math.max(...values)
    const min = Math.min(...values)
    const latest = values[values.length - 1]
    const first = values[0]

    // Calculate trend (comparing latest to first)
    const trendPercent = latest - first
    const trend: 'up' | 'down' | 'neutral' =
      trendPercent > 5 ? 'up' : trendPercent < -5 ? 'down' : 'neutral'

    // Format window type for display
    const windowLabel = windowType === '5_hour' ? '5小時' :
                        windowType === '7_day' ? '7天' :
                        windowType === '7_day_opus' ? 'Opus' :
                        windowType === '7_day_sonnet' ? 'Sonnet' : windowType

    return {
      avg,
      max,
      min,
      latest,
      trend,
      trendPercent,
      windowLabel,
      dataPoints: data.length,
    }
  }, [data, windowType])

  if (!stats) {
    return null
  }

  return (
    <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
      <StatCard
        label={`平均用量（${stats.windowLabel}）`}
        value={`${stats.avg.toFixed(1)}%`}
        icon={<BarChart3 className="w-4 h-4 text-blue-500" />}
      />
      <StatCard
        label={`最高用量（${stats.windowLabel}）`}
        value={`${stats.max.toFixed(1)}%`}
        icon={<TrendingUp className="w-4 h-4 text-orange-500" />}
      />
      <StatCard
        label={`最低用量（${stats.windowLabel}）`}
        value={`${stats.min.toFixed(1)}%`}
        icon={<TrendingDown className="w-4 h-4 text-green-500" />}
      />
      <StatCard
        label="目前用量"
        value={`${stats.latest.toFixed(1)}%`}
        subValue={`趨勢: ${stats.trendPercent > 0 ? '+' : ''}${stats.trendPercent.toFixed(1)}%`}
        icon={<Activity className="w-4 h-4 text-purple-500" />}
        trend={stats.trend}
      />
    </div>
  )
}

export default QuotaStats
