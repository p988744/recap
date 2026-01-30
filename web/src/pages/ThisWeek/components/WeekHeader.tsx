import { ChevronLeft, ChevronRight } from 'lucide-react'
import { Button } from '@/components/ui/button'

interface WeekHeaderProps {
  weekNumber: number
  startDate: string
  endDate: string
  isCurrentWeek: boolean
  onPrev: () => void
  onNext: () => void
  onToday: () => void
}

function formatDisplayDate(dateStr: string): string {
  const d = new Date(dateStr + 'T00:00:00')
  return `${d.getMonth() + 1}/${d.getDate()}`
}

export function WeekHeader({
  weekNumber,
  startDate,
  endDate,
  isCurrentWeek,
  onPrev,
  onNext,
  onToday,
}: WeekHeaderProps) {
  const year = startDate.slice(0, 4)

  return (
    <header className="animate-fade-up opacity-0 delay-1">
      <div className="flex items-start justify-between mb-4">
        <div>
          <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
            {year} {formatDisplayDate(startDate)} - {formatDisplayDate(endDate)}
          </p>
          <h1 className="font-display text-4xl text-foreground tracking-tight">
            第 {weekNumber} 週
          </h1>
        </div>
        <div className="flex items-center gap-2">
          <div className="flex items-center">
            <Button variant="ghost" size="icon" className="h-8 w-8" onClick={onPrev}>
              <ChevronLeft className="w-4 h-4" strokeWidth={1.5} />
            </Button>
            <Button variant="ghost" size="icon" className="h-8 w-8" onClick={onNext}>
              <ChevronRight className="w-4 h-4" strokeWidth={1.5} />
            </Button>
          </div>
          {!isCurrentWeek && (
            <Button variant="outline" size="sm" className="text-xs h-8" onClick={onToday}>
              本週
            </Button>
          )}
        </div>
      </div>
    </header>
  )
}
