import { useMemo } from 'react'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { cn } from '@/lib/utils'

interface DailyData {
  date: string
  hours: number
  count: number
}

interface ContributionHeatmapProps {
  data: DailyData[]
  weeks?: number
}

// Cell size: 10px for full year view (like GitHub)
const CELL_SIZE = 10
const CELL_GAP = 2
const CELL_TOTAL = CELL_SIZE + CELL_GAP

// Get intensity level based on hours worked (0-4)
function getIntensityLevel(hours: number): number {
  if (hours === 0) return 0
  if (hours < 2) return 1
  if (hours < 4) return 2
  if (hours < 6) return 3
  return 4
}

// Generate all dates for the past N weeks
function generateDateRange(weeks: number): string[] {
  const dates: string[] = []
  const today = new Date()
  const totalDays = weeks * 7

  for (let i = totalDays - 1; i >= 0; i--) {
    const date = new Date(today)
    date.setDate(today.getDate() - i)
    dates.push(date.toISOString().split('T')[0])
  }

  return dates
}

// Get day of week (0 = Sunday, 6 = Saturday)
function getDayOfWeek(dateStr: string): number {
  return new Date(dateStr).getDay()
}

// Format date for tooltip
function formatDate(dateStr: string): string {
  const date = new Date(dateStr)
  return date.toLocaleDateString('zh-TW', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    weekday: 'short'
  })
}

// Get month labels with positions
function getMonthLabels(dates: string[]): { month: string; weekIndex: number }[] {
  const labels: { month: string; weekIndex: number }[] = []
  let lastMonth = ''
  let currentWeekIndex = 0

  dates.forEach((date, index) => {
    const dayOfWeek = getDayOfWeek(date)

    // Track week index
    if (dayOfWeek === 0 && index > 0) {
      currentWeekIndex++
    }

    const month = new Date(date).toLocaleDateString('zh-TW', { month: 'short' })
    if (month !== lastMonth) {
      labels.push({ month, weekIndex: currentWeekIndex })
      lastMonth = month
    }
  })

  return labels
}

