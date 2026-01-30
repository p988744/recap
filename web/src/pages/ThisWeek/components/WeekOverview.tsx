import { CalendarDays } from 'lucide-react'
import { Card, CardContent } from '@/components/ui/card'
import type { WorklogDay } from '@/types/worklog'
import { WeekTimeline } from './WeekTimeline'

interface WeekOverviewProps {
  days: WorklogDay[]
  startDate: string
  endDate: string
}

export function WeekOverview({
  days,
  startDate,
  endDate,
}: WeekOverviewProps) {
  return (
    <section className="animate-fade-up opacity-0 delay-2">
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
