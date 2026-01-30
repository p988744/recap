import { useEffect, useState, useMemo } from 'react'
import { Clock, GitCommit, FileCode } from 'lucide-react'
import { worklog } from '@/services'
import type { WorklogDayProject, HourlyBreakdownItem } from '@/types/worklog'
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip'

interface DayGanttChartProps {
  date: string
  projects: WorklogDayProject[]
}

interface CommitMarker {
  hash: string
  message: string
  timestamp: string
  hour: number
  minute: number
}

interface HourData {
  summaries: string[]
  commits: number
  files: number
  sources: Set<string>
  commitMarkers: CommitMarker[]
}

interface TimeSpan {
  startHour: number
  endHour: number // exclusive
  data: HourData[]
  totalCommits: number
  totalFiles: number
  allSources: Set<string>
  allCommitMarkers: CommitMarker[]
}

interface ProjectRowData {
  projectPath: string
  projectName: string
  spans: TimeSpan[]
}

// Parse hour string to number (e.g., "09:00" -> 9)
function parseHour(hourStr: string): number {
  return parseInt(hourStr.split(':')[0], 10)
}

// Parse commit timestamp to local hour and minute
function parseCommitTime(timestamp: string): { hour: number; minute: number } | null {
  if (!timestamp) return null
  try {
    const date = new Date(timestamp)
    if (isNaN(date.getTime())) return null
    return {
      hour: date.getHours(),
      minute: date.getMinutes(),
    }
  } catch {
    return null
  }
}

// Format hour to full time string
function formatHour(hour: number): string {
  return `${String(hour).padStart(2, '0')}:00`
}

// Color palette for different projects
const PROJECT_COLORS = [
  'bg-blue-500/80',
  'bg-emerald-500/80',
  'bg-violet-500/80',
  'bg-amber-500/80',
  'bg-rose-500/80',
  'bg-cyan-500/80',
  'bg-indigo-500/80',
  'bg-orange-500/80',
]

