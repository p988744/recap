import { useMemo } from 'react'
import { useNavigate } from 'react-router-dom'
import { cn } from '@/lib/utils'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { GitCommit, FileCode } from 'lucide-react'
import type { WorklogDay } from '@/types/worklog'

interface WeekTimelineProps {
  days: WorklogDay[]
  startDate: string
  endDate: string
  weekStartDay?: number // 0 = Sunday, 1 = Monday (default), etc.
}

// Get weekday label in Chinese based on actual day of week (0=Sunday, 1=Monday, ...)
function getWeekdayLabel(dayOfWeek: number): string {
  const labels = ['週日', '週一', '週二', '週三', '週四', '週五', '週六']
  return labels[dayOfWeek] || ''
}

// Format date as YYYY-MM-DD using local timezone
function formatDateLocal(d: Date): string {
  const year = d.getFullYear()
  const month = String(d.getMonth() + 1).padStart(2, '0')
  const day = String(d.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
}

// Check if a date is today
function isToday(dateStr: string): boolean {
  const today = formatDateLocal(new Date())
  return dateStr === today
}

// Format date for display (MM/DD)
function formatShortDate(dateStr: string): string {
  const d = new Date(dateStr + 'T00:00:00')
  return `${d.getMonth() + 1}/${d.getDate()}`
}

// Get heatmap color based on hours (0-8+ hours scale)
function getHeatmapColor(hours: number): string {
  if (hours === 0) return 'bg-muted/30'
  if (hours < 1) return 'bg-sage/20'
  if (hours < 2) return 'bg-sage/35'
  if (hours < 4) return 'bg-sage/50'
  if (hours < 6) return 'bg-sage/65'
  if (hours < 8) return 'bg-sage/80'
  return 'bg-sage' // 8+ hours
}

// Get text color for contrast
function getTextColor(hours: number): string {
  if (hours >= 6) return 'text-white'
  return 'text-foreground'
}

interface ProjectWeekData {
  projectName: string
  projectPath: string
  dailyData: Array<{
    date: string
    hours: number
    commits: number
    files: number
    summary?: string
  } | null>
  totalHours: number
  totalCommits: number
  totalFiles: number
}

// Get day of week from date string (0=Sunday, 1=Monday, ...)
function getDayOfWeek(dateStr: string): number {
  const d = new Date(dateStr + 'T00:00:00')
  return d.getDay()
}

export function WeekTimeline({ days, startDate }: WeekTimelineProps) {
  const navigate = useNavigate()

  // Navigate to day detail page
  const handleDateClick = (date: string) => {
    navigate(`/day/${date}`)
  }

  // Navigate to project day detail page
  const handleProjectClick = (date: string, projectPath: string) => {
    navigate(`/day/${date}/${encodeURIComponent(projectPath)}`)
  }

  // Generate all dates in the week
  const weekDates = useMemo(() => {
    const dates: string[] = []
    const start = new Date(startDate + 'T00:00:00')
    for (let i = 0; i < 7; i++) {
      const d = new Date(start)
      d.setDate(start.getDate() + i)
      dates.push(formatDateLocal(d))
    }
    return dates
  }, [startDate])

  // Aggregate project data across the week
  const projectsData = useMemo(() => {
    const projectMap = new Map<string, ProjectWeekData>()

    days.forEach(day => {
      day.projects.forEach(project => {
        if (!projectMap.has(project.project_path)) {
          projectMap.set(project.project_path, {
            projectName: project.project_name,
            projectPath: project.project_path,
            dailyData: weekDates.map(() => null),
            totalHours: 0,
            totalCommits: 0,
            totalFiles: 0,
          })
        }

        const data = projectMap.get(project.project_path)!
        const dayIndex = weekDates.indexOf(day.date)
        if (dayIndex !== -1) {
          data.dailyData[dayIndex] = {
            date: day.date,
            hours: project.total_hours,
            commits: project.total_commits,
            files: project.total_files,
            summary: project.daily_summary,
          }
          data.totalHours += project.total_hours
          data.totalCommits += project.total_commits
          data.totalFiles += project.total_files
        }
      })
    })

    // Sort by total hours descending
    return Array.from(projectMap.values()).sort((a, b) => b.totalHours - a.totalHours)
  }, [days, weekDates])

  // Week stats (projects from heatmap + manual items for total hours)
  const weekStats = useMemo(() => {
    const projectHours = projectsData.reduce((sum, p) => sum + p.totalHours, 0)
    const manualHours = days.reduce((sum, d) => sum + d.manual_items.reduce((s, m) => s + m.hours, 0), 0)
    return {
      totalHours: projectHours + manualHours,
      totalCommits: projectsData.reduce((sum, p) => sum + p.totalCommits, 0),
      totalProjects: projectsData.length, // Only count projects shown in heatmap
    }
  }, [projectsData, days])

  if (projectsData.length === 0) {
    return (
      <div className="text-center text-muted-foreground text-sm py-8">
        本週尚無工作紀錄
      </div>
    )
  }

  return (
    <div className="space-y-4">
      {/* Header stats */}
      <div className="flex items-center justify-end gap-4 text-xs text-muted-foreground">
        <span>{weekStats.totalProjects} 個專案</span>
        <span className="flex items-center gap-1">
          <GitCommit className="h-3 w-3" />
          {weekStats.totalCommits} commits
        </span>
        <span>{weekStats.totalHours.toFixed(1)}h</span>
      </div>

      {/* Heatmap */}
      <div className="relative">
        {/* Day headers */}
        <div className="grid grid-cols-[120px_repeat(7,1fr)] gap-1 mb-2">
          <div /> {/* Empty corner */}
          {weekDates.map((date) => (
            <button
              key={date}
              type="button"
              onClick={() => handleDateClick(date)}
              className={cn(
                'text-center py-1 rounded cursor-pointer transition-all hover:bg-muted/50',
                isToday(date) && 'bg-sage/10 hover:bg-sage/20'
              )}
            >
              <div className={cn(
                'text-[10px] font-medium',
                isToday(date) ? 'text-sage' : 'text-muted-foreground'
              )}>
                {getWeekdayLabel(getDayOfWeek(date))}
              </div>
              <div className={cn(
                'text-xs',
                isToday(date) ? 'text-sage font-medium' : 'text-muted-foreground/60'
              )}>
                {formatShortDate(date)}
              </div>
            </button>
          ))}
        </div>

        {/* Project rows */}
        <TooltipProvider delayDuration={100}>
          <div className="space-y-1">
            {projectsData.map((project) => (
              <div
                key={project.projectPath}
                className="grid grid-cols-[120px_repeat(7,1fr)] gap-1 group"
              >
                {/* Project label */}
                <div className="flex items-center gap-2 pr-2">
                  <span className="text-xs text-muted-foreground truncate">
                    {project.projectName}
                  </span>
                </div>

                {/* Daily cells */}
                {project.dailyData.map((dayData, dayIndex) => {
                  const date = weekDates[dayIndex]
                  const hours = dayData?.hours ?? 0
                  const dayOfWeek = getDayOfWeek(date)
                  const isClickable = hours > 0

                  return (
                    <Tooltip key={date}>
                      <TooltipTrigger asChild>
                        <div
                          role={isClickable ? 'button' : undefined}
                          tabIndex={isClickable ? 0 : undefined}
                          onClick={isClickable ? () => handleProjectClick(date, project.projectPath) : undefined}
                          onKeyDown={isClickable ? (e) => {
                            if (e.key === 'Enter' || e.key === ' ') {
                              e.preventDefault()
                              handleProjectClick(date, project.projectPath)
                            }
                          } : undefined}
                          className={cn(
                            'h-8 rounded flex items-center justify-center transition-all',
                            getHeatmapColor(hours),
                            isToday(date) && 'ring-1 ring-sage/40',
                            isClickable && 'cursor-pointer hover:ring-2 hover:ring-foreground/20'
                          )}
                        >
                          {hours > 0 && (
                            <span className={cn(
                              'text-xs font-medium',
                              getTextColor(hours)
                            )}>
                              {hours.toFixed(1)}
                            </span>
                          )}
                        </div>
                      </TooltipTrigger>
                      <TooltipContent
                        side="top"
                        className="bg-popover border border-border px-3 py-2 max-w-xs shadow-lg"
                      >
                        {dayData ? (
                          <div className="space-y-2">
                            <div className="flex items-center justify-between gap-4">
                              <span className="text-sm font-medium text-foreground">
                                {project.projectName}
                              </span>
                              <span className="text-sm font-medium text-foreground">
                                {dayData.hours.toFixed(1)}h
                              </span>
                            </div>
                            <div className="text-xs text-muted-foreground">
                              {getWeekdayLabel(dayOfWeek)} {formatShortDate(date)}
                            </div>
                            <div className="flex items-center gap-3 text-xs text-muted-foreground">
                              {dayData.commits > 0 && (
                                <span className="flex items-center gap-1">
                                  <GitCommit className="h-3 w-3" />
                                  {dayData.commits}
                                </span>
                              )}
                              {dayData.files > 0 && (
                                <span className="flex items-center gap-1">
                                  <FileCode className="h-3 w-3" />
                                  {dayData.files}
                                </span>
                              )}
                            </div>
                            {dayData.summary && (
                              <p className="text-xs text-muted-foreground line-clamp-2 pt-1 border-t border-border">
                                {dayData.summary}
                              </p>
                            )}
                          </div>
                        ) : (
                          <div className="text-xs text-muted-foreground">
                            {getWeekdayLabel(dayOfWeek)} {formatShortDate(date)} - 無紀錄
                          </div>
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

      {/* Legend */}
      <div className="flex items-center gap-2 text-[10px] text-muted-foreground pt-2 border-t border-border">
        <span>工時：</span>
        <div className="flex items-center gap-1">
          <div className="w-4 h-3 rounded bg-muted/30" />
          <span>0</span>
        </div>
        <div className="flex items-center gap-1">
          <div className="w-4 h-3 rounded bg-sage/20" />
          <span>&lt;1h</span>
        </div>
        <div className="flex items-center gap-1">
          <div className="w-4 h-3 rounded bg-sage/50" />
          <span>2-4h</span>
        </div>
        <div className="flex items-center gap-1">
          <div className="w-4 h-3 rounded bg-sage/80" />
          <span>6-8h</span>
        </div>
        <div className="flex items-center gap-1">
          <div className="w-4 h-3 rounded bg-sage" />
          <span>8h+</span>
        </div>
      </div>
    </div>
  )
}
