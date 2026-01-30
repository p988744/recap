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
    <header className="flex items-center gap-4">
      <div className="flex items-center gap-1">
        <p className="text-xs text-muted-foreground">
          {year} {formatDisplayDate(startDate)} - {formatDisplayDate(endDate)}
        </p>
      </div>
      <h1 className="font-display text-2xl text-foreground tracking-tight">
        第 {weekNumber} 週
      </h1>
      <div className="flex items-center gap-1">
        <Button variant="ghost" size="icon" className="h-7 w-7" onClick={onPrev}>
          <ChevronLeft className="w-4 h-4" strokeWidth={1.5} />
        </Button>
        <Button variant="ghost" size="icon" className="h-7 w-7" onClick={onNext}>
          <ChevronRight className="w-4 h-4" strokeWidth={1.5} />
        </Button>
      </div>
      {!isCurrentWeek && (
        <Button variant="outline" size="sm" className="text-xs h-7 px-2" onClick={onToday}>
          本週
        </Button>
      )}
    </header>
  )
}
