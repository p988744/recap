import { ChevronDown, ChevronRight, Clock, FolderKanban, Plus } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import type { WorklogDay, HourlyBreakdownItem } from '@/types/worklog'
import type { WorklogSyncRecord, TempoSyncTarget } from '@/types'
import { DayDetails } from './DayDetails'
import { DailySummary } from './DailySummary'

interface DayCardProps {
  day: WorklogDay
  isToday: boolean
  isExpanded: boolean
  onToggle: () => void
  expandedProject: { date: string; projectPath: string } | null
  hourlyData: HourlyBreakdownItem[]
  hourlyLoading: boolean
  onToggleHourly: (date: string, projectPath: string) => void
  onAddManualItem: (date: string) => void
  onEditManualItem: (id: string) => void
  onDeleteManualItem: (id: string) => void
  getSyncRecord?: (projectPath: string, date: string) => WorklogSyncRecord | undefined
  onSyncProject?: (target: TempoSyncTarget) => void
  getMappedIssueKey?: (path: string) => string
  onIssueKeyChange?: (path: string, key: string) => void
}

function formatDayDisplay(dateStr: string): string {
  const d = new Date(dateStr + 'T00:00:00')
  return `${d.getMonth() + 1}/${d.getDate()}`
}

export function DayCard({
  day,
  isToday,
  isExpanded,
  onToggle,
  expandedProject,
  hourlyData,
  hourlyLoading,
  onToggleHourly,
  onAddManualItem,
  onEditManualItem,
  onDeleteManualItem,
  getSyncRecord,
  onSyncProject,
  getMappedIssueKey,
  onIssueKeyChange,
}: DayCardProps) {
  const isEmpty = day.projects.length === 0 && day.manual_items.length === 0

  // Calculate total hours and project count
  const totalHours = day.projects.reduce((sum, p) => sum + p.total_hours, 0) +
    day.manual_items.reduce((sum, m) => sum + m.hours, 0)
  const projectCount = day.projects.length + day.manual_items.length

  return (
    <Card className={`group transition-all ${isToday ? 'ring-2 ring-sage/30' : ''}`}>
      {/* Header - always visible */}
      <button
        className="w-full px-5 py-4 flex items-center justify-between text-left hover:bg-muted/30 transition-colors rounded-t-lg"
        onClick={onToggle}
      >
        <div className="flex items-center gap-3">
          {/* Expand icon */}
          <div className="text-muted-foreground">
            {isExpanded ? (
              <ChevronDown className="w-4 h-4" strokeWidth={1.5} />
            ) : (
              <ChevronRight className="w-4 h-4" strokeWidth={1.5} />
            )}
          </div>

          {/* Day info */}
          <div className="flex items-baseline gap-2">
            <span className="font-medium text-foreground">
              {day.weekday}
            </span>
            <span className="text-sm text-muted-foreground">
              {formatDayDisplay(day.date)}
            </span>
            {isToday && (
              <span className="text-[10px] uppercase tracking-wider text-sage font-medium px-1.5 py-0.5 bg-sage/10 rounded">
                今天
              </span>
            )}
          </div>
        </div>

        {/* Stats */}
        {!isEmpty && (
          <div className="flex items-center gap-4 text-sm text-muted-foreground">
            <span className="flex items-center gap-1.5">
              <Clock className="w-3.5 h-3.5" strokeWidth={1.5} />
              {totalHours.toFixed(1)}h
            </span>
            <span className="flex items-center gap-1.5">
              <FolderKanban className="w-3.5 h-3.5" strokeWidth={1.5} />
              {projectCount}
            </span>
          </div>
        )}
      </button>

      {/* Content */}
      <CardContent className={`px-5 pb-4 pt-0 ${isExpanded ? '' : 'hidden'}`}>
        {isEmpty ? (
          <div className="py-6 text-center border-t border-border">
            <p className="text-sm text-muted-foreground mb-3">尚無紀錄</p>
            <Button
              variant="outline"
              size="sm"
              onClick={(e) => { e.stopPropagation(); onAddManualItem(day.date) }}
            >
              <Plus className="w-3.5 h-3.5 mr-1.5" strokeWidth={1.5} />
              新增項目
            </Button>
          </div>
        ) : isExpanded ? (
          <DayDetails
            day={day}
            expandedProject={expandedProject}
            hourlyData={hourlyData}
            hourlyLoading={hourlyLoading}
            onToggleHourly={onToggleHourly}
            onAddManualItem={onAddManualItem}
            onEditManualItem={onEditManualItem}
            onDeleteManualItem={onDeleteManualItem}
            getSyncRecord={getSyncRecord}
            onSyncProject={onSyncProject}
            getMappedIssueKey={getMappedIssueKey}
            onIssueKeyChange={onIssueKeyChange}
          />
        ) : (
          <DailySummary day={day} />
        )}
      </CardContent>

      {/* Collapsed summary preview */}
      {!isExpanded && !isEmpty && (
        <CardContent className="px-5 pb-4 pt-0">
          <DailySummary day={day} />
        </CardContent>
      )}
    </Card>
  )
}
