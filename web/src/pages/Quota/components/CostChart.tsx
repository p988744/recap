/**
 * CostChart component
 *
 * Displays a bar chart of daily costs over the past 30 days.
 */

import { useMemo } from 'react'
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  TooltipProps,
} from 'recharts'
import { format, parseISO } from 'date-fns'
import { zhTW } from 'date-fns/locale'
import type { CostSummary } from '@/types/quota'
import { formatCost, formatTokens } from '@/types/quota'

interface CostChartProps {
  costSummary: CostSummary
}

// Custom tooltip component
function CustomTooltip({
  active,
  payload,
}: TooltipProps<number, string>) {
  if (!active || !payload?.length) return null

  const data = payload[0].payload as {
    date: string
    cost: number
    tokens: number
    displayDate: string
  }

  return (
    <div className="bg-popover border border-border rounded-lg shadow-lg p-3 text-sm">
      <p className="font-medium mb-1">{data.displayDate}</p>
      <div className="space-y-1 text-muted-foreground">
        <p>
          費用：<span className="text-foreground font-medium">{formatCost(data.cost)}</span>
        </p>
        <p>
          Tokens：<span className="text-foreground">{formatTokens(data.tokens)}</span>
        </p>
      </div>
    </div>
  )
}

export function CostChart({ costSummary }: CostChartProps) {
  // Transform data for the chart
  const chartData = useMemo(() => {
    return costSummary.daily_usage.map((day) => {
      const date = parseISO(day.date)
      return {
        date: day.date,
        cost: day.total_cost,
        tokens: day.total_tokens,
        displayDate: format(date, 'M月d日', { locale: zhTW }),
        shortDate: format(date, 'M/d'),
      }
    })
  }, [costSummary.daily_usage])

  // Calculate Y-axis domain
  const maxCost = Math.max(...chartData.map((d) => d.cost), 1)
  const yAxisMax = Math.ceil(maxCost * 1.1) // Add 10% padding

  if (chartData.length === 0) {
    return (
      <div className="flex items-center justify-center h-[200px] text-muted-foreground">
        <p>尚無費用資料</p>
      </div>
    )
  }

  return (
    <div className="h-[200px] w-full">
      <ResponsiveContainer width="100%" height="100%">
        <BarChart
          data={chartData}
          margin={{ top: 10, right: 10, left: 0, bottom: 0 }}
        >
          <CartesianGrid
            strokeDasharray="3 3"
            stroke="hsl(var(--border))"
            opacity={0.5}
          />
          <XAxis
            dataKey="shortDate"
            tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }}
            tickLine={false}
            axisLine={{ stroke: 'hsl(var(--border))' }}
            interval="preserveStartEnd"
          />
          <YAxis
            domain={[0, yAxisMax]}
            tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }}
            tickLine={false}
            axisLine={false}
            tickFormatter={(value) => `$${value}`}
            width={45}
          />
          <Tooltip content={<CustomTooltip />} />
          <Bar
            dataKey="cost"
            fill="hsl(var(--chart-1))"
            radius={[2, 2, 0, 0]}
            maxBarSize={20}
          />
        </BarChart>
      </ResponsiveContainer>
    </div>
  )
}

export default CostChart
