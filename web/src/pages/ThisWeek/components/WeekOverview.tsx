import { useMemo } from 'react'
import { CalendarDays } from 'lucide-react'
import { Card, CardContent } from '@/components/ui/card'
import type { WorklogDay } from '@/types/worklog'
import { WeekTimeline } from './WeekTimeline'

interface WeekOverviewProps {
  projectCount: number
  daysWorked: number
  days: WorklogDay[]
  startDate: string
  endDate: string
}

export function WeekOverview({
  projectCount,
  daysWorked,
  days,
  startDate,
  endDate,
}: WeekOverviewProps) {
  // Calculate total hours from days (projects + manual items) to match timeline
  const totalHours = useMemo(() => {
    const projectHours = days.reduce(
      (sum, d) => sum + d.projects.reduce((s, p) => s + p.total_hours, 0),
      0
    )
    const manualHours = days.reduce(
      (sum, d) => sum + d.manual_items.reduce((s, m) => s + m.hours, 0),
      0
    )
    return projectHours + manualHours
  }, [days])

  return (
    <section className="space-y-8 animate-fade-up opacity-0 delay-2">
      {/* Stats Grid */}
      <div className="grid grid-cols-3 gap-6">
        {/* Weekly Hours - Featured stat */}
        <Card className="col-span-2 border-l-2 border-l-warm/60">
          <CardContent className="p-6">
            <div>
              <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
                本週工時
              </p>
              <p className="font-display text-5xl text-foreground tracking-tight">
                {totalHours.toFixed(1)}
              </p>
              <p className="text-sm text-muted-foreground mt-1">小時</p>
            </div>
          </CardContent>
        </Card>

        {/* Secondary stats */}
        <div className="space-y-4">
          <Card className="border-l-2 border-l-sage/40">
            <CardContent className="p-4">
              <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-1">
                專案數
              </p>
              <p className="font-display text-2xl text-foreground">{projectCount}</p>
            </CardContent>
          </Card>
          <Card className="border-l-2 border-l-stone/40">
            <CardContent className="p-4">
              <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-1">
                工作天數
              </p>
              <p className="font-display text-2xl text-foreground">{daysWorked}</p>
            </CardContent>
          </Card>
        </div>
      </div>

      {/* Week Timeline */}
      <div>
        <div className="flex items-center gap-2 mb-4">
          <CalendarDays className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
          <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
            本週工作時間軸
          </h2>
        </div>
        <Card>
          <CardContent className="p-6">
            <WeekTimeline
              days={days}
              startDate={startDate}
              endDate={endDate}
            />
          </CardContent>
        </Card>
      </div>
    </section>
  )
}
