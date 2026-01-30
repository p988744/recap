import { useNavigate } from 'react-router-dom'
import { Calendar, Clock, FolderKanban, ChevronRight } from 'lucide-react'
import { Card, CardContent } from '@/components/ui/card'
import type { WorklogDay } from '@/types/worklog'

interface WeekTimelineSectionProps {
  days: WorklogDay[]
  today: string
}

function formatDayDisplay(dateStr: string): string {
  const d = new Date(dateStr + 'T00:00:00')
  return `${d.getMonth() + 1}/${d.getDate()}`
}

export function WeekTimelineSection({ days, today }: WeekTimelineSectionProps) {
  const navigate = useNavigate()

  // Filter out today (it's shown in TodayWorkSection)
  const otherDays = days.filter(d => d.date !== today)

  if (otherDays.length === 0) {
    return null
  }

  return (
    <section className="space-y-4 animate-fade-up opacity-0 delay-4">
      <div className="flex items-center gap-2">
        <Calendar className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
        <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
          本週時間軸
        </h2>
      </div>

      <div className="space-y-2">
        {otherDays.map((day) => (
          <DaySummaryCard
            key={day.date}
            day={day}
            onClick={() => navigate(`/day/${day.date}`)}
          />
        ))}
      </div>
    </section>
  )
}

interface DaySummaryCardProps {
  day: WorklogDay
  onClick: () => void
}

function DaySummaryCard({ day, onClick }: DaySummaryCardProps) {
  const isEmpty = day.projects.length === 0 && day.manual_items.length === 0

  // Calculate stats
  const totalHours = day.projects.reduce((sum, p) => sum + p.total_hours, 0) +
    day.manual_items.reduce((sum, m) => sum + m.hours, 0)
  const projectCount = day.projects.length + day.manual_items.length

  // Create summary text from project summaries
  const summaryText = day.projects
    .filter(p => p.daily_summary)
    .map(p => {
      const firstLine = p.daily_summary!.split('\n')[0]
      return firstLine.length > 80 ? firstLine.slice(0, 80) + '...' : firstLine
    })
    .slice(0, 2)
    .join(' • ')

  // Project/item names
  const projectNames = day.projects.map(p => p.project_name)
  const manualItemNames = day.manual_items.map(m => m.title)
  const allNames = [...projectNames, ...manualItemNames]

  return (
    <Card
      className="group cursor-pointer hover:bg-muted/30 transition-colors"
      onClick={onClick}
    >
      <CardContent className="px-5 py-4">
        <div className="flex items-center justify-between">
          {/* Left: Day info and summary */}
          <div className="flex-1 min-w-0">
            {/* Header row */}
            <div className="flex items-center gap-3 mb-2">
              <div className="flex items-baseline gap-2">
                <span className="font-medium text-foreground">
                  {day.weekday}
                </span>
                <span className="text-sm text-muted-foreground">
                  {formatDayDisplay(day.date)}
                </span>
              </div>

              {!isEmpty && (
                <div className="flex items-center gap-3 text-xs text-muted-foreground">
                  <span className="flex items-center gap-1">
                    <Clock className="w-3 h-3" strokeWidth={1.5} />
                    {totalHours.toFixed(1)}h
                  </span>
                  <span className="flex items-center gap-1">
                    <FolderKanban className="w-3 h-3" strokeWidth={1.5} />
                    {projectCount}
                  </span>
                </div>
              )}
            </div>

            {/* Project tags */}
            {allNames.length > 0 && (
              <div className="flex flex-wrap gap-1.5 mb-2">
                {allNames.slice(0, 4).map((name, i) => (
                  <span
                    key={i}
                    className="text-xs px-2 py-0.5 bg-muted/50 text-muted-foreground rounded"
                  >
                    {name}
                  </span>
                ))}
                {allNames.length > 4 && (
                  <span className="text-xs text-muted-foreground">
                    +{allNames.length - 4}
                  </span>
                )}
              </div>
            )}

            {/* Summary text */}
            {summaryText && (
              <p className="text-sm text-muted-foreground truncate">
                {summaryText}
              </p>
            )}

            {isEmpty && (
              <p className="text-sm text-muted-foreground/60">無工作紀錄</p>
            )}
          </div>

          {/* Right: Arrow */}
          <ChevronRight className="w-5 h-5 text-muted-foreground/40 group-hover:text-muted-foreground transition-colors shrink-0 ml-4" strokeWidth={1.5} />
        </div>
      </CardContent>
    </Card>
  )
}
