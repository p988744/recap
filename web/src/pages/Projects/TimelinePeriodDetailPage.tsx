import { useParams, useSearchParams, useNavigate } from 'react-router-dom'
import { ArrowLeft, Clock, Calendar, GitCommit } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useProjectTimeline } from './hooks/useProjectTimeline'
import { TimelineSession } from './components/Timeline/TimelineSession'
import { useProjectDetail } from './hooks/useProjectDetail'

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

  // Get timeline data for this project
  const { groups, isLoading } = useProjectTimeline({ projectName: decodedProjectName })

  // Find the specific period group
  const periodGroup = groups.find(g => g.period_start === periodStart)

  // Calculate stats
  const sessions = periodGroup?.sessions ?? []
  const totalHours = periodGroup?.total_hours ?? 0
  const totalCommits = sessions.reduce((sum, s) => sum + (s.commits?.length ?? 0), 0)

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
            {totalHours.toFixed(1)}h total
          </span>
          <span className="flex items-center gap-1.5">
            <GitCommit className="w-4 h-4" strokeWidth={1.5} />
            {totalCommits} commits
          </span>
        </div>
      </div>

      {/* Sessions list */}
      {sessions.length > 0 ? (
        <div className="space-y-4">
          <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
            Sessions ({sessions.length})
          </h2>
          <div className="space-y-3">
            {sessions.map((session) => (
              <TimelineSession
                key={session.id}
                session={session}
                projectPath={detail?.project_path ?? null}
              />
            ))}
          </div>
        </div>
      ) : (
        <div className="py-16 text-center">
          <p className="text-muted-foreground">No sessions in this period</p>
        </div>
      )}
    </div>
  )
}
