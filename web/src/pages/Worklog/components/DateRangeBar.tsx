import { ChevronLeft, ChevronRight } from 'lucide-react'
import { Button } from '@/components/ui/button'

interface DateRangeBarProps {
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

export function DateRangeBar({
  startDate,
  endDate,
  isCurrentWeek,
  onPrev,
  onNext,
  onToday,
}: DateRangeBarProps) {
  const startYear = startDate.slice(0, 4)

  return (
    <div className="flex items-center gap-3">
      <div className="flex items-center gap-1">
        <Button variant="ghost" size="icon" className="h-8 w-8" onClick={onPrev}>
          <ChevronLeft className="w-4 h-4" strokeWidth={1.5} />
        </Button>
        <span className="text-sm text-foreground min-w-[140px] text-center font-medium">
          {startYear} {formatDisplayDate(startDate)} — {formatDisplayDate(endDate)}
        </span>
        <Button variant="ghost" size="icon" className="h-8 w-8" onClick={onNext}>
          <ChevronRight className="w-4 h-4" strokeWidth={1.5} />
        </Button>
      </div>
      {!isCurrentWeek && (
        <Button variant="outline" size="sm" className="text-xs h-7" onClick={onToday}>
          本週
        </Button>
      )}
    </div>
  )
}
