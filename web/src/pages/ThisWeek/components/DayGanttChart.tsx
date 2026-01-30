import { useEffect, useState, useMemo } from 'react'
import { Clock, GitCommit, FileCode } from 'lucide-react'
import { worklog } from '@/services'
import type { WorklogDayProject, HourlyBreakdownItem } from '@/types/worklog'
import { ClaudeIcon } from '@/pages/Settings/components/ProjectsSection/icons/ClaudeIcon'
import { GeminiIcon } from '@/pages/Settings/components/ProjectsSection/icons/GeminiIcon'
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

interface ProjectHourData {
  projectPath: string
  projectName: string
  hours: Map<number, {
    summary: string
    commits: number
    files: number
    source: string
  }>
}

// Parse hour string to number (e.g., "09:00" -> 9)
function parseHour(hourStr: string): number {
  return parseInt(hourStr.split(':')[0], 10)
}

const SOURCE_ICONS: Record<string, React.ReactNode> = {
  claude_code: <ClaudeIcon className="w-3 h-3" />,
  antigravity: <GeminiIcon className="w-3 h-3" />,
}

// Color palette for different projects
const PROJECT_COLORS = [
  'bg-blue-500/70',
  'bg-emerald-500/70',
  'bg-violet-500/70',
  'bg-amber-500/70',
  'bg-rose-500/70',
  'bg-cyan-500/70',
  'bg-indigo-500/70',
  'bg-orange-500/70',
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

  // Build project hour data for Gantt chart
  const { projectRows, minHour, maxHour } = useMemo(() => {
    const rows: ProjectHourData[] = []
    let min = 24
    let max = 0

    hourlyData.forEach((items, projectPath) => {
      const project = projects.find(p => p.project_path === projectPath)
      if (!project) return

      const hoursMap = new Map<number, {
        summary: string
        commits: number
        files: number
        source: string
      }>()

      items.forEach(item => {
        const hour = parseHour(item.hour_start)
        min = Math.min(min, hour)
        max = Math.max(max, hour)

        hoursMap.set(hour, {
          summary: item.summary,
          commits: item.git_commits.length,
          files: item.files_modified.length,
          source: item.source,
        })
      })

      if (hoursMap.size > 0) {
        rows.push({
          projectPath: project.project_path,
          projectName: project.project_name,
          hours: hoursMap,
        })
      }
    })

    // Default to work hours if no data
    if (min > max) {
      min = 9
      max = 18
    } else {
      // Add padding
      min = Math.max(0, min - 1)
      max = Math.min(23, max + 1)
    }

    return { projectRows: rows, minHour: min, maxHour: max }
  }, [hourlyData, projects])

  // Generate hour columns
  const hourColumns = useMemo(() => {
    const cols: number[] = []
    for (let h = minHour; h <= maxHour; h++) {
      cols.push(h)
    }
    return cols
  }, [minHour, maxHour])

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
        <div className="overflow-x-auto">
          <div className="min-w-fit">
            {/* Hour header row */}
            <div className="flex">
              {/* Project name column */}
              <div className="w-32 shrink-0" />

              {/* Hour columns */}
              <div className="flex-1 flex">
                {hourColumns.map((hour) => (
                  <div
                    key={hour}
                    className="flex-1 min-w-[40px] text-center text-[10px] text-muted-foreground font-mono py-1 border-l border-border/30 first:border-l-0"
                  >
                    {String(hour).padStart(2, '0')}
                  </div>
                ))}
              </div>
            </div>

            {/* Project rows */}
            {projectRows.map((row, rowIndex) => (
              <div key={row.projectPath} className="flex items-center h-10 border-t border-border/20">
                {/* Project name */}
                <div className="w-32 shrink-0 pr-2 flex items-center gap-1.5">
                  <span className="text-xs text-foreground truncate" title={row.projectName}>
                    {row.projectName}
                  </span>
                </div>

                {/* Hour cells */}
                <div className="flex-1 flex h-full">
                  {hourColumns.map((hour) => {
                    const hourData = row.hours.get(hour)
                    const hasWork = !!hourData

                    return (
                      <div
                        key={hour}
                        className="flex-1 min-w-[40px] h-full flex items-center justify-center border-l border-border/20 first:border-l-0"
                      >
                        {hasWork && (
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <div
                                className={`w-[90%] h-6 rounded ${PROJECT_COLORS[rowIndex % PROJECT_COLORS.length]} cursor-pointer hover:opacity-80 transition-opacity flex items-center justify-center gap-1`}
                              >
                                {/* Source icon */}
                                <div className="text-white/90">
                                  {SOURCE_ICONS[hourData.source] || null}
                                </div>
                                {/* Commit indicator */}
                                {hourData.commits > 0 && (
                                  <span className="text-[9px] text-white/90 font-medium">
                                    {hourData.commits}
                                  </span>
                                )}
                              </div>
                            </TooltipTrigger>
                            <TooltipContent side="top" className="max-w-xs">
                              <div className="space-y-1">
                                <div className="font-medium text-xs">
                                  {row.projectName} · {String(hour).padStart(2, '0')}:00
                                </div>
                                <p className="text-xs text-muted-foreground">
                                  {hourData.summary}
                                </p>
                                <div className="flex items-center gap-3 text-[10px] text-muted-foreground pt-1">
                                  {hourData.commits > 0 && (
                                    <span className="flex items-center gap-1">
                                      <GitCommit className="w-3 h-3" strokeWidth={1.5} />
                                      {hourData.commits} commits
                                    </span>
                                  )}
                                  {hourData.files > 0 && (
                                    <span className="flex items-center gap-1">
                                      <FileCode className="w-3 h-3" strokeWidth={1.5} />
                                      {hourData.files} files
                                    </span>
                                  )}
                                </div>
                              </div>
                            </TooltipContent>
                          </Tooltip>
                        )}
                      </div>
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
