/**
 * QuotaChart component
 *
 * Line chart visualization for quota history using Recharts.
 * Shows multiple lines for different window types (5-hour and 7-day).
 * Includes reference lines for warning and critical thresholds.
 */

import { useMemo } from 'react'
import {
  LineChart,
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
}

interface ChartDataPoint {
  time: string
  fullTime: string
  timestamp: number
  fiveHour?: number
  sevenDay?: number
}

export function QuotaChart({ data, settings }: QuotaChartProps) {
  // Transform data for chart - group by timestamp and separate by window type
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
            minute: '2-digit',
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

    // Sort by timestamp and return as array
    return Array.from(dataMap.values()).sort((a, b) => a.timestamp - b.timestamp)
  }, [data])

  return (
    <ResponsiveContainer width="100%" height={300}>
      <LineChart
        data={chartData}
        margin={{ top: 10, right: 30, left: 0, bottom: 0 }}
      >
        <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
        <XAxis
          dataKey="time"
          fontSize={12}
          tick={{ fill: 'hsl(var(--muted-foreground))' }}
          tickLine={{ stroke: 'hsl(var(--border))' }}
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
            const label = name === 'fiveHour' ? '5 小時' : '7 天'
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
          formatter={(value: string) => (value === 'fiveHour' ? '5 小時' : '7 天')}
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
        {/* 5-hour usage line */}
        <Line
          type="monotone"
          dataKey="fiveHour"
          name="fiveHour"
          stroke="#3b82f6"
          strokeWidth={2}
          dot={false}
          activeDot={{ r: 4, fill: '#3b82f6' }}
          connectNulls
        />
        {/* 7-day usage line */}
        <Line
          type="monotone"
          dataKey="sevenDay"
          name="sevenDay"
          stroke="#22c55e"
          strokeWidth={2}
          dot={false}
          activeDot={{ r: 4, fill: '#22c55e' }}
          connectNulls
        />
      </LineChart>
    </ResponsiveContainer>
  )
}
