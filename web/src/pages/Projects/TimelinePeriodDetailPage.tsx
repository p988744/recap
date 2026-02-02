import { useState, useEffect, useMemo } from 'react'
import { useParams, useSearchParams, useNavigate } from 'react-router-dom'
import { ArrowLeft, Clock, Calendar, GitCommit, FileCode, ChevronDown, ChevronRight } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useProjectDetail } from './hooks/useProjectDetail'
import { worklog } from '@/services'
import type { HourlyBreakdownItem } from '@/types/worklog'
import { MarkdownSummary } from '@/components/MarkdownSummary'
import { ClaudeIcon } from '@/pages/Settings/components/ProjectsSection/icons/ClaudeIcon'
import { GeminiIcon } from '@/pages/Settings/components/ProjectsSection/icons/GeminiIcon'
import { CommitDiffModal } from './components/Modals/CommitDiffModal'

// Format period label for display
function formatPeriodLabel(label: string): string {
  if (label.includes(' W')) {
    const [year, week] = label.split(' W')
    return `Week ${parseInt(week)}, ${year}`
  }

  if (label.includes(' Q')) {
    return label
  }

  if (label.match(/^\d{4}-\d{2}-\d{2}$/)) {
    const date = new Date(label)
    return date.toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
    })
  }

  if (label.match(/^\d{4}-\d{2}$/)) {
    const date = new Date(`${label}-01`)
    return date.toLocaleDateString('en-US', {
      month: 'long',
      year: 'numeric',
    })
  }

  return label
}

// Format date range for display
function formatDateRange(start: string, end: string): string {
  const startDate = new Date(start)
  const endDate = new Date(end)

  if (start === end) {
    return startDate.toLocaleDateString('en-US', {
      weekday: 'short',
      month: 'short',
      day: 'numeric',
    })
  }

  const startMonth = startDate.getMonth()
  const endMonth = endDate.getMonth()

  if (startMonth === endMonth) {
    return `${startDate.toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
    })} - ${endDate.getDate()}`
  }

  return `${startDate.toLocaleDateString('en-US', {
    month: 'short',
    day: 'numeric',
  })} - ${endDate.toLocaleDateString('en-US', {
    month: 'short',
    day: 'numeric',
  })}`
}

// Generate date range array from start to end
function getDateRange(start: string, end: string): string[] {
  const dates: string[] = []
  const startDate = new Date(start + 'T00:00:00')
  const endDate = new Date(end + 'T00:00:00')

  const current = new Date(startDate)
  while (current <= endDate) {
    dates.push(current.toISOString().split('T')[0])
    current.setDate(current.getDate() + 1)
  }

  return dates
}

// Format date for display
function formatDateDisplay(dateStr: string): string {
  const d = new Date(dateStr + 'T00:00:00')
  const weekdays = ['週日', '週一', '週二', '週三', '週四', '週五', '週六']
  return `${weekdays[d.getDay()]} ${d.getMonth() + 1}/${d.getDate()}`
}

// Source configuration
const SOURCE_CONFIG: Record<string, { icon: React.ReactNode; label: string; headerBgClass: string }> = {
  claude_code: {
    icon: <ClaudeIcon className="w-3.5 h-3.5" />,
    label: 'Claude Code',
    headerBgClass: 'bg-amber-50 dark:bg-amber-900/20',
  },
  antigravity: {
    icon: <GeminiIcon className="w-3.5 h-3.5" />,
    label: 'Antigravity',
    headerBgClass: 'bg-blue-50 dark:bg-blue-900/20',
  },
}

// Hourly item with expandable commits
interface HourlyItemProps {
  item: HourlyBreakdownItem
  projectPath: string | null
}

