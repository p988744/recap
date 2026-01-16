import { Card, CardContent } from '@/components/ui/card'
import { Progress } from '@/components/ui/progress'
import type { WorkItemStatsResponse } from '@/types'
import { getWeekProgress } from '@/lib/utils'

interface WeeklyStatsProps {
  stats: WorkItemStatsResponse | null
  projectCount: number
  daysWorked: number
}

export function WeeklyStats({ stats, projectCount, daysWorked }: WeeklyStatsProps) {
  const weekProgress = getWeekProgress()

  return (
    <section className="grid grid-cols-3 gap-8 animate-fade-up opacity-0 delay-2">
      {/* Weekly Hours - Featured stat */}
      <Card className="col-span-2 border-l-2 border-l-warm/60">
        <CardContent className="p-8">
          <div className="flex items-end justify-between">
            <div>
              <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-3">
                本週工時
              </p>
              <p className="font-display text-6xl text-foreground tracking-tight">
                {(stats?.total_hours ?? 0).toFixed(1)}
              </p>
              <p className="text-sm text-muted-foreground mt-2">小時</p>
            </div>
            <div className="text-right">
              <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-1">
                目標進度
              </p>
              <p className="text-2xl text-muted-foreground">{weekProgress.toFixed(0)}%</p>
              <div className="w-32 mt-3">
                <Progress value={Math.min(weekProgress, 100)} className="h-1" />
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Secondary stats */}
      <div className="space-y-6">
        <Card className="border-l-2 border-l-sage/40">
          <CardContent className="p-6">
            <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
              專案數
            </p>
            <p className="font-display text-3xl text-foreground">{projectCount}</p>
          </CardContent>
        </Card>
        <Card className="border-l-2 border-l-stone/40">
          <CardContent className="p-6">
            <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
              工作天數
            </p>
            <p className="font-display text-3xl text-foreground">{daysWorked}</p>
          </CardContent>
        </Card>
      </div>
    </section>
  )
}
