import { CalendarDays, Filter, Bot, Sparkles } from 'lucide-react'
import { Card, CardContent } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { Checkbox } from '@/components/ui/checkbox'
import { Label } from '@/components/ui/label'
import { Progress } from '@/components/ui/progress'
import { WorkGanttChart } from '@/components/WorkGanttChart'
import type { TimelineSession } from '@/components/WorkGanttChart'
import type { WorkItemStatsResponse } from '@/types'
import { getWeekProgress } from '@/lib/utils'

interface WeekOverviewProps {
  stats: WorkItemStatsResponse | null
  projectCount: number
  daysWorked: number
  ganttDate: string
  ganttSessions: TimelineSession[]
  ganttLoading: boolean
  ganttSources: string[]
  onGanttDateChange: (date: string) => void
  onGanttSourcesChange: (sources: string[]) => void
}

export function WeekOverview({
  stats,
  projectCount,
  daysWorked,
  ganttDate,
  ganttSessions,
  ganttLoading,
  ganttSources,
  onGanttDateChange,
  onGanttSourcesChange,
}: WeekOverviewProps) {
  const weekProgress = getWeekProgress()

  return (
    <section className="space-y-8 animate-fade-up opacity-0 delay-2">
      {/* Stats Grid */}
      <div className="grid grid-cols-3 gap-6">
        {/* Weekly Hours - Featured stat */}
        <Card className="col-span-2 border-l-2 border-l-warm/60">
          <CardContent className="p-6">
            <div className="flex items-end justify-between">
              <div>
                <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
                  本週工時
                </p>
                <p className="font-display text-5xl text-foreground tracking-tight">
                  {(stats?.total_hours ?? 0).toFixed(1)}
                </p>
                <p className="text-sm text-muted-foreground mt-1">小時</p>
              </div>
              <div className="text-right">
                <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-1">
                  目標進度
                </p>
                <p className="text-xl text-muted-foreground">{weekProgress.toFixed(0)}%</p>
                <div className="w-24 mt-2">
                  <Progress value={Math.min(weekProgress, 100)} className="h-1" />
                </div>
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Secondary stats */}
        <div className="space-y-4">
          <Card className="border-l-2 border-l-sage/40">
            <CardContent className="p-4">
              <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-1">
                專案數
              </p>
              <p className="font-display text-2xl text-foreground">{projectCount}</p>
            </CardContent>
          </Card>
          <Card className="border-l-2 border-l-stone/40">
            <CardContent className="p-4">
              <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-1">
                工作天數
              </p>
              <p className="font-display text-2xl text-foreground">{daysWorked}</p>
            </CardContent>
          </Card>
        </div>
      </div>

      {/* Daily Work Gantt Chart */}
      <div>
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <CalendarDays className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
            <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
              每日工作時間軸
            </h2>
          </div>
          {/* Source Filter */}
          <Popover>
            <PopoverTrigger asChild>
              <Button variant="outline" size="sm" className="h-8 gap-2">
                <Filter className="h-3.5 w-3.5" />
                <span className="text-xs">
                  {ganttSources.length === 2
                    ? '全部來源'
                    : ganttSources.includes('claude_code')
                      ? 'Claude Code'
                      : 'Antigravity'}
                </span>
              </Button>
            </PopoverTrigger>
            <PopoverContent className="w-48 p-3" align="end">
              <div className="space-y-3">
                <p className="text-xs font-medium text-muted-foreground">資料來源</p>
                <div className="space-y-2">
                  <div className="flex items-center space-x-2">
                    <Checkbox
                      id="tw-source-claude_code"
                      checked={ganttSources.includes('claude_code')}
                      disabled={ganttSources.includes('claude_code') && ganttSources.length === 1}
                      onCheckedChange={(checked) => {
                        if (checked) {
                          onGanttSourcesChange([...ganttSources, 'claude_code'])
                        } else if (ganttSources.length > 1) {
                          onGanttSourcesChange(ganttSources.filter(s => s !== 'claude_code'))
                        }
                      }}
                    />
                    <Label htmlFor="tw-source-claude_code" className="flex items-center gap-2 text-sm font-normal cursor-pointer">
                      <Bot className="h-3.5 w-3.5 text-muted-foreground" />
                      Claude Code
                    </Label>
                  </div>
                  <div className="flex items-center space-x-2">
                    <Checkbox
                      id="tw-source-antigravity"
                      checked={ganttSources.includes('antigravity')}
                      disabled={ganttSources.includes('antigravity') && ganttSources.length === 1}
                      onCheckedChange={(checked) => {
                        if (checked) {
                          onGanttSourcesChange([...ganttSources, 'antigravity'])
                        } else if (ganttSources.length > 1) {
                          onGanttSourcesChange(ganttSources.filter(s => s !== 'antigravity'))
                        }
                      }}
                    />
                    <Label htmlFor="tw-source-antigravity" className="flex items-center gap-2 text-sm font-normal cursor-pointer">
                      <Sparkles className="h-3.5 w-3.5 text-muted-foreground" />
                      Antigravity
                    </Label>
                  </div>
                </div>
              </div>
            </PopoverContent>
          </Popover>
        </div>
        <Card>
          <CardContent className="p-6">
            {ganttLoading ? (
              <div className="flex items-center justify-center h-32">
                <div className="w-5 h-5 border border-border border-t-foreground/60 rounded-full animate-spin" />
              </div>
            ) : (
              <WorkGanttChart
                sessions={ganttSessions}
                date={ganttDate}
                onDateChange={onGanttDateChange}
              />
            )}
          </CardContent>
        </Card>
      </div>
    </section>
  )
}
