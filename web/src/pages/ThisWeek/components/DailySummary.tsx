import type { WorklogDay } from '@/types/worklog'

interface DailySummaryProps {
  day: WorklogDay
}

export function DailySummary({ day }: DailySummaryProps) {
  // Create a brief summary from project summaries
  const summaries = day.projects
    .filter(p => p.daily_summary)
    .map(p => p.daily_summary!)
    .slice(0, 2) // Show max 2 summaries

  const projectNames = day.projects.map(p => p.project_name)
  const manualItemNames = day.manual_items.map(m => m.title)
  const allNames = [...projectNames, ...manualItemNames]

  return (
    <div className="border-t border-border pt-3 space-y-2">
      {/* Project/item names */}
      <div className="flex flex-wrap gap-2">
        {allNames.map((name, i) => (
          <span
            key={i}
            className="text-xs px-2 py-1 bg-muted/50 text-muted-foreground rounded"
          >
            {name}
          </span>
        ))}
      </div>

      {/* Brief summaries */}
      {summaries.length > 0 && (
        <div className="text-sm text-muted-foreground">
          {summaries.map((summary, i) => {
            // Truncate to first line or 100 chars
            const truncated = summary.split('\n')[0].slice(0, 100)
            return (
              <p key={i} className="truncate">
                {truncated}{truncated.length >= 100 ? '...' : ''}
              </p>
            )
          })}
        </div>
      )}
    </div>
  )
}
