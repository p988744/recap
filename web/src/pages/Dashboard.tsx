import { useEffect, useState, useMemo, useCallback } from 'react'
import { Link } from 'react-router-dom'
import {
  Upload,
  FileText,
  TrendingUp,
  ArrowRight,
  Briefcase,
  Link2,
  CheckCircle2,
  Loader2,
  Check,
  AlertCircle,
  CalendarDays,
  RefreshCw,
} from 'lucide-react'
import { PieChart, Pie, Cell, ResponsiveContainer, Tooltip } from 'recharts'
import { Card, CardContent } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Separator } from '@/components/ui/separator'
import { Progress } from '@/components/ui/progress'
import { ContributionHeatmap } from '@/components/ContributionHeatmap'
import { WorkGanttChart, TimelineSession } from '@/components/WorkGanttChart'
import { api, WorkItemStats, WorkItem, SyncStatus as SyncStatusType } from '@/lib/api'
import { getGreeting, formatDate, getWeekProgress, cn } from '@/lib/utils'

// Muted warm chart colors
const CHART_COLORS = [
  'hsl(35, 25%, 55%)',   // warm
  'hsl(15, 25%, 50%)',   // terracotta muted
  'hsl(90, 15%, 50%)',   // sage
  'hsl(30, 15%, 55%)',   // stone
  'hsl(45, 20%, 60%)',   // warm light
  'hsl(15, 20%, 60%)',   // terracotta light
]

type SyncState = 'idle' | 'syncing' | 'success' | 'error'

// Get this week's start and end dates
function getThisWeekRange() {
  const now = new Date()
  const dayOfWeek = now.getDay()
  const monday = new Date(now)
  monday.setDate(now.getDate() - (dayOfWeek === 0 ? 6 : dayOfWeek - 1))
  monday.setHours(0, 0, 0, 0)

  const sunday = new Date(monday)
  sunday.setDate(monday.getDate() + 6)

  return {
    start: monday.toISOString().split('T')[0],
    end: sunday.toISOString().split('T')[0],
  }
}

// Get date range for heatmap (past N weeks, default 53 weeks = ~1 year like GitHub)
function getHeatmapRange(weeks: number = 53) {
  const now = new Date()
  const end = now.toISOString().split('T')[0]

  const start = new Date(now)
  start.setDate(now.getDate() - weeks * 7)

  return {
    start: start.toISOString().split('T')[0],
    end,
  }
}