export function DayGanttChart({ date, projects }: DayGanttChartProps) {
  const [hourlyData, setHourlyData] = useState<Map<string, HourlyBreakdownItem[]>>(new Map())
  const [loading, setLoading] = useState(true)

  // Fetch hourly data for all projects with hourly data
  useEffect(() => {
    const fetchAllHourly = async () => {
      setLoading(true)
      const projectsWithHourly = projects.filter(p => p.has_hourly_data)

      if (projectsWithHourly.length === 0) {
        setHourlyData(new Map())
        setLoading(false)
        return
      }

      const results = new Map<string, HourlyBreakdownItem[]>()

      await Promise.all(
        projectsWithHourly.map(async (project) => {
          try {
            const data = await worklog.getHourlyBreakdown(date, project.project_path)
            results.set(project.project_path, data)
          } catch (err) {
            console.error(`Failed to fetch hourly data for ${project.project_name}:`, err)
            results.set(project.project_path, [])
          }
        })
      )

      setHourlyData(results)
      setLoading(false)
    }

    fetchAllHourly()
  }, [date, projects])

  // Build project rows with merged consecutive time spans
  const { projectRows, minHour, maxHour } = useMemo(() => {
    const rows: ProjectRowData[] = []
    let globalMin = 24
    let globalMax = 0

    hourlyData.forEach((items, projectPath) => {
      const project = projects.find(p => p.project_path === projectPath)
      if (!project || items.length === 0) return

      // Build hour -> data map (merge multiple sources for same hour)
      const hoursMap = new Map<number, HourData>()
      items.forEach(item => {
        const hour = parseHour(item.hour_start)
        globalMin = Math.min(globalMin, hour)
        globalMax = Math.max(globalMax, hour)

        // Parse commit timestamps to markers
        const markers: CommitMarker[] = item.git_commits
          .map(commit => {
            const time = parseCommitTime(commit.timestamp)
            if (!time) return null
            return {
              hash: commit.hash,
              message: commit.message,
              timestamp: commit.timestamp,
              hour: time.hour,
              minute: time.minute,
            }
          })
          .filter((m): m is CommitMarker => m !== null)

        const existing = hoursMap.get(hour)
        if (existing) {
          // Merge with existing data for this hour
          existing.summaries.push(item.summary)
          existing.commits += item.git_commits.length
          existing.files += item.files_modified.length
          existing.sources.add(item.source)
          existing.commitMarkers.push(...markers)
        } else {
          hoursMap.set(hour, {
            summaries: [item.summary],
            commits: item.git_commits.length,
            files: item.files_modified.length,
            sources: new Set([item.source]),
            commitMarkers: markers,
          })
        }
      })

      // Merge consecutive hours into spans
      const sortedHours = Array.from(hoursMap.keys()).sort((a, b) => a - b)
      const spans: TimeSpan[] = []

      let currentSpan: TimeSpan | null = null

      for (const hour of sortedHours) {
        const data = hoursMap.get(hour)!

        if (currentSpan && hour === currentSpan.endHour) {
          // Extend current span
          currentSpan.endHour = hour + 1
          currentSpan.data.push(data)
          currentSpan.totalCommits += data.commits
          currentSpan.totalFiles += data.files
          data.sources.forEach(s => currentSpan!.allSources.add(s))
          currentSpan.allCommitMarkers.push(...data.commitMarkers)
        } else {
          // Start new span
          if (currentSpan) spans.push(currentSpan)
          currentSpan = {
            startHour: hour,
            endHour: hour + 1,
            data: [data],
            totalCommits: data.commits,
            totalFiles: data.files,
            allSources: new Set(data.sources),
            allCommitMarkers: [...data.commitMarkers],
          }
        }
      }
      if (currentSpan) spans.push(currentSpan)

      if (spans.length > 0) {
        rows.push({
          projectPath: project.project_path,
          projectName: project.project_name,
          spans,
        })
      }
    })

    // Default to work hours if no data
    if (globalMin > globalMax) {
      globalMin = 9
      globalMax = 18
    } else {
      // Add padding
      globalMin = Math.max(0, globalMin - 1)
      globalMax = Math.min(23, globalMax + 1)
    }

    return { projectRows: rows, minHour: globalMin, maxHour: globalMax }
  }, [hourlyData, projects])

  // Generate hour columns
  const hourColumns = useMemo(() => {
    const cols: number[] = []
    for (let h = minHour; h <= maxHour; h++) {
      cols.push(h)
    }
    return cols
  }, [minHour, maxHour])

  const totalColumns = hourColumns.length

  if (loading) {
    return (
      <div className="flex items-center justify-center py-8">
        <div className="w-4 h-4 border border-border border-t-foreground/60 rounded-full animate-spin" />
      </div>
    )
  }

  if (projectRows.length === 0) {
    return (
      <div className="py-6 text-center text-sm text-muted-foreground">
        無甘特圖資料
      </div>
    )
  }

  return (
    <TooltipProvider delayDuration={200}>
      <div className="space-y-2">
        {/* Header */}
        <div className="flex items-center gap-1 px-1 text-[10px] text-muted-foreground">
          <Clock className="w-3 h-3 mr-1" strokeWidth={1.5} />
          <span>今日甘特圖</span>
        </div>

        {/* Gantt Chart */}
        <div>
          <div>
            {/* Hour header row */}
            <div className="flex">
              {/* Project name column */}
              <div className="w-24 shrink-0" />

              {/* Hour columns */}
              <div className="flex-1 flex">
                {hourColumns.map((hour) => (
                  <div
                    key={hour}
                    className="flex-1 text-center text-[10px] text-muted-foreground font-mono py-1 border-l border-border/30 first:border-l-0"
                  >
                    {formatHour(hour)}
                  </div>
                ))}
              </div>
            </div>

            {/* Project rows */}
            {projectRows.map((row, rowIndex) => (
              <div key={row.projectPath} className="flex items-center h-10 border-t border-border/20">
                {/* Project name */}
                <div className="w-24 shrink-0 pr-2 flex items-center">
                  <span className="text-xs text-foreground truncate" title={row.projectName}>
                    {row.projectName}
                  </span>
                </div>

                {/* Timeline area - use relative positioning for spans */}
                <div className="flex-1 relative h-full">
                  {/* Grid lines */}
                  <div className="absolute inset-0 flex">
                    {hourColumns.map((hour) => (
                      <div
                        key={hour}
                        className="flex-1 border-l border-border/20 first:border-l-0"
                      />
                    ))}
                  </div>

                  {/* Time spans */}
                  {row.spans.map((span, spanIndex) => {
                    const startOffset = span.startHour - minHour
                    const spanWidth = span.endHour - span.startHour
                    const leftPercent = (startOffset / totalColumns) * 100
                    const widthPercent = (spanWidth / totalColumns) * 100

                    // Combine summaries for tooltip (flatten all summaries from all hours)
                    const combinedSummary = span.data
                      .flatMap(d => d.summaries)
                      .filter(s => s && s.trim())
                      .join('\n\n')

                    // Format sources for display
                    const sourcesDisplay = Array.from(span.allSources)
                      .map(s => s === 'claude_code' ? 'Claude Code' : s === 'antigravity' ? 'Antigravity' : s)
                      .join(' + ')

                    // Calculate commit marker positions within the span
                    const commitPositions = span.allCommitMarkers.map(marker => {
                      // Calculate position as percentage within the span
                      const commitHourOffset = marker.hour + marker.minute / 60 - span.startHour
                      const spanDuration = span.endHour - span.startHour
                      const positionPercent = Math.max(0, Math.min(100, (commitHourOffset / spanDuration) * 100))
                      return { ...marker, positionPercent }
                    })

                    return (
                      <Tooltip key={spanIndex}>
                        <TooltipTrigger asChild>
                          <div
                            className={`absolute top-1/2 -translate-y-1/2 h-7 rounded ${PROJECT_COLORS[rowIndex % PROJECT_COLORS.length]} cursor-pointer hover:opacity-90 transition-opacity overflow-hidden`}
                            style={{
                              left: `calc(${leftPercent}% + 2px)`,
                              width: `calc(${widthPercent}% - 4px)`,
                            }}
                          >
                            {/* Commit markers */}
                            {commitPositions.map((commit, idx) => (
                              <div
                                key={`${commit.hash}-${idx}`}
                                className="absolute top-0 bottom-0 w-0.5 bg-white/60"
                                style={{ left: `${commit.positionPercent}%` }}
                                title={`${commit.hash.slice(0, 7)}: ${commit.message}`}
                              />
                            ))}
                          </div>
                        </TooltipTrigger>
                        <TooltipContent side="top" className="max-w-sm bg-popover border border-border shadow-lg">
                          <div className="space-y-2">
                            <div className="flex items-center justify-between gap-2">
                              <span className="font-medium text-sm text-foreground">
                                {row.projectName}
                              </span>
                              <span className="text-[10px] text-muted-foreground/70">
                                {sourcesDisplay}
                              </span>
                            </div>
                            <div className="text-xs text-muted-foreground">
                              {formatHour(span.startHour)} - {formatHour(span.endHour)} ({span.endHour - span.startHour}h)
                            </div>
                            {combinedSummary && (
                              <p className="text-xs text-muted-foreground whitespace-pre-line line-clamp-4">
                                {combinedSummary}
                              </p>
                            )}
                            <div className="flex items-center gap-3 text-[10px] text-muted-foreground pt-1 border-t border-border">
                              {span.totalCommits > 0 && (
                                <span className="flex items-center gap-1">
                                  <GitCommit className="w-3 h-3" strokeWidth={1.5} />
                                  {span.totalCommits} commits
                                </span>
                              )}
                              {span.totalFiles > 0 && (
                                <span className="flex items-center gap-1">
                                  <FileCode className="w-3 h-3" strokeWidth={1.5} />
                                  {span.totalFiles} files
                                </span>
                              )}
                            </div>
                            {/* Commit timeline */}
                            {span.allCommitMarkers.length > 0 && (
                              <div className="space-y-1 pt-1 border-t border-border max-h-24 overflow-y-auto">
                                <div className="text-[10px] text-muted-foreground/70 flex items-center gap-1">
                                  <GitCommit className="w-3 h-3" strokeWidth={1.5} />
                                  Commits:
                                </div>
                                {span.allCommitMarkers
                                  .sort((a, b) => a.hour * 60 + a.minute - (b.hour * 60 + b.minute))
                                  .slice(0, 5)
                                  .map((commit, idx) => (
                                    <div key={idx} className="text-[10px] text-muted-foreground flex items-start gap-1.5">
                                      <span className="text-muted-foreground/60 font-mono shrink-0">
                                        {String(commit.hour).padStart(2, '0')}:{String(commit.minute).padStart(2, '0')}
                                      </span>
                                      <span className="truncate">{commit.message}</span>
                                    </div>
                                  ))}
                                {span.allCommitMarkers.length > 5 && (
                                  <div className="text-[10px] text-muted-foreground/50">
                                    +{span.allCommitMarkers.length - 5} more
                                  </div>
                                )}
                              </div>
                            )}
                          </div>
                        </TooltipContent>
                      </Tooltip>
                    )
                  })}
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </TooltipProvider>
  )
}
