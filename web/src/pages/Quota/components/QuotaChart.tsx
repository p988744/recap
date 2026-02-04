/**
 * QuotaChart component
 *
 * Line chart visualization for quota history using Recharts.
 * Shows multiple lines for different window types (5-hour and 7-day).
 * Includes reference lines for warning and critical thresholds.
 */

import { useMemo } from 'react'
import {
  ComposedChart,
  Bar,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  ReferenceLine,
  Legend,
} from 'recharts'
import type { QuotaSnapshot, QuotaSettings } from '@/types/quota'

interface QuotaChartProps {
  data: QuotaSnapshot[]
  settings: QuotaSettings
  /** Current quota snapshots to calculate window boundaries */
  currentQuota?: QuotaSnapshot[]
}

interface ChartDataPoint {
  time: string
  fullTime: string
  timestamp: number
  fiveHour?: number
  sevenDay?: number
}

// Window durations in milliseconds
const WINDOW_DURATIONS: Record<string, number> = {
  '5_hour': 5 * 60 * 60 * 1000,
  '7_day': 7 * 24 * 60 * 60 * 1000,
}

export function QuotaChart({ data, settings, currentQuota }: QuotaChartProps) {
  // Transform data for chart - group by timestamp and separate by window type
  // Insert gap markers where time difference is too large
  const chartData = useMemo(() => {
    // Group data by timestamp (rounded to nearest minute)
    const dataMap = new Map<string, ChartDataPoint>()

    data.forEach((snapshot) => {
      const date = new Date(snapshot.fetched_at)
      const timestamp = Math.floor(date.getTime() / 60000) * 60000 // Round to minute
      const key = timestamp.toString()

      if (!dataMap.has(key)) {
        dataMap.set(key, {
          time: date.toLocaleString('zh-TW', {
            month: 'numeric',
            day: 'numeric',
            hour: '2-digit',
          }),
          fullTime: date.toLocaleString('zh-TW'),
          timestamp,
        })
      }

      const point = dataMap.get(key)!
      if (snapshot.window_type === '5_hour') {
        point.fiveHour = snapshot.used_percent
      } else if (snapshot.window_type === '7_day') {
        point.sevenDay = snapshot.used_percent
      }
    })

    // Sort by timestamp
    const sorted = Array.from(dataMap.values()).sort((a, b) => a.timestamp - b.timestamp)

    // Insert null points where gap is larger than 1 hour to break the line
    const maxGap = 60 * 60 * 1000 // 1 hour
    const result: ChartDataPoint[] = []

    for (let i = 0; i < sorted.length; i++) {
      if (i > 0) {
        const gap = sorted[i].timestamp - sorted[i - 1].timestamp
        if (gap > maxGap) {
          // Insert a gap marker with null values
          result.push({
            time: '',
            fullTime: '',
            timestamp: sorted[i - 1].timestamp + 1,
            fiveHour: undefined,
            sevenDay: undefined,
          })
        }
      }
      result.push(sorted[i])
    }

    return result
  }, [data])

  // Extract window boundaries from history data's resets_at
  // Each distinct resets_at + 1 second = start of a new window
  const windowBoundaries = useMemo(() => {
    if (chartData.length === 0) return []

    // Get distinct resets_at values from 5-hour quota history
    const resetsAtSet = new Set<string>()
    data.forEach((snapshot) => {
      if (snapshot.window_type === '5_hour' && snapshot.resets_at) {
        resetsAtSet.add(snapshot.resets_at)
      }
    })

    const chartStart = chartData[0].timestamp
    const chartEnd = chartData[chartData.length - 1].timestamp

    const boundaries: { xValue: string; label: string }[] = []

    resetsAtSet.forEach((resetsAtStr) => {
      // resets_at + 1 second = window start time
      const resetsAt = new Date(resetsAtStr).getTime()
      const windowStart = resetsAt + 1000 // Add 1 second

      // Only include if within chart range
      if (windowStart < chartStart || windowStart > chartEnd) return

      // Find closest data point
      let closestPoint = chartData[0]
      let minDiff = Math.abs(chartData[0].timestamp - windowStart)

      for (const point of chartData) {
        if (!point.time) continue // Skip gap markers
        const diff = Math.abs(point.timestamp - windowStart)
        if (diff < minDiff) {
          minDiff = diff
          closestPoint = point
        }
      }

      // Only show if within 1 hour of a data point
      const oneHour = 60 * 60 * 1000
      if (minDiff > oneHour) return

      const date = new Date(windowStart)
      boundaries.push({
        xValue: closestPoint.time,
        label: date.toLocaleString('zh-TW', {
          hour: '2-digit',
          minute: '2-digit',
        }),
      })
    })

    // Remove duplicates and sort by label
    const unique = boundaries.filter(
      (b, i, arr) => arr.findIndex((x) => x.xValue === b.xValue) === i
    )

    return unique
  }, [data, chartData])

  return (
    <ResponsiveContainer width="100%" height={300}>
      <ComposedChart
        data={chartData}
        margin={{ top: 10, right: 30, left: 0, bottom: 0 }}
      >
        <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
        <XAxis
          dataKey="time"
          fontSize={11}
          tick={{ fill: 'hsl(var(--muted-foreground))' }}
          tickLine={{ stroke: 'hsl(var(--border))' }}
          interval="preserveStartEnd"
          minTickGap={60}
        />
        <YAxis
          domain={[0, 100]}
          fontSize={12}
          tick={{ fill: 'hsl(var(--muted-foreground))' }}
          tickLine={{ stroke: 'hsl(var(--border))' }}
          tickFormatter={(v) => `${v}%`}
        />
        <Tooltip
          formatter={(value: number, name: string) => {
            const label = name === 'fiveHour' ? '5 小時' : '本週用量'
            return [`${value.toFixed(1)}%`, label]
          }}
          labelFormatter={(_, payload) => {
            if (payload && payload[0]) {
              return payload[0].payload.fullTime
            }
            return ''
          }}
          contentStyle={{
            backgroundColor: 'hsl(var(--popover))',
            border: '1px solid hsl(var(--border))',
            borderRadius: '6px',
          }}
          labelStyle={{ color: 'hsl(var(--popover-foreground))' }}
        />
        <Legend
          formatter={(value: string) => (value === 'fiveHour' ? '5 小時' : '本週用量')}
          wrapperStyle={{ fontSize: 12 }}
        />
        {/* Warning threshold line */}
        <ReferenceLine
          y={settings.warning_threshold}
          stroke="#eab308"
          strokeDasharray="5 5"
          label={{
            value: `Warning (${settings.warning_threshold}%)`,
            position: 'insideTopRight',
            fill: '#eab308',
            fontSize: 11,
          }}
        />
        {/* Critical threshold line */}
        <ReferenceLine
          y={settings.critical_threshold}
          stroke="#ef4444"
          strokeDasharray="5 5"
          label={{
            value: `Critical (${settings.critical_threshold}%)`,
            position: 'insideTopRight',
            fill: '#ef4444',
            fontSize: 11,
          }}
        />
        {/* Window boundary lines from history resets_at */}
        {windowBoundaries.map((boundary, index) => (
          <ReferenceLine
            key={`boundary-${index}`}
            x={boundary.xValue}
            stroke="#64748b"
            strokeWidth={1}
            label={{
              value: boundary.label,
              position: 'insideTopLeft',
              fill: '#64748b',
              fontSize: 10,
            }}
          />
        ))}
        {/* 5-hour usage bar */}
        <Bar
          dataKey="fiveHour"
          name="fiveHour"
          fill="#3b82f6"
          radius={[2, 2, 0, 0]}
          barSize={8}
        />
        {/* 7-day usage line - dashed line to connect gaps (hidden from tooltip) */}
        <Line
          type="monotone"
          dataKey="sevenDay"
          stroke="#22c55e"
          strokeWidth={1.5}
          strokeDasharray="4 4"
          dot={false}
          activeDot={false}
          connectNulls
          legendType="none"
          tooltipType="none"
        />
        {/* 7-day usage line - solid line for actual data */}
        <Line
          type="monotone"
          dataKey="sevenDay"
          name="sevenDay"
          stroke="#22c55e"
          strokeWidth={2}
          dot={false}
          activeDot={{ r: 4, fill: '#22c55e' }}
        />
      </ComposedChart>
    </ResponsiveContainer>
  )
}
