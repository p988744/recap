import { useEffect, useState, useMemo } from 'react'
import { Clock, GitCommit, FileCode } from 'lucide-react'
import { worklog } from '@/services'
import type { WorklogDayProject, HourlyBreakdownItem } from '@/types/worklog'
import { ClaudeIcon } from '@/pages/Settings/components/ProjectsSection/icons/ClaudeIcon'
import { GeminiIcon } from '@/pages/Settings/components/ProjectsSection/icons/GeminiIcon'

interface DayGanttChartProps {
  date: string
  projects: WorklogDayProject[]
}

interface HourlySlot {
  hour: string // "09:00"
  hourEnd: string // "10:00"
  items: Array<{
    projectName: string
    projectPath: string
    summary: string
    commits: number
    files: number
    source: string
  }>
}

// Generate hour labels from 00:00 to 23:00
function generateHourLabels(): string[] {
  return Array.from({ length: 24 }, (_, i) =>
    `${String(i).padStart(2, '0')}:00`
  )
}

// Parse hour string to number (e.g., "09:00" -> 9)
function parseHour(hourStr: string): number {
  return parseInt(hourStr.split(':')[0], 10)
}

const SOURCE_ICONS: Record<string, React.ReactNode> = {
  claude_code: <ClaudeIcon className="w-3 h-3" />,
  antigravity: <GeminiIcon className="w-3 h-3" />,
}

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

  // Build timeline slots
  const timelineSlots = useMemo(() => {
    const slots: HourlySlot[] = []
    const hourLabels = generateHourLabels()

    // Create a map of hour -> items
    const hourMap = new Map<string, HourlySlot['items']>()

    hourlyData.forEach((items, projectPath) => {
      const project = projects.find(p => p.project_path === projectPath)
      if (!project) return

      items.forEach(item => {
        const hourKey = item.hour_start
        if (!hourMap.has(hourKey)) {
          hourMap.set(hourKey, [])
        }
        hourMap.get(hourKey)!.push({
          projectName: project.project_name,
          projectPath: project.project_path,
          summary: item.summary,
          commits: item.git_commits.length,
          files: item.files_modified.length,
          source: item.source,
        })
      })
    })

    // Only include hours that have data
    hourLabels.forEach((hour, idx) => {
      const items = hourMap.get(hour)
      if (items && items.length > 0) {
        slots.push({
          hour,
          hourEnd: hourLabels[idx + 1] || '24:00',
          items,
        })
      }
    })

    return slots
  }, [hourlyData, projects])

  // Find the range of active hours
  const { minHour, maxHour } = useMemo(() => {
    if (timelineSlots.length === 0) {
      return { minHour: 9, maxHour: 18 } // Default work hours
    }
    const hours = timelineSlots.map(s => parseHour(s.hour))
    return {
      minHour: Math.min(...hours),
      maxHour: Math.max(...hours) + 1,
    }
  }, [timelineSlots])

  if (loading) {
    return (
      <div className="flex items-center justify-center py-8">
        <div className="w-4 h-4 border border-border border-t-foreground/60 rounded-full animate-spin" />
      </div>
    )
  }

  if (timelineSlots.length === 0) {
    return (
      <div className="py-6 text-center text-sm text-muted-foreground">
        無甘特圖資料
      </div>
    )
  }

  return (
    <div className="space-y-1">
      {/* Gantt chart header - hour markers */}
      <div className="flex items-center gap-1 px-2 py-1 text-[10px] text-muted-foreground">
        <Clock className="w-3 h-3 mr-1" strokeWidth={1.5} />
        <span>今日甘特圖</span>
        <span className="ml-auto">
          {String(minHour).padStart(2, '0')}:00 - {String(maxHour).padStart(2, '0')}:00
        </span>
      </div>

      {/* Timeline slots */}
      <div className="space-y-1">
        {timelineSlots.map((slot) => (
          <div
            key={slot.hour}
            className="flex items-stretch gap-3 bg-muted/30 rounded-lg overflow-hidden"
          >
            {/* Time label */}
            <div className="w-16 shrink-0 bg-muted/50 flex items-center justify-center py-2">
              <span className="text-xs font-mono text-muted-foreground">
                {slot.hour}
              </span>
            </div>

            {/* Items */}
            <div className="flex-1 py-2 pr-3 space-y-1">
              {slot.items.map((item, idx) => (
                <div key={idx} className="flex items-start gap-2">
                  {/* Source icon */}
                  <div className="mt-0.5 text-muted-foreground">
                    {SOURCE_ICONS[item.source] || <div className="w-3 h-3" />}
                  </div>

                  {/* Content */}
                  <div className="flex-1 min-w-0">
                    {/* Project name and stats */}
                    <div className="flex items-center gap-2 mb-0.5">
                      <span className="text-xs font-medium text-foreground">
                        {item.projectName}
                      </span>
                      {item.commits > 0 && (
                        <span className="flex items-center gap-0.5 text-[10px] text-muted-foreground">
                          <GitCommit className="w-2.5 h-2.5" strokeWidth={1.5} />
                          {item.commits}
                        </span>
                      )}
                      {item.files > 0 && (
                        <span className="flex items-center gap-0.5 text-[10px] text-muted-foreground">
                          <FileCode className="w-2.5 h-2.5" strokeWidth={1.5} />
                          {item.files}
                        </span>
                      )}
                    </div>

                    {/* Summary */}
                    <p className="text-xs text-muted-foreground line-clamp-2">
                      {item.summary}
                    </p>
                  </div>
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
