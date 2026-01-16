import { Calendar } from 'lucide-react'
import { Card, CardContent } from '@/components/ui/card'
import { WorkGanttChart } from '@/components/WorkGanttChart'
import type { TimelineSession } from '../hooks/types'

interface TimelineViewProps {
  sessions: TimelineSession[]
  date: string
  onDateChange: (date: string) => void
}

export function TimelineView({ sessions, date, onDateChange }: TimelineViewProps) {
  return (
    <>
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-2">
          <Calendar className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
          <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
            時間軸檢視
          </p>
        </div>
      </div>

      <Card>
        <CardContent className="p-6">
          <WorkGanttChart
            sessions={sessions}
            date={date}
            onDateChange={onDateChange}
          />
        </CardContent>
      </Card>
    </>
  )
}
