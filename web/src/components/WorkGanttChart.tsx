import { useMemo, useState } from 'react'
import { cn } from '@/lib/utils'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { ChevronLeft, ChevronRight, GitCommit, Code2 } from 'lucide-react'
import { Button } from '@/components/ui/button'

export interface TimelineSession {
  id: string
  project: string
  title: string
  startTime: string  // ISO timestamp
  endTime: string    // ISO timestamp
  hours: number
  commits: TimelineCommit[]
}

export interface TimelineCommit {
  hash: string
  message: string
  time: string  // ISO timestamp
  author: string
}

interface WorkGanttChartProps {
  sessions: TimelineSession[]
  date: string  // YYYY-MM-DD
  onDateChange?: (date: string) => void
}

// Work hours range (8 AM to 8 PM)
const WORK_START_HOUR = 8
const WORK_END_HOUR = 22
const TOTAL_HOURS = WORK_END_HOUR - WORK_START_HOUR

// Color palette for projects
const PROJECT_COLORS = [
  'bg-warm/70',
  'bg-sage/70',
  'bg-terracotta/70',
  'bg-sky/70',
  'bg-amber-500/50',
  'bg-violet-500/50',
]

function getProjectColor(_projectName: string, projectIndex: number): string {
  return PROJECT_COLORS[projectIndex % PROJECT_COLORS.length]
}

function parseTime(timestamp: string): Date {
  return new Date(timestamp)
}

function formatTime(date: Date): string {
  return date.toLocaleTimeString('zh-TW', { hour: '2-digit', minute: '2-digit' })
}

function formatDate(dateStr: string): string {
  const date = new Date(dateStr)
  return date.toLocaleDateString('zh-TW', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    weekday: 'short'
  })
}

function getPositionPercent(time: Date): number {
  const hours = time.getHours() + time.getMinutes() / 60
  const position = ((hours - WORK_START_HOUR) / TOTAL_HOURS) * 100
  return Math.max(0, Math.min(100, position))
}

function getWidthPercent(startTime: Date, endTime: Date): number {
  const startHours = startTime.getHours() + startTime.getMinutes() / 60
  const endHours = endTime.getHours() + endTime.getMinutes() / 60
  const width = ((endHours - startHours) / TOTAL_HOURS) * 100
  return Math.max(1, Math.min(100, width))
}

