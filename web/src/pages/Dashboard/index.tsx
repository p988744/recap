import { TrendingUp, CalendarDays, CheckCircle2, RefreshCw } from 'lucide-react'
import { Card, CardContent } from '@/components/ui/card'
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
  QuickActions,
} from './components'

export function Dashboard() {
  const { token, isAuthenticated } = useAuth()
  const dashboardState = useDashboard(isAuthenticated, token)

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
          {/* Sync Status Indicator */}
          <div className="flex items-center gap-2">
            {dashboardState.autoSyncState === 'syncing' ? (
              <div className="flex items-center gap-2 text-xs text-muted-foreground">
                <RefreshCw className="w-3 h-3 animate-spin" strokeWidth={1.5} />
                <span>同步中...</span>
              </div>
            ) : dashboardState.syncStatusData.length > 0 && (
              <div className="flex items-center gap-2 text-xs text-muted-foreground">
                <CheckCircle2 className="w-3 h-3 text-sage" strokeWidth={1.5} />
                <span>
                  {dashboardState.syncStatusData[0]?.last_sync_at
                    ? `上次同步: ${formatDate(dashboardState.syncStatusData[0].last_sync_at)}`
                    : '已同步'}
                </span>
              </div>
            )}
          </div>
        </div>
      </header>

      {/* Weekly Stats */}
      <WeeklyStats
        stats={dashboardState.stats}
        projectCount={dashboardState.projectCount}
        daysWorked={dashboardState.daysWorked}
      />

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
        <div className="flex items-center gap-2 mb-6">
          <CalendarDays className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
          <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
            每日工作時間軸
          </h2>
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

      {/* Quick Actions */}
      <QuickActions
        syncStatus={dashboardState.syncStatus}
        syncMessage={dashboardState.syncMessage}
        onSyncToTempo={dashboardState.handleSyncToTempo}
      />

      {/* Notifications */}
      {dashboardState.claudeSyncInfo && (
        <div className="fixed top-4 right-4 p-3 bg-sage/10 border border-sage/30 text-sage text-sm rounded-lg animate-fade-up shadow-sm z-50">
          <div className="flex items-center gap-2">
            <CheckCircle2 className="w-4 h-4" strokeWidth={1.5} />
            {dashboardState.claudeSyncInfo}
          </div>
        </div>
      )}

      {dashboardState.error && (
        <div className="p-4 border-l-2 border-l-destructive bg-destructive/5 text-destructive text-sm">
          {dashboardState.error}
        </div>
      )}
    </div>
  )
}
