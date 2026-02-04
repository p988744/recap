import { Plus, Clock, FolderKanban, Sparkles } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import type { WorklogDay, HourlyBreakdownItem } from '@/types/worklog'
import type { WorklogSyncRecord, TempoSyncTarget } from '@/types'
import { DayDetails } from './DayDetails'
import { DayGanttChart } from './DayGanttChart'

interface TodayWorkSectionProps {
  day: WorklogDay | null
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

export function TodayWorkSection({
  day,
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
}: TodayWorkSectionProps) {
  const today = new Date().toISOString().split('T')[0]
  const isEmpty = !day || (day.projects.length === 0 && day.manual_items.length === 0)

  // Calculate stats
  const totalHours = day
    ? day.projects.reduce((sum, p) => sum + p.total_hours, 0) +
      day.manual_items.reduce((sum, m) => sum + m.hours, 0)
    : 0
  const projectCount = day ? day.projects.length + day.manual_items.length : 0

  return (
    <section className="space-y-4 animate-fade-up opacity-0 delay-3">
      <div className="flex items-center gap-2">
        <Sparkles className="w-4 h-4 text-sage" strokeWidth={1.5} />
        <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
          今日工作
        </h2>
      </div>

      <Card className="ring-2 ring-sage/30">
        {/* Header */}
        <div className="px-5 py-4 flex items-center justify-between border-b border-border">
          <div className="flex items-center gap-3">
            <span className="text-[10px] uppercase tracking-wider text-sage font-medium px-2 py-1 bg-sage/10 rounded">
              Today
            </span>
            {!isEmpty && (
              <div className="flex items-center gap-4 text-sm text-muted-foreground">
                <span className="flex items-center gap-1.5">
                  <Clock className="w-3.5 h-3.5" strokeWidth={1.5} />
                  {totalHours.toFixed(1)}h
                </span>
                <span className="flex items-center gap-1.5">
                  <FolderKanban className="w-3.5 h-3.5" strokeWidth={1.5} />
                  {projectCount} 個專案
                </span>
              </div>
            )}
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={() => onAddManualItem(today)}
          >
            <Plus className="w-3.5 h-3.5 mr-1.5" strokeWidth={1.5} />
            新增項目
          </Button>
        </div>

        {/* Content */}
        <CardContent className="px-5 py-4">
          {isEmpty ? (
            <div className="py-8 text-center">
              <p className="text-sm text-muted-foreground mb-4">今日尚無工作紀錄</p>
              <p className="text-xs text-muted-foreground/60">
                開始使用 Claude Code 或 Antigravity 工作，或手動新增項目
              </p>
            </div>
          ) : (
            <div className="space-y-6">
              {/* Gantt Chart - hourly timeline */}
              <DayGanttChart date={today} projects={day.projects} manualItems={day.manual_items} />

              {/* Detailed work items */}
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
            </div>
          )}
        </CardContent>
      </Card>
    </section>
  )
}
