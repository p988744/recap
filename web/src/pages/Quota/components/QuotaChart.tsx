/**
 * QuotaChart component
 *
 * Line chart visualization for quota history using Recharts.
 * Includes reference lines for warning and critical thresholds.
 */

import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  ReferenceLine,
} from 'recharts'
import type { QuotaSnapshot, QuotaSettings } from '@/types/quota'

interface QuotaChartProps {
  data: QuotaSnapshot[]
  settings: QuotaSettings
}

export function QuotaChart({ data, settings }: QuotaChartProps) {
  // Transform data for chart
  const chartData = data.map((snapshot) => ({
    time: new Date(snapshot.fetched_at).toLocaleString('zh-TW', {
      month: 'numeric',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    }),
    value: snapshot.used_percent,
    fullTime: new Date(snapshot.fetched_at).toLocaleString('zh-TW'),
  }))

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
          formatter={(value: number) => [`${value.toFixed(1)}%`, 'Usage']}
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
        {/* Usage line */}
        <Line
          type="monotone"
          dataKey="value"
          stroke="#3b82f6"
          strokeWidth={2}
          dot={false}
          activeDot={{ r: 4, fill: '#3b82f6' }}
        />
      </LineChart>
    </ResponsiveContainer>
  )
}
