import { TrendingUp } from 'lucide-react'
import { PieChart, Pie, Cell, ResponsiveContainer, Tooltip } from 'recharts'

// Muted warm chart colors
const CHART_COLORS = [
  'hsl(35, 25%, 55%)',   // warm
  'hsl(15, 25%, 50%)',   // terracotta muted
  'hsl(90, 15%, 50%)',   // sage
  'hsl(30, 15%, 55%)',   // stone
  'hsl(45, 20%, 60%)',   // warm light
  'hsl(15, 20%, 60%)',   // terracotta light
]

interface ChartDataItem {
  name: string
  value: number
  hours: string
}

interface ProjectDistributionProps {
  chartData: ChartDataItem[]
}

export function ProjectDistribution({ chartData }: ProjectDistributionProps) {
  return (
    <div className="col-span-2">
      <div className="flex items-center gap-2 mb-6">
        <TrendingUp className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
        <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
          專案分佈
        </h2>
      </div>

      {chartData.length > 0 ? (
        <>
          <div className="h-44">
            <ResponsiveContainer width="100%" height="100%">
              <PieChart>
                <Pie
                  data={chartData}
                  cx="50%"
                  cy="50%"
                  innerRadius={45}
                  outerRadius={70}
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
                      const data = payload[0].payload
                      return (
                        <div className="bg-popover/90 backdrop-blur px-3 py-2 border border-border text-sm">
                          <p className="text-foreground">{data.name}</p>
                          <p className="text-muted-foreground">{data.hours} 小時</p>
                        </div>
                      )
                    }
                    return null
                  }}
                />
              </PieChart>
            </ResponsiveContainer>
          </div>
          <div className="mt-6 space-y-3">
            {chartData.slice(0, 4).map((item, index) => (
              <div key={item.name} className="flex items-center gap-3">
                <div
                  className="w-2 h-2"
                  style={{ backgroundColor: CHART_COLORS[index % CHART_COLORS.length] }}
                />
                <span className="text-sm text-muted-foreground flex-1 truncate">{item.name}</span>
                <span className="text-sm text-foreground tabular-nums">{item.hours}h</span>
              </div>
            ))}
          </div>
        </>
      ) : (
        <div className="h-44 flex items-center justify-center text-muted-foreground text-sm">
          暫無資料
        </div>
      )}
    </div>
  )
}
