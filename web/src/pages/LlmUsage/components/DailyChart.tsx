import { BarChart3 } from 'lucide-react'
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from 'recharts'
import type { DailyUsage } from '@/types'

interface DailyChartProps {
  data: DailyUsage[]
}

function formatDate(date: string): string {
  return date.slice(5) // "MM-DD"
}

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`
  return `${n}`
}

export function DailyChart({ data }: DailyChartProps) {
  return (
    <div>
      <div className="flex items-center gap-2 mb-6">
        <BarChart3 className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
        <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
          每日 Token 用量
        </h2>
      </div>

      {data.length > 0 ? (
        <div className="h-64">
          <ResponsiveContainer width="100%" height="100%">
            <AreaChart data={data} margin={{ top: 5, right: 5, left: 0, bottom: 5 }}>
              <CartesianGrid strokeDasharray="3 3" stroke="hsl(30, 10%, 90%)" />
              <XAxis
                dataKey="date"
                tickFormatter={formatDate}
                tick={{ fontSize: 11, fill: 'hsl(30, 5%, 50%)' }}
                axisLine={{ stroke: 'hsl(30, 10%, 85%)' }}
              />
              <YAxis
                tickFormatter={formatTokens}
                tick={{ fontSize: 11, fill: 'hsl(30, 5%, 50%)' }}
                axisLine={{ stroke: 'hsl(30, 10%, 85%)' }}
                width={50}
              />
              <Tooltip
                content={({ payload, label }) => {
                  if (payload && payload.length > 0) {
                    return (
                      <div className="bg-popover/90 backdrop-blur px-3 py-2 border border-border text-sm">
                        <p className="text-foreground font-medium mb-1">{label}</p>
                        {payload.map((entry) => (
                          <p key={entry.name} className="text-muted-foreground">
                            {entry.name === 'prompt_tokens' ? '輸入' : '輸出'}:{' '}
                            {formatTokens(entry.value as number)} tokens
                          </p>
                        ))}
                      </div>
                    )
                  }
                  return null
                }}
              />
              <Legend
                formatter={(value) => (value === 'prompt_tokens' ? '輸入 Tokens' : '輸出 Tokens')}
                wrapperStyle={{ fontSize: 11 }}
              />
              <Area
                type="monotone"
                dataKey="prompt_tokens"
                stackId="1"
                stroke="hsl(35, 25%, 55%)"
                fill="hsl(35, 25%, 55%)"
                fillOpacity={0.4}
              />
              <Area
                type="monotone"
                dataKey="completion_tokens"
                stackId="1"
                stroke="hsl(15, 25%, 50%)"
                fill="hsl(15, 25%, 50%)"
                fillOpacity={0.4}
              />
            </AreaChart>
          </ResponsiveContainer>
        </div>
      ) : (
        <div className="h-64 flex items-center justify-center text-muted-foreground text-sm">
          暫無資料
        </div>
      )}
    </div>
  )
}