export function WorkGanttChart({ sessions, date, onDateChange }: WorkGanttChartProps) {
  const [hoveredSession, setHoveredSession] = useState<string | null>(null)

  // Group sessions by project
  const projectGroups = useMemo(() => {
    const groups: Map<string, TimelineSession[]> = new Map()

    sessions.forEach(session => {
      const existing = groups.get(session.project) || []
      existing.push(session)
      groups.set(session.project, existing)
    })

    return Array.from(groups.entries()).map(([project, sessions]) => ({
      project,
      sessions: sessions.sort((a, b) =>
        parseTime(a.startTime).getTime() - parseTime(b.startTime).getTime()
      )
    }))
  }, [sessions])

  // Calculate total hours
  const totalHours = useMemo(() => {
    return sessions.reduce((sum, s) => sum + s.hours, 0)
  }, [sessions])

  // Total commits
  const totalCommits = useMemo(() => {
    return sessions.reduce((sum, s) => sum + s.commits.length, 0)
  }, [sessions])

  // Time markers
  const timeMarkers = useMemo(() => {
    const markers = []
    for (let h = WORK_START_HOUR; h <= WORK_END_HOUR; h += 2) {
      markers.push({
        hour: h,
        label: `${h}:00`,
        percent: ((h - WORK_START_HOUR) / TOTAL_HOURS) * 100
      })
    }
    return markers
  }, [])

  // Navigate dates
  const goToPrevDay = () => {
    const d = new Date(date)
    d.setDate(d.getDate() - 1)
    onDateChange?.(d.toISOString().split('T')[0])
  }

  const goToNextDay = () => {
    const d = new Date(date)
    d.setDate(d.getDate() + 1)
    onDateChange?.(d.toISOString().split('T')[0])
  }

  const goToToday = () => {
    onDateChange?.(new Date().toISOString().split('T')[0])
  }

  return (
    <div className="space-y-4">
      {/* Header with date navigation */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8"
            onClick={goToPrevDay}
          >
            <ChevronLeft className="h-4 w-4" />
          </Button>
          <span className="text-sm font-medium min-w-[140px] text-center">
            {formatDate(date)}
          </span>
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8"
            onClick={goToNextDay}
          >
            <ChevronRight className="h-4 w-4" />
          </Button>
          <Button
            variant="ghost"
            size="sm"
            className="text-xs"
            onClick={goToToday}
          >
            今天
          </Button>
        </div>
        <div className="flex items-center gap-4 text-xs text-muted-foreground">
          <span className="flex items-center gap-1">
            <Code2 className="h-3 w-3" />
            {sessions.length} sessions
          </span>
          <span className="flex items-center gap-1">
            <GitCommit className="h-3 w-3" />
            {totalCommits} commits
          </span>
          <span>{totalHours.toFixed(1)}h</span>
        </div>
      </div>

      {/* Timeline */}
      <div className="relative">
        {/* Time axis */}
        <div className="relative h-6 border-b border-border mb-2">
          {timeMarkers.map(marker => (
            <div
              key={marker.hour}
              className="absolute top-0 flex flex-col items-center"
              style={{ left: `${marker.percent}%` }}
            >
              <div className="h-2 w-px bg-border" />
              <span className="text-[10px] text-muted-foreground mt-1">
                {marker.label}
              </span>
            </div>
          ))}
        </div>

        {/* Project rows */}
        <TooltipProvider delayDuration={100}>
          <div className="space-y-3">
            {projectGroups.length === 0 ? (
              <div className="text-center text-muted-foreground text-sm py-8">
                當日無工作紀錄
              </div>
            ) : (
              projectGroups.map(({ project, sessions }, projectIndex) => (
                <div key={project} className="relative">
                  {/* Project label */}
                  <div className="flex items-center gap-2 mb-1">
                    <div
                      className={cn(
                        'w-2 h-2 rounded-full',
                        getProjectColor(project, projectIndex)
                      )}
                    />
                    <span className="text-xs text-muted-foreground truncate max-w-[120px]">
                      {project}
                    </span>
                  </div>

                  {/* Timeline row */}
                  <div className="relative h-8 bg-muted/20 rounded">
                    {/* Grid lines */}
                    {timeMarkers.map(marker => (
                      <div
                        key={marker.hour}
                        className="absolute top-0 bottom-0 w-px bg-border/30"
                        style={{ left: `${marker.percent}%` }}
                      />
                    ))}

                    {/* Session bars */}
                    {sessions.map((session) => {
                      const startTime = parseTime(session.startTime)
                      const endTime = parseTime(session.endTime)
                      const left = getPositionPercent(startTime)
                      const width = getWidthPercent(startTime, endTime)

                      return (
                        <Tooltip key={session.id}>
                          <TooltipTrigger asChild>
                            <div
                              className={cn(
                                'absolute top-1 bottom-1 rounded cursor-pointer transition-all',
                                getProjectColor(project, projectIndex),
                                hoveredSession === session.id && 'ring-2 ring-foreground/20'
                              )}
                              style={{
                                left: `${left}%`,
                                width: `${width}%`,
                                minWidth: '8px'
                              }}
                              onMouseEnter={() => setHoveredSession(session.id)}
                              onMouseLeave={() => setHoveredSession(null)}
                            >
                              {/* Commit markers */}
                              {session.commits.map((commit) => {
                                const commitTime = parseTime(commit.time)
                                const sessionStart = startTime.getTime()
                                const sessionEnd = endTime.getTime()
                                const commitPos = ((commitTime.getTime() - sessionStart) / (sessionEnd - sessionStart)) * 100

                                if (commitPos < 0 || commitPos > 100) return null

                                return (
                                  <div
                                    key={commit.hash}
                                    className="absolute top-1/2 -translate-y-1/2 w-1.5 h-1.5 rounded-full bg-foreground/80"
                                    style={{ left: `${commitPos}%` }}
                                    title={commit.message}
                                  />
                                )
                              })}
                            </div>
                          </TooltipTrigger>
                          <TooltipContent
                            side="top"
                            className="bg-popover border border-border px-3 py-2 max-w-xs shadow-lg"
                          >
                            <div className="space-y-2">
                              <p className="text-sm font-medium text-foreground line-clamp-2">{session.title}</p>
                              <div className="text-xs space-y-1">
                                <p className="text-foreground/80">
                                  {formatTime(startTime)} - {formatTime(endTime)}
                                  <span className="ml-2 font-medium">{session.hours.toFixed(1)}h</span>
                                </p>
                                {session.commits.length > 0 && (
                                  <div className="pt-1 border-t border-border mt-1">
                                    <p className="font-medium text-foreground mb-1">
                                      Commits ({session.commits.length}):
                                    </p>
                                    {session.commits.slice(0, 3).map(commit => (
                                      <p key={commit.hash} className="truncate text-foreground/90">
                                        <span className="text-warm font-mono">{commit.hash.slice(0, 7)}</span>
                                        {' '}{commit.message}
                                      </p>
                                    ))}
                                    {session.commits.length > 3 && (
                                      <p className="text-foreground/60">
                                        +{session.commits.length - 3} more
                                      </p>
                                    )}
                                  </div>
                                )}
                              </div>
                            </div>
                          </TooltipContent>
                        </Tooltip>
                      )
                    })}
                  </div>
                </div>
              ))
            )}
          </div>
        </TooltipProvider>
      </div>

      {/* Legend */}
      <div className="flex items-center gap-4 text-[10px] text-muted-foreground pt-2 border-t border-border">
        <div className="flex items-center gap-1.5">
          <div className="w-6 h-2 rounded bg-warm/70" />
          <span>Session 時段</span>
        </div>
        <div className="flex items-center gap-1.5">
          <div className="w-1.5 h-1.5 rounded-full bg-foreground/80" />
          <span>Git Commit</span>
        </div>
      </div>
    </div>
  )
}
