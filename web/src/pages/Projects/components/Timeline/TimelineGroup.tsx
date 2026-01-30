import { useNavigate } from 'react-router-dom'
import { Calendar, Clock, ChevronRight } from 'lucide-react'
import type { TimelineGroup as TimelineGroupType } from '@/types'
import { MarkdownSummary } from '@/components/MarkdownSummary'

interface TimelineGroupProps {
  group: TimelineGroupType
  projectName: string
  summary: string | null
}

function formatPeriodLabel(label: string): string {
  // Handle different formats:
  // "2026-01-30" -> "Jan 30, 2026"
  // "2026 W05" -> "Week 5, 2026"
  // "2026-01" -> "January 2026"
  // "2026 Q1" -> "Q1 2026"
  // "2026" -> "2026"

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

export function TimelineGroupComponent({ group, projectName, summary }: TimelineGroupProps) {
  const navigate = useNavigate()
  const formattedLabel = formatPeriodLabel(group.period_label)
  const dateRange = formatDateRange(group.period_start, group.period_end)

  const handleViewDetails = () => {
    // Navigate to period detail page with project and period info
    const params = new URLSearchParams({
      start: group.period_start,
      end: group.period_end,
      label: group.period_label,
    })
    navigate(`/projects/${encodeURIComponent(projectName)}/period?${params.toString()}`)
  }

  return (
    <div className="space-y-4">
      {/* Period header */}
      <div className="flex items-baseline justify-between">
        <div className="flex items-baseline gap-3">
          <h3 className="text-lg font-semibold">{formattedLabel}</h3>
          <span className="text-sm text-muted-foreground flex items-center gap-1">
            <Calendar className="w-3.5 h-3.5" />
            {dateRange}
          </span>
        </div>
        <div className="flex items-center gap-1 text-sm text-muted-foreground">
          <Clock className="w-3.5 h-3.5" />
          {group.total_hours.toFixed(1)}h total
        </div>
      </div>

      {/* Period summary with markdown rendering */}
      {summary && (
        <div className="bg-muted/30 rounded-md px-4 py-3">
          <MarkdownSummary content={summary} />
        </div>
      )}

      {/* View details link - replaces session list */}
      {group.sessions.length > 0 ? (
        <button
          onClick={handleViewDetails}
          className="w-full flex items-center justify-between px-4 py-3 text-sm text-muted-foreground hover:text-foreground hover:bg-muted/50 rounded-md transition-colors cursor-pointer group"
        >
          <span>{group.sessions.length} sessions in this period</span>
          <ChevronRight className="w-4 h-4 transition-transform group-hover:translate-x-0.5" />
        </button>
      ) : (
        <div className="py-4 text-center text-sm text-muted-foreground">
          No sessions in this period
        </div>
      )}
    </div>
  )
}
