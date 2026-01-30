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

function formatFullDate(dateStr: string): string {
  const d = new Date(dateStr + 'T00:00:00')
  return `${d.getFullYear()}/${d.getMonth() + 1}/${d.getDate()}`
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
  return (
    <header className="flex items-center gap-6">
      <div>
        <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-1">
          {formatFullDate(startDate)} - {formatFullDate(endDate)}
        </p>
        <h1 className="font-display text-4xl text-foreground tracking-tight">
          第 {weekNumber} 週
        </h1>
      </div>
      <div className="flex items-center gap-1 self-end mb-1">
        <Button variant="ghost" size="icon" className="h-8 w-8" onClick={onPrev}>
          <ChevronLeft className="w-4 h-4" strokeWidth={1.5} />
        </Button>
        <Button variant="ghost" size="icon" className="h-8 w-8" onClick={onNext}>
          <ChevronRight className="w-4 h-4" strokeWidth={1.5} />
        </Button>
        {!isCurrentWeek && (
          <Button variant="outline" size="sm" className="text-xs h-8 ml-1" onClick={onToday}>
            本週
          </Button>
        )}
      </div>
    </header>
  )
}