export function Dashboard() {
  const [stats, setStats] = useState<WorkItemStats | null>(null)
  const [heatmapStats, setHeatmapStats] = useState<WorkItemStats | null>(null)
  const [recentItems, setRecentItems] = useState<WorkItem[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [syncStatus, setSyncStatus] = useState<SyncState>('idle')
  const [syncMessage, setSyncMessage] = useState<string>('')
  const [weekRange] = useState(getThisWeekRange)
  const [heatmapRange] = useState(() => getHeatmapRange(53))
  const [claudeSyncInfo, setClaudeSyncInfo] = useState<string>('')

  // Gantt chart state
  const [ganttDate, setGanttDate] = useState(() => new Date().toISOString().split('T')[0])
  const [ganttSessions, setGanttSessions] = useState<TimelineSession[]>([])
  const [ganttLoading, setGanttLoading] = useState(false)

  // Auto-sync data sources on load
  const [autoSyncState, setAutoSyncState] = useState<'idle' | 'syncing' | 'done'>('idle')
  const [syncStatusData, setSyncStatusData] = useState<SyncStatusType[]>([])

  useEffect(() => {
    async function autoSyncAllSources() {
      if (autoSyncState !== 'idle') return
      setAutoSyncState('syncing')

      try {
        // Use the new auto-sync API that syncs all available projects
        const result = await api.autoSync()
        if (result.total_items > 0) {
          const totalCreatedUpdated = result.results.reduce((sum, r) => sum + r.items_synced, 0)
          setClaudeSyncInfo(`已同步 ${totalCreatedUpdated} 筆工作項目`)
          // Auto-hide after 4 seconds
          setTimeout(() => setClaudeSyncInfo(''), 4000)
        }

        // Fetch sync status for display
        const statuses = await api.getSyncStatus().catch(() => [])
        setSyncStatusData(statuses)
      } catch {
        // Silent fail for auto-sync
      } finally {
        setAutoSyncState('done')
      }
    }
    autoSyncAllSources()
  }, [autoSyncState])

  useEffect(() => {
    async function fetchData() {
      try {
        const [statsResult, heatmapResult, itemsResult] = await Promise.all([
          api.getWorkItemStats(weekRange.start, weekRange.end).catch(() => null),
          api.getWorkItemStats(heatmapRange.start, heatmapRange.end).catch(() => null),
          api.getWorkItems({
            start_date: weekRange.start,
            end_date: weekRange.end,
            per_page: 20
          }).catch(() => null),
        ])
        setStats(statsResult)
        setHeatmapStats(heatmapResult)
        setRecentItems(itemsResult?.items ?? [])
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load data')
      } finally {
        setLoading(false)
      }
    }
    fetchData()
  }, [weekRange, heatmapRange, claudeSyncInfo])

  // Fetch timeline data for Gantt chart
  useEffect(() => {
    async function fetchTimeline() {
      setGanttLoading(true)
      try {
        const result = await api.getTimeline(ganttDate)
        // Convert API response to component format
        const sessions: TimelineSession[] = result.sessions.map(s => ({
          id: s.id,
          project: s.project,
          title: s.title,
          startTime: s.start_time,
          endTime: s.end_time,
          hours: s.hours,
          commits: s.commits.map(c => ({
            hash: c.hash,
            message: c.message,
            time: c.time,
            author: c.author,
          })),
        }))
        setGanttSessions(sessions)
      } catch {
        // Silent fail for timeline
        setGanttSessions([])
      } finally {
        setGanttLoading(false)
      }
    }
    fetchTimeline()
  }, [ganttDate])

  // Sync to Tempo function
  const handleSyncToTempo = useCallback(async () => {
    setSyncStatus('syncing')
    setSyncMessage('')

    try {
      // 1. Fetch work items that are mapped to Jira but not synced to Tempo
      const response = await api.getWorkItems({
        jira_mapped: true,
        synced_to_tempo: false,
        per_page: 100,
      })

      const itemsToSync = response.items.filter(item => item.jira_issue_key && item.hours > 0)
      if (itemsToSync.length === 0) {
        setSyncStatus('success')
        setSyncMessage('所有項目都已同步')
        setTimeout(() => setSyncStatus('idle'), 3000)
        return
      }

      // 2. Convert work items to Tempo sync format, keeping track of mapping
      const entries = itemsToSync.map(item => ({
        issue_key: item.jira_issue_key!,
        date: item.date.split('T')[0],
        minutes: Math.round(item.hours * 60),
        description: item.title,
      }))

      // 3. Sync to Tempo
      const result = await api.syncWorklogs(entries, false)

      // 4. Update work items with sync status and worklog IDs
      // Match results back to work items by issue_key + date
      if (result.results.length > 0) {
        for (let i = 0; i < itemsToSync.length; i++) {
          const item = itemsToSync[i]
          const syncResult = result.results[i]

          if (syncResult && syncResult.status === 'success') {
            try {
              await api.updateWorkItem(item.id, {
                synced_to_tempo: true,
                tempo_worklog_id: syncResult.id || undefined,
              })
            } catch {
              // Ignore individual update errors
            }
          }
        }
      }

      // 5. Refresh stats
      const newStats = await api.getWorkItemStats(weekRange.start, weekRange.end).catch(() => null)
      if (newStats) setStats(newStats)

      setSyncStatus('success')
      setSyncMessage(`成功同步 ${result.successful}/${result.total_entries} 筆`)
      setTimeout(() => setSyncStatus('idle'), 3000)
    } catch (err) {
      setSyncStatus('error')
      setSyncMessage(err instanceof Error ? err.message : '同步失敗')
      setTimeout(() => setSyncStatus('idle'), 5000)
    }
  }, [weekRange])

  // Chart data from hours_by_project (instead of hours_by_source)
  const chartData = useMemo(() => {
    if (!stats?.hours_by_project) return []
    return Object.entries(stats.hours_by_project)
      .sort((a, b) => b[1] - a[1]) // Sort by hours descending
      .map(([name, hours]) => ({
        name,
        value: hours,
        hours: hours.toFixed(1),
      }))
  }, [stats])

  // Recent activities from work items
  const recentActivities = useMemo(() => {
    return recentItems.slice(0, 5).map(item => ({
      title: item.title,
      source: item.source,
      date: item.date,
      hours: item.hours,
      jiraKey: item.jira_issue_key,
    }))
  }, [recentItems])

  const weekProgress = getWeekProgress()
  const projectCount = Object.keys(stats?.hours_by_project ?? {}).length
  // Calculate unique days from work items
  const daysWorked = useMemo(() => {
    const dates = new Set(recentItems.map(item => item.date.split('T')[0]))
    return dates.size
  }, [recentItems])

  if (loading) {
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
              {`${formatDate(weekRange.start)} — ${formatDate(weekRange.end)}`}
            </p>
            <h1 className="font-display text-4xl text-foreground tracking-tight">
              {getGreeting()}
            </h1>
          </div>
          {/* Sync Status Indicator */}
          <div className="flex items-center gap-2">
            {autoSyncState === 'syncing' ? (
              <div className="flex items-center gap-2 text-xs text-muted-foreground">
                <RefreshCw className="w-3 h-3 animate-spin" strokeWidth={1.5} />
                <span>同步中...</span>
              </div>
            ) : syncStatusData.length > 0 && (
              <div className="flex items-center gap-2 text-xs text-muted-foreground">
                <CheckCircle2 className="w-3 h-3 text-sage" strokeWidth={1.5} />
                <span>
                  {syncStatusData[0]?.last_sync_at
                    ? `上次同步: ${formatDate(syncStatusData[0].last_sync_at)}`
                    : '已同步'}
                </span>
              </div>
            )}
          </div>
        </div>
      </header>

      {/* Stats - Asymmetric layout with generous spacing */}
      <section className="grid grid-cols-3 gap-8 animate-fade-up opacity-0 delay-2">
        {/* Weekly Hours - Featured stat */}
        <Card className="col-span-2 border-l-2 border-l-warm/60">
          <CardContent className="p-8">
            <div className="flex items-end justify-between">
              <div>
                <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-3">
                  本週工時
                </p>
                <p className="font-display text-6xl text-foreground tracking-tight">
                  {(stats?.total_hours ?? 0).toFixed(1)}
                </p>
                <p className="text-sm text-muted-foreground mt-2">小時</p>
              </div>
              <div className="text-right">
                <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-1">
                  目標進度
                </p>
                <p className="text-2xl text-muted-foreground">{weekProgress.toFixed(0)}%</p>
                <div className="w-32 mt-3">
                  <Progress value={Math.min(weekProgress, 100)} className="h-1" />
                </div>
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Secondary stats */}
        <div className="space-y-6">
          <Card className="border-l-2 border-l-sage/40">
            <CardContent className="p-6">
              <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
                專案數
              </p>
              <p className="font-display text-3xl text-foreground">{projectCount}</p>
            </CardContent>
          </Card>
          <Card className="border-l-2 border-l-stone/40">
            <CardContent className="p-6">
              <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
                工作天數
              </p>
              <p className="font-display text-3xl text-foreground">{daysWorked}</p>
            </CardContent>
          </Card>
        </div>
      </section>

      {/* Contribution Heatmap */}
      {heatmapStats?.daily_hours && heatmapStats.daily_hours.length > 0 && (
        <section className="animate-fade-up opacity-0 delay-3">
          <div className="flex items-center gap-2 mb-6">
            <TrendingUp className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
            <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
              工作熱力圖
            </h2>
          </div>
          <Card>
            <CardContent className="p-6">
              <ContributionHeatmap data={heatmapStats.daily_hours} weeks={53} />
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
            {ganttLoading ? (
              <div className="flex items-center justify-center h-32">
                <div className="w-5 h-5 border border-border border-t-foreground/60 rounded-full animate-spin" />
              </div>
            ) : (
              <WorkGanttChart
                sessions={ganttSessions}
                date={ganttDate}
                onDateChange={setGanttDate}
              />
            )}
          </CardContent>
        </Card>
      </section>

      {/* Work Items Stats */}
      {stats && stats.total_items > 0 && (
        <section className="animate-fade-up opacity-0 delay-3">
          <div className="flex items-center justify-between mb-6">
            <div className="flex items-center gap-2">
              <Briefcase className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
              <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
                工作項目狀態
              </h2>
            </div>
            <Link to="/work-items" className="text-xs text-muted-foreground hover:text-foreground transition-colors flex items-center gap-1">
              管理項目
              <ArrowRight className="w-3 h-3" strokeWidth={1.5} />
            </Link>
          </div>
          <div className="grid grid-cols-3 gap-4">
            <Card>
              <CardContent className="p-5">
                <div className="flex items-center gap-3">
                  <Briefcase className="w-5 h-5 text-muted-foreground/50" strokeWidth={1.5} />
                  <div>
                    <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">總項目</p>
                    <p className="font-display text-2xl text-foreground">{stats.total_items}</p>
                  </div>
                </div>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="p-5">
                <div className="flex items-center gap-3">
                  <Link2 className="w-5 h-5 text-muted-foreground/50" strokeWidth={1.5} />
                  <div>
                    <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">已對應 Jira</p>
                    <p className="font-display text-2xl text-foreground">
                      {stats.jira_mapping.percentage.toFixed(0)}%
                      <span className="text-sm text-muted-foreground ml-1">({stats.jira_mapping.mapped})</span>
                    </p>
                  </div>
                </div>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="p-5">
                <div className="flex items-center gap-3">
                  <CheckCircle2 className="w-5 h-5 text-muted-foreground/50" strokeWidth={1.5} />
                  <div>
                    <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">已同步 Tempo</p>
                    <p className="font-display text-2xl text-foreground">
                      {stats.tempo_sync.percentage.toFixed(0)}%
                      <span className="text-sm text-muted-foreground ml-1">({stats.tempo_sync.synced})</span>
                    </p>
                  </div>
                </div>
              </CardContent>
            </Card>
          </div>
        </section>
      )}

      {/* Main Content - Editorial grid */}
      <section className="grid grid-cols-5 gap-12 animate-fade-up opacity-0 delay-3">
        {/* Project Distribution */}
        <div className="col-span-2">
          <div className="flex items-center gap-2 mb-6">
            <TrendingUp className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
            <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
              專案分佈
            </h2>
          </div>

          {chartData.length > 0 ? (
            <>
              <div className="h-44">
                <ResponsiveContainer width="100%" height="100%">
                  <PieChart>
                    <Pie
                      data={chartData}
                      cx="50%"
                      cy="50%"
                      innerRadius={45}
                      outerRadius={70}
                      paddingAngle={2}
                      dataKey="value"
                      stroke="none"
                    >
                      {chartData.map((_, index) => (
                        <Cell
                          key={`cell-${index}`}
                          fill={CHART_COLORS[index % CHART_COLORS.length]}
                        />
                      ))}
                    </Pie>
                    <Tooltip
                      content={({ payload }) => {
                        if (payload && payload[0]) {
                          const data = payload[0].payload
                          return (
                            <div className="bg-popover/90 backdrop-blur px-3 py-2 border border-border text-sm">
                              <p className="text-foreground">{data.name}</p>
                              <p className="text-muted-foreground">{data.hours} 小時</p>
                            </div>
                          )
                        }
                        return null
                      }}
                    />
                  </PieChart>
                </ResponsiveContainer>
              </div>
              <div className="mt-6 space-y-3">
                {chartData.slice(0, 4).map((item, index) => (
                  <div key={item.name} className="flex items-center gap-3">
                    <div
                      className="w-2 h-2"
                      style={{ backgroundColor: CHART_COLORS[index % CHART_COLORS.length] }}
                    />
                    <span className="text-sm text-muted-foreground flex-1 truncate">{item.name}</span>
                    <span className="text-sm text-foreground tabular-nums">{item.hours}h</span>
                  </div>
                ))}
              </div>
            </>
          ) : (
            <div className="h-44 flex items-center justify-center text-muted-foreground text-sm">
              暫無資料
            </div>
          )}
        </div>

        {/* Recent Activities */}
        <div className="col-span-3">
          <div className="flex items-center justify-between mb-6">
            <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
              最近活動
            </h2>
            <Link to="/work-items" className="text-xs text-muted-foreground hover:text-foreground transition-colors flex items-center gap-1">
              查看全部
              <ArrowRight className="w-3 h-3" strokeWidth={1.5} />
            </Link>
          </div>

          {recentActivities.length > 0 ? (
            <div className="space-y-0">
              {recentActivities.map((activity, index) => (
                <div
                  key={`${activity.source}-${activity.date}-${index}`}
                  className={cn(
                    "py-4 border-b border-border last:border-b-0",
                    "animate-fade-up opacity-0",
                    index === 0 && "delay-4",
                    index === 1 && "delay-5",
                    index === 2 && "delay-6"
                  )}
                >
                  <div className="flex items-start justify-between gap-4">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-3 mb-1">
                        <span className="text-sm text-foreground line-clamp-1">{activity.title}</span>
                        <span className="text-xs text-muted-foreground tabular-nums">{activity.hours.toFixed(1)}h</span>
                      </div>
                      <div className="flex items-center gap-2">
                        <span className="text-xs text-muted-foreground">{activity.source}</span>
                        {activity.jiraKey && (
                          <span className="text-xs text-blue-600">{activity.jiraKey}</span>
                        )}
                      </div>
                    </div>
                    <span className="text-xs text-muted-foreground tabular-nums flex-shrink-0">
                      {formatDate(activity.date)}
                    </span>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="h-44 flex items-center justify-center text-muted-foreground text-sm">
              暫無活動記錄
            </div>
          )}
        </div>
      </section>

      {/* Quick Actions - Minimal */}
      <section className="pt-8 animate-fade-up opacity-0 delay-6">
        <Separator className="mb-8" />
        <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-4">
          快捷操作
        </p>
        <div className="flex items-center gap-3">
          <Button
            variant="outline"
            onClick={handleSyncToTempo}
            disabled={syncStatus === 'syncing'}
            className={cn(
              syncStatus === 'success' && 'border-sage text-sage',
              syncStatus === 'error' && 'border-destructive text-destructive'
            )}
          >
            {syncStatus === 'syncing' ? (
              <Loader2 className="w-4 h-4 mr-2 animate-spin" strokeWidth={1.5} />
            ) : syncStatus === 'success' ? (
              <Check className="w-4 h-4 mr-2" strokeWidth={1.5} />
            ) : syncStatus === 'error' ? (
              <AlertCircle className="w-4 h-4 mr-2" strokeWidth={1.5} />
            ) : (
              <Upload className="w-4 h-4 mr-2" strokeWidth={1.5} />
            )}
            {syncStatus === 'syncing' ? '同步中...' : syncMessage || '同步到 Tempo'}
          </Button>
          <Link to="/reports">
            <Button variant="ghost">
              <FileText className="w-4 h-4 mr-2" strokeWidth={1.5} />
              生成週報
            </Button>
          </Link>
          <Link to="/reports?tab=pe">
            <Button variant="ghost">
              <TrendingUp className="w-4 h-4 mr-2" strokeWidth={1.5} />
              績效考核
            </Button>
          </Link>
        </div>
      </section>

      {claudeSyncInfo && (
        <div className="fixed top-4 right-4 p-3 bg-sage/10 border border-sage/30 text-sage text-sm rounded-lg animate-fade-up shadow-sm z-50">
          <div className="flex items-center gap-2">
            <CheckCircle2 className="w-4 h-4" strokeWidth={1.5} />
            {claudeSyncInfo}
          </div>
        </div>
      )}

      {error && (
        <div className="p-4 border-l-2 border-l-destructive bg-destructive/5 text-destructive text-sm">
          {error}
        </div>
      )}
    </div>
  )
}