function HourlyItem({ item, projectPath }: HourlyItemProps) {
  const [isExpanded, setIsExpanded] = useState(false)
  const [selectedCommit, setSelectedCommit] = useState<{ hash: string; message: string } | null>(null)

  const fileNames = useMemo(() => {
    const seen = new Set<string>()
    const result: string[] = []
    for (const f of item.files_modified) {
      const name = f.split(/[/\\]/).pop() || f
      if (!seen.has(name)) {
        seen.add(name)
        result.push(name)
      }
    }
    return result
  }, [item.files_modified])

  const hasCommits = item.git_commits.length > 0

  return (
    <>
      <CommitDiffModal
        open={selectedCommit !== null}
        onOpenChange={(open) => !open && setSelectedCommit(null)}
        projectPath={projectPath}
        commitHash={selectedCommit?.hash || ''}
        commitMessage={selectedCommit?.message}
      />
      <div className="px-4 py-3 pl-11">
        {/* Time label */}
        <span className="flex items-center gap-1 text-xs text-muted-foreground mb-1.5">
          <Clock className="w-3 h-3" strokeWidth={1.5} />
          {item.hour_start}–{item.hour_end}
        </span>

        {/* Summary */}
        <MarkdownSummary content={item.summary} />

        {/* Files modified */}
        {fileNames.length > 0 && (
          <div className="mt-2">
            <div className="flex items-center gap-1 mb-1">
              <FileCode className="w-3 h-3 text-muted-foreground" strokeWidth={1.5} />
              <span className="text-[10px] uppercase tracking-wider text-muted-foreground">
                修改檔案
              </span>
            </div>
            <div className="flex flex-wrap gap-1">
              {fileNames.slice(0, 8).map((name, j) => (
                <span
                  key={j}
                  className="text-xs text-muted-foreground bg-muted/50 px-1.5 py-0.5 rounded font-mono"
                >
                  {name}
                </span>
              ))}
              {fileNames.length > 8 && (
                <span className="text-xs text-muted-foreground">
                  +{fileNames.length - 8}
                </span>
              )}
            </div>
          </div>
        )}

        {/* Git commits - expandable */}
        {hasCommits && (
          <div className="mt-2">
            <button
              onClick={() => setIsExpanded(!isExpanded)}
              className="flex items-center gap-1 mb-1 hover:text-foreground transition-colors"
            >
              {isExpanded ? (
                <ChevronDown className="w-3 h-3 text-muted-foreground" strokeWidth={1.5} />
              ) : (
                <ChevronRight className="w-3 h-3 text-muted-foreground" strokeWidth={1.5} />
              )}
              <GitCommit className="w-3 h-3 text-muted-foreground" strokeWidth={1.5} />
              <span className="text-[10px] uppercase tracking-wider text-muted-foreground">
                Commits ({item.git_commits.length})
              </span>
            </button>
            {isExpanded && (
              <div className="space-y-0.5 ml-4">
                {item.git_commits.map((commit, j) => (
                  <div
                    key={j}
                    className="flex items-baseline gap-2 py-0.5 px-1 -mx-1 rounded cursor-pointer hover:bg-muted/50 transition-colors"
                    onClick={() => setSelectedCommit({ hash: commit.hash, message: commit.message })}
                    title="Click to view diff"
                  >
                    <span className="text-xs font-mono text-muted-foreground shrink-0 bg-muted/50 px-1.5 py-0.5 rounded">
                      {commit.hash.slice(0, 7)}
                    </span>
                    <span className="text-xs text-foreground truncate">{commit.message}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </>
  )
}

// Source section with header
interface SourceSectionProps {
  source: string
  items: HourlyBreakdownItem[]
  projectPath: string | null
}

function SourceSection({ source, items, projectPath }: SourceSectionProps) {
  const config = SOURCE_CONFIG[source]
  if (!config) return null

  return (
    <div>
      {/* Source header */}
      <div className={`flex items-center gap-1.5 px-4 py-2 pl-11 ${config.headerBgClass}`}>
        {config.icon}
        <span className="text-xs font-medium text-foreground/80">{config.label}</span>
        <span className="text-xs text-muted-foreground">({items.length})</span>
      </div>
      {/* Items */}
      <div className="divide-y divide-border/50">
        {items.map((item, i) => (
          <HourlyItem key={i} item={item} projectPath={projectPath} />
        ))}
      </div>
    </div>
  )
}

// Day section with date header
interface DaySectionProps {
  date: string
  items: HourlyBreakdownItem[]
  projectPath: string | null
}

function DaySection({ date, items, projectPath }: DaySectionProps) {
  const claudeItems = items.filter(item => item.source === 'claude_code')
  const antigravityItems = items.filter(item => item.source === 'antigravity')
  const totalHours = items.length // Each item represents approximately 1 hour
  const totalCommits = items.reduce((sum, item) => sum + item.git_commits.length, 0)

  return (
    <div className="border border-border rounded-lg bg-white/60 dark:bg-white/5 overflow-hidden">
      {/* Date header */}
      <div className="px-4 py-3 bg-muted/30 border-b border-border">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <span className="text-sm font-medium text-foreground">
              {formatDateDisplay(date)}
            </span>
          </div>
          <div className="flex items-center gap-3 text-xs text-muted-foreground">
            <span className="flex items-center gap-1">
              <Clock className="w-3 h-3" strokeWidth={1.5} />
              {totalHours}h
            </span>
            {totalCommits > 0 && (
              <span className="flex items-center gap-1">
                <GitCommit className="w-3 h-3" strokeWidth={1.5} />
                {totalCommits}
              </span>
            )}
          </div>
        </div>
      </div>

      {/* Source sections */}
      <div className="divide-y divide-border">
        {claudeItems.length > 0 && (
          <SourceSection source="claude_code" items={claudeItems} projectPath={projectPath} />
        )}
        {antigravityItems.length > 0 && (
          <SourceSection source="antigravity" items={antigravityItems} projectPath={projectPath} />
        )}
      </div>
    </div>
  )
}

export function TimelinePeriodDetailPage() {
  const { projectName } = useParams<{ projectName: string }>()
  const [searchParams] = useSearchParams()
  const navigate = useNavigate()

  const periodStart = searchParams.get('start') ?? ''
  const periodEnd = searchParams.get('end') ?? ''
  const periodLabel = searchParams.get('label') ?? ''

  const decodedProjectName = decodeURIComponent(projectName ?? '')

  // Get project detail for project path
  const { detail } = useProjectDetail(decodedProjectName)
  const projectPath = detail?.project_path ?? null

  // State for hourly breakdown data
  const [hourlyData, setHourlyData] = useState<Record<string, HourlyBreakdownItem[]>>({})
  const [isLoading, setIsLoading] = useState(true)

  // Fetch hourly breakdown for each day in the period
  useEffect(() => {
    if (!projectPath || !periodStart || !periodEnd) {
      setIsLoading(false)
      return
    }

    const fetchData = async () => {
      setIsLoading(true)
      const dates = getDateRange(periodStart, periodEnd)
      const dataByDate: Record<string, HourlyBreakdownItem[]> = {}

      await Promise.all(
        dates.map(async (date) => {
          try {
            const items = await worklog.getHourlyBreakdown(date, projectPath)
            if (items.length > 0) {
              dataByDate[date] = items
            }
          } catch (err) {
            console.error(`Failed to fetch hourly breakdown for ${date}:`, err)
          }
        })
      )

      setHourlyData(dataByDate)
      setIsLoading(false)
    }

    fetchData()
  }, [projectPath, periodStart, periodEnd])

  // Calculate stats
  const { totalHours, totalCommits, datesWithData } = useMemo(() => {
    const sortedDates = Object.keys(hourlyData).sort((a, b) => b.localeCompare(a))
    let hours = 0
    let commits = 0

    for (const items of Object.values(hourlyData)) {
      hours += items.length
      commits += items.reduce((sum, item) => sum + item.git_commits.length, 0)
    }

    return { totalHours: hours, totalCommits: commits, datesWithData: sortedDates }
  }, [hourlyData])

  const formattedLabel = formatPeriodLabel(periodLabel)
  const dateRange = formatDateRange(periodStart, periodEnd)

  if (!projectName || !periodStart) {
    return (
      <div className="p-8 text-center">
        <p className="text-muted-foreground">Invalid parameters</p>
      </div>
    )
  }

  if (isLoading) {
    return (
      <div className="space-y-8">
        <Button variant="ghost" size="sm" onClick={() => navigate(-1)}>
          <ArrowLeft className="w-4 h-4 mr-2" strokeWidth={1.5} />
          返回時間軸
        </Button>

        <div className="flex items-center justify-center h-48">
          <div className="w-6 h-6 border border-border border-t-foreground/60 rounded-full animate-spin" />
        </div>
      </div>
    )
  }

  return (
    <div className="space-y-8 animate-fade-up">
      {/* Back button */}
      <Button variant="ghost" size="sm" onClick={() => navigate(-1)}>
        <ArrowLeft className="w-4 h-4 mr-2" strokeWidth={1.5} />
        返回時間軸
      </Button>

      {/* Header */}
      <div>
        <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
          {decodedProjectName}
        </p>
        <h1 className="text-2xl font-semibold text-foreground mb-2">
          {formattedLabel}
        </h1>
        <div className="flex items-center gap-6 text-sm text-muted-foreground">
          <span className="flex items-center gap-1.5">
            <Calendar className="w-4 h-4" strokeWidth={1.5} />
            {dateRange}
          </span>
          <span className="flex items-center gap-1.5">
            <Clock className="w-4 h-4" strokeWidth={1.5} />
            {totalHours}h total
          </span>
          <span className="flex items-center gap-1.5">
            <GitCommit className="w-4 h-4" strokeWidth={1.5} />
            {totalCommits} commits
          </span>
        </div>
      </div>

      {/* Hourly breakdown by date */}
      {datesWithData.length > 0 ? (
        <div className="space-y-4">
          <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
            工作紀錄 ({datesWithData.length} 天)
          </h2>
          <div className="space-y-4">
            {datesWithData.map((date) => (
              <DaySection
                key={date}
                date={date}
                items={hourlyData[date]}
                projectPath={projectPath}
              />
            ))}
          </div>
        </div>
      ) : (
        <div className="py-16 text-center">
          <p className="text-muted-foreground">此期間無工作紀錄</p>
        </div>
      )}
    </div>
  )
}