export function ContributionHeatmap({ data, weeks = 53 }: ContributionHeatmapProps) {
  // Build a map of date -> data for quick lookup
  const dataMap = useMemo(() => {
    const map = new Map<string, DailyData>()
    data.forEach(d => map.set(d.date, d))
    return map
  }, [data])

  // Generate all dates
  const allDates = useMemo(() => generateDateRange(weeks), [weeks])

  // Month labels
  const monthLabels = useMemo(() => getMonthLabels(allDates), [allDates])

  // Group dates by week (column)
  const weekColumns = useMemo(() => {
    const columns: string[][] = []
    let currentWeek: string[] = []

    // Find the first Sunday or start padding
    const firstDate = allDates[0]
    const firstDayOfWeek = getDayOfWeek(firstDate)

    // Pad the first week if it doesn't start on Sunday
    for (let i = 0; i < firstDayOfWeek; i++) {
      currentWeek.push('')
    }

    allDates.forEach((date) => {
      const dayOfWeek = getDayOfWeek(date)

      if (dayOfWeek === 0 && currentWeek.length > 0) {
        columns.push(currentWeek)
        currentWeek = []
      }

      currentWeek.push(date)
    })

    // Push the last week
    if (currentWeek.length > 0) {
      columns.push(currentWeek)
    }

    return columns
  }, [allDates])

  // Calculate total hours
  const totalHours = useMemo(() => {
    return data.reduce((sum, d) => sum + d.hours, 0)
  }, [data])

  // Calculate active days
  const activeDays = useMemo(() => {
    return data.filter(d => d.hours > 0).length
  }, [data])

  const dayLabels = ['日', '一', '二', '三', '四', '五', '六']

  return (
    <div className="space-y-3">
      {/* Month labels */}
      <div className="flex text-[10px] text-muted-foreground relative" style={{ marginLeft: 24 }}>
        {monthLabels.map((label, idx) => {
          const nextLabel = monthLabels[idx + 1]
          const width = nextLabel
            ? (nextLabel.weekIndex - label.weekIndex) * CELL_TOTAL
            : undefined

          return (
            <div
              key={`month-${idx}`}
              className="flex-shrink-0 truncate"
              style={{
                position: idx === 0 ? 'relative' : undefined,
                left: idx === 0 ? label.weekIndex * CELL_TOTAL : undefined,
                width: width,
                minWidth: width ? undefined : 30,
              }}
            >
              {label.month}
            </div>
          )
        })}
      </div>

      {/* Heatmap grid */}
      <div className="flex overflow-x-auto pb-2">
        {/* Day labels */}
        <div className="flex flex-col flex-shrink-0 mr-1 text-[9px] text-muted-foreground">
          {dayLabels.map((label, idx) => (
            <div
              key={`day-${idx}`}
              className="flex items-center justify-end pr-1"
              style={{ height: CELL_SIZE, marginBottom: idx < 6 ? CELL_GAP : 0 }}
            >
              {idx % 2 === 1 ? label : ''}
            </div>
          ))}
        </div>

        {/* Week columns */}
        <TooltipProvider delayDuration={100}>
          <div className="flex" style={{ gap: CELL_GAP }}>
            {weekColumns.map((week, weekIdx) => (
              <div key={`week-${weekIdx}`} className="flex flex-col" style={{ gap: CELL_GAP }}>
                {week.map((date, dayIdx) => {
                  if (!date) {
                    return (
                      <div
                        key={`empty-${dayIdx}`}
                        style={{ width: CELL_SIZE, height: CELL_SIZE }}
                      />
                    )
                  }

                  const dayData = dataMap.get(date)
                  const hours = dayData?.hours ?? 0
                  const count = dayData?.count ?? 0
                  const level = getIntensityLevel(hours)

                  return (
                    <Tooltip key={date}>
                      <TooltipTrigger asChild>
                        <div
                          className={cn(
                            'rounded-sm cursor-pointer transition-colors',
                            level === 0 && 'bg-muted/40 hover:bg-muted/60',
                            level === 1 && 'bg-warm/30 hover:bg-warm/40',
                            level === 2 && 'bg-warm/50 hover:bg-warm/60',
                            level === 3 && 'bg-warm/70 hover:bg-warm/80',
                            level === 4 && 'bg-warm hover:bg-warm/90'
                          )}
                          style={{ width: CELL_SIZE, height: CELL_SIZE }}
                        />
                      </TooltipTrigger>
                      <TooltipContent
                        side="top"
                        className="bg-popover border border-border px-3 py-2 shadow-lg"
                      >
                        <p className="text-sm font-medium text-foreground">{formatDate(date)}</p>
                        {hours > 0 ? (
                          <p className="text-xs text-foreground/80">
                            {hours.toFixed(1)} 小時 / {count} 個工作項目
                          </p>
                        ) : (
                          <p className="text-xs text-foreground/60">無工作紀錄</p>
                        )}
                      </TooltipContent>
                    </Tooltip>
                  )
                })}
              </div>
            ))}
          </div>
        </TooltipProvider>
      </div>

      {/* Legend and stats */}
      <div className="flex items-center justify-between text-[10px] text-muted-foreground">
        <div className="flex items-center gap-4">
          <span>過去一年共 {totalHours.toFixed(1)} 小時</span>
          <span>{activeDays} 個工作日</span>
        </div>
        <div className="flex items-center gap-1">
          <span className="mr-1">少</span>
          <div className="rounded-sm bg-muted/40" style={{ width: CELL_SIZE, height: CELL_SIZE }} />
          <div className="rounded-sm bg-warm/30" style={{ width: CELL_SIZE, height: CELL_SIZE }} />
          <div className="rounded-sm bg-warm/50" style={{ width: CELL_SIZE, height: CELL_SIZE }} />
          <div className="rounded-sm bg-warm/70" style={{ width: CELL_SIZE, height: CELL_SIZE }} />
          <div className="rounded-sm bg-warm" style={{ width: CELL_SIZE, height: CELL_SIZE }} />
          <span className="ml-1">多</span>
        </div>
      </div>
    </div>
  )
}
