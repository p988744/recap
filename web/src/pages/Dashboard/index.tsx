import { TrendingUp, CalendarDays, Filter, Bot, Sparkles } from 'lucide-react'
import { Card, CardContent } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { Checkbox } from '@/components/ui/checkbox'
import { Label } from '@/components/ui/label'
import { ContributionHeatmap } from '@/components/ContributionHeatmap'
import { WorkGanttChart } from '@/components/WorkGanttChart'
import { getGreeting, formatDate } from '@/lib/utils'
import { useAuth } from '@/lib/auth'
import { useDashboard } from './hooks'
import {
  WeeklyStats,
  WorkItemsStatus,
  ProjectDistribution,
  RecentActivities,
  QuotaCard,
} from './components'

export function Dashboard() {
  const { isAuthenticated } = useAuth()
  const dashboardState = useDashboard(isAuthenticated)

  if (dashboardState.loading) {
    return (
      <div className="flex items-center justify-center h-96">
        <div className="w-6 h-6 border border-border border-t-foreground/60 rounded-full animate-spin" />
      </div>
    )
  }

  return (
    <div className="space-y-12">
      {/* Header - Editorial style */}
      <header className="animate-fade-up opacity-0 delay-1">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
              {`${formatDate(dashboardState.weekRange.start)} — ${formatDate(dashboardState.weekRange.end)}`}
            </p>
            <h1 className="font-display text-4xl text-foreground tracking-tight">
              {getGreeting()}
            </h1>
          </div>
        </div>
      </header>

      {/* Weekly Stats */}
      <WeeklyStats
        stats={dashboardState.stats}
        projectCount={dashboardState.projectCount}
        daysWorked={dashboardState.daysWorked}
      />

      {/* Quota Card */}
      <section className="animate-fade-up opacity-0 delay-2">
        <QuotaCard />
      </section>

      {/* Contribution Heatmap */}
      {dashboardState.heatmapStats?.daily_hours && dashboardState.heatmapStats.daily_hours.length > 0 && (
        <section className="animate-fade-up opacity-0 delay-3">
          <div className="flex items-center gap-2 mb-6">
            <TrendingUp className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
            <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
              工作熱力圖
            </h2>
          </div>
          <Card>
            <CardContent className="p-6">
              <ContributionHeatmap data={dashboardState.heatmapStats.daily_hours} weeks={53} />
            </CardContent>
          </Card>
        </section>
      )}

      {/* Daily Work Gantt Chart */}
      <section className="animate-fade-up opacity-0 delay-3">
        <div className="flex items-center justify-between mb-6">
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
                  {dashboardState.ganttSources.length === 2
                    ? '全部來源'
                    : dashboardState.ganttSources.includes('claude_code')
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
                      id="source-claude_code"
                      checked={dashboardState.ganttSources.includes('claude_code')}
                      disabled={dashboardState.ganttSources.includes('claude_code') && dashboardState.ganttSources.length === 1}
                      onCheckedChange={(checked) => {
                        if (checked) {
                          dashboardState.setGanttSources([...dashboardState.ganttSources, 'claude_code'])
                        } else if (dashboardState.ganttSources.length > 1) {
                          dashboardState.setGanttSources(dashboardState.ganttSources.filter(s => s !== 'claude_code'))
                        }
                      }}
                    />
                    <Label htmlFor="source-claude_code" className="flex items-center gap-2 text-sm font-normal cursor-pointer">
                      <Bot className="h-3.5 w-3.5 text-muted-foreground" />
                      Claude Code
                    </Label>
                  </div>
                  <div className="flex items-center space-x-2">
                    <Checkbox
                      id="source-antigravity"
                      checked={dashboardState.ganttSources.includes('antigravity')}
                      disabled={dashboardState.ganttSources.includes('antigravity') && dashboardState.ganttSources.length === 1}
                      onCheckedChange={(checked) => {
                        if (checked) {
                          dashboardState.setGanttSources([...dashboardState.ganttSources, 'antigravity'])
                        } else if (dashboardState.ganttSources.length > 1) {
                          dashboardState.setGanttSources(dashboardState.ganttSources.filter(s => s !== 'antigravity'))
                        }
                      }}
                    />
                    <Label htmlFor="source-antigravity" className="flex items-center gap-2 text-sm font-normal cursor-pointer">
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
            {dashboardState.ganttLoading ? (
              <div className="flex items-center justify-center h-32">
                <div className="w-5 h-5 border border-border border-t-foreground/60 rounded-full animate-spin" />
              </div>
            ) : (
              <WorkGanttChart
                sessions={dashboardState.ganttSessions}
                date={dashboardState.ganttDate}
                onDateChange={dashboardState.setGanttDate}
              />
            )}
          </CardContent>
        </Card>
      </section>

      {/* Work Items Stats */}
      {dashboardState.stats && dashboardState.stats.total_items > 0 && (
        <WorkItemsStatus stats={dashboardState.stats} />
      )}

      {/* Main Content - Editorial grid */}
      <section className="grid grid-cols-5 gap-12 animate-fade-up opacity-0 delay-3">
        <ProjectDistribution chartData={dashboardState.chartData} />
        <RecentActivities activities={dashboardState.recentActivities} />
      </section>

      {dashboardState.error && (
        <div className="p-4 border-l-2 border-l-destructive bg-destructive/5 text-destructive text-sm">
          {dashboardState.error}
        </div>
      )}
    </div>
  )
}
