/**
 * QuotaStats component
 *
 * Displays usage statistics for quota data including
 * average and maximum usage for both 5-hour and 7-day periods.
 */

import { useMemo } from 'react'
import { Clock, Calendar, TrendingUp, BarChart3 } from 'lucide-react'
import type { QuotaSnapshot } from '@/types/quota'

interface QuotaStatsProps {
  /** Current snapshots for all window types */
  currentQuota: QuotaSnapshot[]
  /** History data for the selected window type */
  historyData: QuotaSnapshot[]
  /** Selected window type for history */
  windowType: string
}

function StatRow({
  icon,
  label,
  current,
  avg,
  max,
}: {
  icon: React.ReactNode
  label: string
  current: number | null
  avg: number | null
  max: number | null
}) {
  const formatValue = (v: number | null) => (v !== null ? `${v.toFixed(1)}%` : '-')

  return (
    <tr className="border-b border-border/50 last:border-0">
      <td className="py-2.5 pr-4">
        <div className="flex items-center gap-2">
          {icon}
          <span className="text-sm font-medium">{label}</span>
        </div>
      </td>
      <td className="py-2.5 px-4 text-right">
        <span className="text-sm tabular-nums font-semibold">{formatValue(current)}</span>
      </td>
      <td className="py-2.5 px-4 text-right">
        <span className="text-sm tabular-nums text-muted-foreground">{formatValue(avg)}</span>
      </td>
      <td className="py-2.5 pl-4 text-right">
        <span className="text-sm tabular-nums text-muted-foreground">{formatValue(max)}</span>
      </td>
    </tr>
  )
}

export function QuotaStats({ currentQuota, historyData, windowType }: QuotaStatsProps) {
  const stats = useMemo(() => {
    // Get current values from snapshots
    const getCurrent = (wt: string) => {
      const snapshot = currentQuota.find(
        (s) => s.provider === 'claude' && s.window_type === wt
      )
      return snapshot?.used_percent ?? null
    }

    // Calculate history stats for a specific window type
    const getHistoryStats = (data: QuotaSnapshot[], wt: string) => {
      const filtered = data.filter((d) => d.window_type === wt)
      if (filtered.length === 0) {
        return { avg: null, max: null }
      }
      const values = filtered.map((d) => d.used_percent)
      return {
        avg: values.reduce((a, b) => a + b, 0) / values.length,
        max: Math.max(...values),
      }
    }

    // 5-hour stats
    const fiveHourCurrent = getCurrent('5_hour')
    const fiveHourHistory = windowType === '5_hour'
      ? getHistoryStats(historyData, '5_hour')
      : { avg: null, max: null }

    // 7-day stats
    const sevenDayCurrent = getCurrent('7_day')
    const sevenDayHistory = windowType === '7_day'
      ? getHistoryStats(historyData, '7_day')
      : { avg: null, max: null }

    // 7-day Opus stats
    const opusCurrent = getCurrent('7_day_opus')
    const opusHistory = windowType === '7_day_opus'
      ? getHistoryStats(historyData, '7_day_opus')
      : { avg: null, max: null }

    // 7-day Sonnet stats
    const sonnetCurrent = getCurrent('7_day_sonnet')
    const sonnetHistory = windowType === '7_day_sonnet'
      ? getHistoryStats(historyData, '7_day_sonnet')
      : { avg: null, max: null }

    return {
      fiveHour: {
        current: fiveHourCurrent,
        avg: fiveHourHistory.avg,
        max: fiveHourHistory.max,
      },
      sevenDay: {
        current: sevenDayCurrent,
        avg: sevenDayHistory.avg,
        max: sevenDayHistory.max,
      },
      opus: {
        current: opusCurrent,
        avg: opusHistory.avg,
        max: opusHistory.max,
      },
      sonnet: {
        current: sonnetCurrent,
        avg: sonnetHistory.avg,
        max: sonnetHistory.max,
      },
    }
  }, [currentQuota, historyData, windowType])

  // Only show rows that have current data
  const hasOpus = stats.opus.current !== null
  const hasSonnet = stats.sonnet.current !== null

  return (
    <div className="overflow-x-auto">
      <table className="w-full">
        <thead>
          <tr className="text-xs text-muted-foreground border-b border-border">
            <th className="py-2 pr-4 text-left font-medium">週期</th>
            <th className="py-2 px-4 text-right font-medium">
              <div className="flex items-center justify-end gap-1">
                <BarChart3 className="w-3 h-3" />
                目前
              </div>
            </th>
            <th className="py-2 px-4 text-right font-medium">
              <div className="flex items-center justify-end gap-1">
                平均
              </div>
            </th>
            <th className="py-2 pl-4 text-right font-medium">
              <div className="flex items-center justify-end gap-1">
                <TrendingUp className="w-3 h-3" />
                最高
              </div>
            </th>
          </tr>
        </thead>
        <tbody>
          <StatRow
            icon={<Clock className="w-4 h-4 text-blue-500" />}
            label="5 小時"
            current={stats.fiveHour.current}
            avg={stats.fiveHour.avg}
            max={stats.fiveHour.max}
          />
          <StatRow
            icon={<Calendar className="w-4 h-4 text-green-500" />}
            label="7 天"
            current={stats.sevenDay.current}
            avg={stats.sevenDay.avg}
            max={stats.sevenDay.max}
          />
          {hasOpus && (
            <StatRow
              icon={<span className="w-4 h-4 text-purple-500 text-xs font-bold">O</span>}
              label="Opus (7天)"
              current={stats.opus.current}
              avg={stats.opus.avg}
              max={stats.opus.max}
            />
          )}
          {hasSonnet && (
            <StatRow
              icon={<span className="w-4 h-4 text-orange-500 text-xs font-bold">S</span>}
              label="Sonnet (7天)"
              current={stats.sonnet.current}
              avg={stats.sonnet.avg}
              max={stats.sonnet.max}
            />
          )}
        </tbody>
      </table>
      <p className="text-[10px] text-muted-foreground mt-2">
        * 平均和最高值基於所選時間範圍內的歷史資料（僅顯示當前選擇的週期類型）
      </p>
    </div>
  )
}

export default QuotaStats
