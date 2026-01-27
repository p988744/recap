import { Cpu } from 'lucide-react'
import { PieChart, Pie, Cell, ResponsiveContainer, Tooltip } from 'recharts'
import type { ModelUsage } from '@/types'

const CHART_COLORS = [
  'hsl(35, 25%, 55%)',
  'hsl(15, 25%, 50%)',
  'hsl(90, 15%, 50%)',
  'hsl(30, 15%, 55%)',
  'hsl(45, 20%, 60%)',
  'hsl(15, 20%, 60%)',
]

interface ModelBreakdownProps {
  data: ModelUsage[]
}

function formatCost(n: number): string {
  if (n < 0.01) return `$${n.toFixed(4)}`
  return `$${n.toFixed(2)}`
}

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`
  return `${n}`
}

export function ModelBreakdown({ data }: ModelBreakdownProps) {
  const chartData = data.map((m) => ({
    name: m.model,
    value: m.calls,
    cost: formatCost(m.cost),
    tokens: formatTokens(m.total_tokens),
    provider: m.provider,
  }))

  return (
    <div>
      <div className="flex items-center gap-2 mb-6">
        <Cpu className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
        <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
          模型分佈
        </h2>
      </div>

      {chartData.length > 0 ? (
        <div className="flex gap-6">
          <div className="h-44 w-44 flex-shrink-0">
            <ResponsiveContainer width="100%" height="100%">
              <PieChart>
                <Pie
                  data={chartData}
                  cx="50%"
                  cy="50%"
                  innerRadius={35}
                  outerRadius={60}
                  paddingAngle={2}
                  dataKey="value"
                  stroke="none"
                >
                  {chartData.map((_, index) => (
                    <Cell
                      key={`cell-${index}`}
                      fill={CHART_COLORS[index % CHART_COLORS.length]}
                    />
                  ))}
                </Pie>
                <Tooltip
                  content={({ payload }) => {
                    if (payload && payload[0]) {
                      const d = payload[0].payload
                      return (
                        <div className="bg-popover/90 backdrop-blur px-3 py-2 border border-border text-sm">
                          <p className="text-foreground">{d.name}</p>
                          <p className="text-muted-foreground">{d.value} 次呼叫</p>
                          <p className="text-muted-foreground">{d.tokens} tokens</p>
                          <p className="text-muted-foreground">{d.cost}</p>
                        </div>
                      )
                    }
                    return null
                  }}
                />
              </PieChart>
            </ResponsiveContainer>
          </div>
          <div className="flex-1 space-y-3">
            {chartData.map((item, index) => (
              <div key={item.name} className="flex items-center gap-3">
                <div
                  className="w-2 h-2 flex-shrink-0"
                  style={{ backgroundColor: CHART_COLORS[index % CHART_COLORS.length] }}
                />
                <div className="flex-1 min-w-0">
                  <span className="text-sm text-foreground truncate block">{item.name}</span>
                  <span className="text-[10px] text-muted-foreground">{item.provider}</span>
                </div>
                <div className="text-right flex-shrink-0">
                  <span className="text-sm text-foreground tabular-nums">{item.value}x</span>
                  <span className="text-[10px] text-muted-foreground block">{item.cost}</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      ) : (
        <div className="h-44 flex items-center justify-center text-muted-foreground text-sm">
          暫無資料
        </div>
      )}
    </div>
  )
}
