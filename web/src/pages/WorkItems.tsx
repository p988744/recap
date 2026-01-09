import { Fragment, useEffect, useState, useMemo } from 'react'
import {
  Briefcase,
  Plus,
  Search,
  Filter,
  Link2,
  CheckCircle2,
  XCircle,
  Edit2,
  Trash2,
  ExternalLink,
  ChevronLeft,
  ChevronRight,
  ChevronDown,
  ChevronUp,
  Layers,
  Loader2,
  Calendar,
} from 'lucide-react'
import { Card, CardContent } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { Badge } from '@/components/ui/badge'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip'
import { api, WorkItem, WorkItemFilters, WorkItemStats, GroupedWorkItemsResponse } from '@/lib/api'
import { cn } from '@/lib/utils'
import { ViewModeSwitcher, ViewMode } from '@/components/ViewModeSwitcher'
import { ProjectSummaryCard, ProjectGroup } from '@/components/ProjectSummaryCard'
import { WorkGanttChart, TimelineSession } from '@/components/WorkGanttChart'

export function WorkItemsPage() {
  const [items, setItems] = useState<WorkItem[]>([])
  const [stats, setStats] = useState<WorkItemStats | null>(null)
  const [loading, setLoading] = useState(true)
  const [page, setPage] = useState(1)
  const [totalPages, setTotalPages] = useState(1)
  const [total, setTotal] = useState(0)
  const [filters, setFilters] = useState<WorkItemFilters>({
    per_page: 20,
  })

  // View mode state
  const [viewMode, setViewMode] = useState<ViewMode>('list')
  const [groupedData, setGroupedData] = useState<GroupedWorkItemsResponse | null>(null)
  const [timelineDate, setTimelineDate] = useState(() => new Date().toISOString().split('T')[0])
  const [timelineSessions, setTimelineSessions] = useState<TimelineSession[]>([])
  const [timelineLoading, setTimelineLoading] = useState(false)
  const [searchTerm, setSearchTerm] = useState('')
  const [showFilters, setShowFilters] = useState(false)
  const [selectedItem, setSelectedItem] = useState<WorkItem | null>(null)
  const [showCreateModal, setShowCreateModal] = useState(false)
  const [showEditModal, setShowEditModal] = useState(false)
  const [showJiraModal, setShowJiraModal] = useState(false)
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false)
  const [itemToDelete, setItemToDelete] = useState<WorkItem | null>(null)
  const [aggregating, setAggregating] = useState(false)
  const [aggregateResult, setAggregateResult] = useState<string | null>(null)
  const [expandedItems, setExpandedItems] = useState<Set<string>>(new Set())
  const [childrenData, setChildrenData] = useState<Record<string, WorkItem[]>>({})
  const [loadingChildren, setLoadingChildren] = useState<Set<string>>(new Set())

  // Form state for create/edit
  const [formData, setFormData] = useState({
    title: '',
    description: '',
    hours: 0,
    date: new Date().toISOString().split('T')[0],
    jira_issue_key: '',
    category: '',
  })

  // Jira mapping state
  const [jiraKey, setJiraKey] = useState('')
  const [jiraTitle, setJiraTitle] = useState('')

  useEffect(() => {
    if (viewMode === 'list') {
      fetchWorkItems()
    } else if (viewMode === 'project' || viewMode === 'task') {
      fetchGroupedData()
    }
    fetchStats()
  }, [page, filters, viewMode])

  // Fetch timeline data when in timeline view
  useEffect(() => {
    if (viewMode === 'timeline') {
      fetchTimelineData()
    }
  }, [viewMode, timelineDate])

  async function fetchGroupedData() {
    setLoading(true)
    try {
      const response = await api.getGroupedWorkItems({
        start_date: filters.start_date,
        end_date: filters.end_date,
      })
      setGroupedData(response)
    } catch (err) {
      console.error('Failed to fetch grouped data:', err)
    } finally {
      setLoading(false)
    }
  }

  async function fetchTimelineData() {
    setTimelineLoading(true)
    try {
      const response = await api.getTimeline(timelineDate)
      const sessions: TimelineSession[] = response.sessions.map(s => ({
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
      setTimelineSessions(sessions)
    } catch (err) {
      console.error('Failed to fetch timeline:', err)
      setTimelineSessions([])
    } finally {
      setTimelineLoading(false)
    }
  }

  async function fetchWorkItems() {
    setLoading(true)
    try {
      const response = await api.getWorkItems({ ...filters, page })
      setItems(response.items)
      setTotalPages(response.pages)
      setTotal(response.total)
    } catch (err) {
      console.error('Failed to fetch work items:', err)
    } finally {
      setLoading(false)
    }
  }

  async function fetchStats() {
    try {
      const response = await api.getWorkItemStats()
      setStats(response)
    } catch (err) {
      console.error('Failed to fetch stats:', err)
    }
  }

  async function handleSearch(e: React.FormEvent) {
    e.preventDefault()
    setPage(1)
    setFilters({ ...filters, search: searchTerm || undefined })
  }

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault()
    try {
      await api.createWorkItem({
        title: formData.title,
        description: formData.description || undefined,
        hours: formData.hours,
        date: formData.date,
        jira_issue_key: formData.jira_issue_key || undefined,
        category: formData.category || undefined,
      })
      setShowCreateModal(false)
      resetForm()
      fetchWorkItems()
      fetchStats()
    } catch (err) {
      console.error('Failed to create work item:', err)
    }
  }

  async function handleUpdate(e: React.FormEvent) {
    e.preventDefault()
    if (!selectedItem) return
    try {
      await api.updateWorkItem(selectedItem.id, {
        title: formData.title,
        description: formData.description || undefined,
        hours: formData.hours,
        date: formData.date,
        jira_issue_key: formData.jira_issue_key || undefined,
        category: formData.category || undefined,
      })
      setShowEditModal(false)
      setSelectedItem(null)
      resetForm()
      fetchWorkItems()
    } catch (err) {
      console.error('Failed to update work item:', err)
    }
  }

  async function handleDelete() {
    if (!itemToDelete) return
    try {
      await api.deleteWorkItem(itemToDelete.id)
      setShowDeleteConfirm(false)
      setItemToDelete(null)
      fetchWorkItems()
      fetchStats()
    } catch (err) {
      console.error('Failed to delete work item:', err)
    }
  }

  function confirmDelete(item: WorkItem) {
    setItemToDelete(item)
    setShowDeleteConfirm(true)
  }

  async function handleMapJira(e: React.FormEvent) {
    e.preventDefault()
    if (!selectedItem) return
    try {
      await api.mapWorkItemJira(selectedItem.id, jiraKey, jiraTitle || undefined)
      setShowJiraModal(false)
      setSelectedItem(null)
      setJiraKey('')
      setJiraTitle('')
      fetchWorkItems()
      fetchStats()
    } catch (err) {
      console.error('Failed to map Jira issue:', err)
    }
  }

  function openEditModal(item: WorkItem) {
    setSelectedItem(item)
    setFormData({
      title: item.title,
      description: item.description || '',
      hours: item.hours,
      date: item.date,
      jira_issue_key: item.jira_issue_key || '',
      category: item.category || '',
    })
    setShowEditModal(true)
  }

  function openJiraModal(item: WorkItem) {
    setSelectedItem(item)
    setJiraKey(item.jira_issue_key || '')
    setJiraTitle(item.jira_issue_title || '')
    setShowJiraModal(true)
  }

  function resetForm() {
    setFormData({
      title: '',
      description: '',
      hours: 0,
      date: new Date().toISOString().split('T')[0],
      jira_issue_key: '',
      category: '',
    })
  }

  async function handleAggregate() {
    setAggregating(true)
    setAggregateResult(null)
    try {
      const result = await api.aggregateWorkItems({
        source: filters.source,
      })
      setAggregateResult(
        `已彙整 ${result.original_count} 個項目為 ${result.aggregated_count} 組，包含 ${result.deleted_count} 個子項目`
      )
      setPage(1)
      setExpandedItems(new Set())
      setChildrenData({})
      fetchWorkItems()
      fetchStats()
    } catch (err) {
      console.error('Failed to aggregate work items:', err)
      setAggregateResult('彙整失敗，請稍後再試')
    } finally {
      setAggregating(false)
    }
  }

  async function toggleExpand(itemId: string) {
    const newExpanded = new Set(expandedItems)

    if (newExpanded.has(itemId)) {
      // Collapse
      newExpanded.delete(itemId)
      setExpandedItems(newExpanded)
    } else {
      // Expand - fetch children if not already loaded
      newExpanded.add(itemId)
      setExpandedItems(newExpanded)

      if (!childrenData[itemId]) {
        setLoadingChildren(prev => new Set(prev).add(itemId))
        try {
          const response = await api.getWorkItems({ parent_id: itemId, per_page: 100 })
          setChildrenData(prev => ({ ...prev, [itemId]: response.items }))
        } catch (err) {
          console.error('Failed to fetch children:', err)
        } finally {
          setLoadingChildren(prev => {
            const newSet = new Set(prev)
            newSet.delete(itemId)
            return newSet
          })
        }
      }
    }
  }

  const sourceLabels: Record<string, string> = {
    claude_code: 'Claude Code',
    gitlab: 'GitLab',
    manual: '手動',
  }

  // Convert grouped data to ProjectGroup format for ProjectSummaryCard
  const projectGroups: ProjectGroup[] = useMemo(() => {
    if (!groupedData?.by_project) return []
    return groupedData.by_project.map(p => ({
      project_name: p.project_name,
      total_hours: p.total_hours,
      issues: p.issues.map(i => ({
        jira_key: i.jira_key || undefined,
        jira_title: i.jira_title || undefined,
        total_hours: i.total_hours,
        logs: i.logs.map(l => ({
          id: l.id,
          title: l.title,
          description: l.description,
          hours: l.hours,
          date: l.date,
          source: l.source,
          synced_to_tempo: l.synced_to_tempo,
        })),
      })),
    }))
  }, [groupedData])

  // Group by Jira task for task view
  const taskGroups = useMemo(() => {
    if (!groupedData?.by_project) return []
    const taskMap = new Map<string, ProjectGroup['issues'][0]>()

    groupedData.by_project.forEach(project => {
      project.issues.forEach(issue => {
        const key = issue.jira_key || 'unmapped'
        const existing = taskMap.get(key)
        if (existing) {
          existing.total_hours += issue.total_hours
          existing.logs.push(...issue.logs)
        } else {
          taskMap.set(key, {
            jira_key: issue.jira_key || undefined,
            jira_title: issue.jira_title || undefined,
            total_hours: issue.total_hours,
            logs: [...issue.logs],
          })
        }
      })
    })

    return Array.from(taskMap.values()).sort((a, b) => b.total_hours - a.total_hours)
  }, [groupedData])

  return (
    <TooltipProvider>
      <div className="space-y-12">
        {/* Header */}
        <header className="animate-fade-up opacity-0 delay-1">
          <div className="flex items-start justify-between mb-6">
            <div>
              <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
                管理
              </p>
              <h1 className="font-display text-4xl text-foreground tracking-tight">工作項目</h1>
            </div>
            <Button onClick={() => setShowCreateModal(true)}>
              <Plus className="w-4 h-4 mr-2" strokeWidth={1.5} />
              新增項目
            </Button>
          </div>
          <ViewModeSwitcher value={viewMode} onChange={setViewMode} />
        </header>

        {/* Stats Cards */}
        {stats && (
          <section className="grid grid-cols-4 gap-4 animate-fade-up opacity-0 delay-2">
            <Card className="border-l-2 border-l-warm/60">
              <CardContent className="p-5">
                <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-1">總項目數</p>
                <p className="font-display text-3xl text-foreground">{stats.total_items}</p>
              </CardContent>
            </Card>
            <Card className="border-l-2 border-l-warm/60">
              <CardContent className="p-5">
                <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-1">總工時</p>
                <p className="font-display text-3xl text-foreground">
                  {stats.total_hours.toFixed(1)}
                  <span className="text-base text-muted-foreground ml-1">hrs</span>
                </p>
              </CardContent>
            </Card>
            <Card className="border-l-2 border-l-sage/60">
              <CardContent className="p-5">
                <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-1">已對應 Jira</p>
                <p className="font-display text-3xl text-foreground">
                  {stats.jira_mapping.percentage.toFixed(0)}%
                  <span className="text-sm text-muted-foreground ml-1">({stats.jira_mapping.mapped})</span>
                </p>
              </CardContent>
            </Card>
            <Card className="border-l-2 border-l-sage/60">
              <CardContent className="p-5">
                <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-1">已同步 Tempo</p>
                <p className="font-display text-3xl text-foreground">
                  {stats.tempo_sync.percentage.toFixed(0)}%
                  <span className="text-sm text-muted-foreground ml-1">({stats.tempo_sync.synced})</span>
                </p>
              </CardContent>
            </Card>
          </section>
        )}

        {/* Search and Filters */}
        <section className="flex items-center gap-4 animate-fade-up opacity-0 delay-3">
          <form onSubmit={handleSearch} className="flex-1 flex gap-2">
            <div className="relative flex-1">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
              <Input
                placeholder="搜尋工作項目..."
                value={searchTerm}
                onChange={(e) => setSearchTerm(e.target.value)}
                className="pl-10"
              />
            </div>
            <Button type="submit" variant="outline">
              搜尋
            </Button>
          </form>
          <Button
            variant="ghost"
            onClick={() => setShowFilters(!showFilters)}
            className={showFilters ? 'bg-accent/10' : ''}
          >
            <Filter className="w-4 h-4 mr-2" strokeWidth={1.5} />
            篩選
          </Button>
        </section>

        {/* Filter Panel */}
        {showFilters && (
          <section className="animate-fade-up">
            <Card>
              <CardContent className="p-6">
                <div className="grid grid-cols-4 gap-4">
                  <div className="space-y-2">
                    <Label className="text-xs">來源</Label>
                    <Select
                      value={filters.source || 'all'}
                      onValueChange={(value) => {
                        setPage(1)
                        setFilters({ ...filters, source: value === 'all' ? undefined : value })
                      }}
                    >
                      <SelectTrigger>
                        <SelectValue placeholder="全部" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="all">全部</SelectItem>
                        <SelectItem value="claude_code">Claude Code</SelectItem>
                        <SelectItem value="gitlab">GitLab</SelectItem>
                        <SelectItem value="manual">手動</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                  <div className="space-y-2">
                    <Label className="text-xs">Jira 對應</Label>
                    <Select
                      value={filters.jira_mapped === undefined ? 'all' : filters.jira_mapped ? 'true' : 'false'}
                      onValueChange={(value) => {
                        setPage(1)
                        setFilters({
                          ...filters,
                          jira_mapped: value === 'all' ? undefined : value === 'true',
                        })
                      }}
                    >
                      <SelectTrigger>
                        <SelectValue placeholder="全部" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="all">全部</SelectItem>
                        <SelectItem value="true">已對應</SelectItem>
                        <SelectItem value="false">未對應</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                  <div className="space-y-2">
                    <Label className="text-xs">Tempo 同步</Label>
                    <Select
                      value={filters.synced_to_tempo === undefined ? 'all' : filters.synced_to_tempo ? 'true' : 'false'}
                      onValueChange={(value) => {
                        setPage(1)
                        setFilters({
                          ...filters,
                          synced_to_tempo: value === 'all' ? undefined : value === 'true',
                        })
                      }}
                    >
                      <SelectTrigger>
                        <SelectValue placeholder="全部" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="all">全部</SelectItem>
                        <SelectItem value="true">已同步</SelectItem>
                        <SelectItem value="false">未同步</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                  <div className="flex items-end">
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => {
                        setPage(1)
                        setFilters({ per_page: 20 })
                        setSearchTerm('')
                      }}
                    >
                      清除篩選
                    </Button>
                  </div>
                </div>
              </CardContent>
            </Card>
          </section>
        )}

        {/* Work Items Content - Conditional by View Mode */}
        <section className="animate-fade-up opacity-0 delay-4">
          {viewMode === 'list' && (
            <>
              <div className="flex items-center justify-between mb-6">
                <div className="flex items-center gap-2">
                  <Briefcase className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
                  <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
                    工作項目列表
                  </p>
                  <Badge variant="secondary" className="ml-2">{total} 筆</Badge>
                </div>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={handleAggregate}
                      disabled={aggregating || items.length === 0}
                    >
                      {aggregating ? (
                        <Loader2 className="w-4 h-4 mr-2 animate-spin" strokeWidth={1.5} />
                      ) : (
                        <Layers className="w-4 h-4 mr-2" strokeWidth={1.5} />
                      )}
                      彙整項目
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>
                    每天每專案合併為一筆，方便上傳 Tempo
                  </TooltipContent>
                </Tooltip>
              </div>

              {/* Aggregate Result */}
              {aggregateResult && (
                <div className="mb-4 p-3 bg-muted/50 border border-border rounded-lg flex items-center justify-between">
                  <span className="text-sm text-foreground">{aggregateResult}</span>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => setAggregateResult(null)}
                  >
                    關閉
                  </Button>
                </div>
              )}
            </>
          )}

          {viewMode === 'project' && (
            <div className="flex items-center justify-between mb-6">
              <div className="flex items-center gap-2">
                <Briefcase className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
                <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
                  依專案分組
                </p>
                <Badge variant="secondary" className="ml-2">{projectGroups.length} 專案</Badge>
              </div>
            </div>
          )}

          {viewMode === 'task' && (
            <div className="flex items-center justify-between mb-6">
              <div className="flex items-center gap-2">
                <Link2 className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
                <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
                  依 Jira 任務分組
                </p>
                <Badge variant="secondary" className="ml-2">{taskGroups.length} 任務</Badge>
              </div>
            </div>
          )}

          {viewMode === 'timeline' && (
            <div className="flex items-center justify-between mb-6">
              <div className="flex items-center gap-2">
                <Calendar className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
                <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
                  時間軸檢視
                </p>
              </div>
            </div>
          )}

          {/* Loading State */}
          {(loading || (viewMode === 'timeline' && timelineLoading)) ? (
            <div className="flex items-center justify-center h-48">
              <div className="w-6 h-6 border border-border border-t-foreground/60 rounded-full animate-spin" />
            </div>
          ) : viewMode === 'list' && items.length === 0 ? (
            <Card>
              <CardContent className="p-16">
                <div className="text-center">
                  <Briefcase className="w-12 h-12 mx-auto mb-4 text-muted-foreground/30" strokeWidth={1} />
                  <h3 className="font-display text-xl text-foreground mb-2">尚無工作項目</h3>
                  <p className="text-sm text-muted-foreground mb-6">
                    從 Claude Code 或 GitLab 同步工作紀錄，或手動新增項目
                  </p>
                  <Button variant="outline" onClick={() => setShowCreateModal(true)}>
                    <Plus className="w-4 h-4 mr-2" strokeWidth={1.5} />
                    新增第一個項目
                  </Button>
                </div>
              </CardContent>
            </Card>
          ) : viewMode === 'project' ? (
            <div className="space-y-4">
              {projectGroups.length === 0 ? (
                <Card>
                  <CardContent className="p-16">
                    <div className="text-center">
                      <Briefcase className="w-12 h-12 mx-auto mb-4 text-muted-foreground/30" strokeWidth={1} />
                      <h3 className="font-display text-xl text-foreground mb-2">尚無專案資料</h3>
                      <p className="text-sm text-muted-foreground">
                        同步工作紀錄後即可檢視專案分組
                      </p>
                    </div>
                  </CardContent>
                </Card>
              ) : (
                projectGroups.map((project) => (
                  <ProjectSummaryCard
                    key={project.project_name}
                    project={project}
                    onItemClick={(item) => {
                      const workItem = items.find(i => i.id === item.id) ||
                        { id: item.id, title: item.title, description: item.description, hours: item.hours, date: item.date, source: item.source, synced_to_tempo: item.synced_to_tempo } as WorkItem
                      openEditModal(workItem)
                    }}
                  />
                ))
              )}
            </div>
          ) : viewMode === 'task' ? (
            <div className="space-y-3">
              {taskGroups.length === 0 ? (
                <Card>
                  <CardContent className="p-16">
                    <div className="text-center">
                      <Link2 className="w-12 h-12 mx-auto mb-4 text-muted-foreground/30" strokeWidth={1} />
                      <h3 className="font-display text-xl text-foreground mb-2">尚無任務資料</h3>
                      <p className="text-sm text-muted-foreground">
                        對應 Jira 後即可檢視任務分組
                      </p>
                    </div>
                  </CardContent>
                </Card>
              ) : (
                taskGroups.map((task, idx) => (
                  <Card key={task.jira_key || `unmapped-${idx}`} className="overflow-hidden">
                    <CardContent className="p-4">
                      <div className="flex items-center justify-between mb-3">
                        <div className="flex items-center gap-2">
                          {task.jira_key ? (
                            <>
                              <Link2 className="w-4 h-4 text-accent" strokeWidth={1.5} />
                              <span className="font-medium text-accent">{task.jira_key}</span>
                              {task.jira_title && (
                                <span className="text-sm text-muted-foreground truncate max-w-[300px]">
                                  {task.jira_title}
                                </span>
                              )}
                            </>
                          ) : (
                            <span className="text-muted-foreground">未對應 Jira</span>
                          )}
                        </div>
                        <div className="flex items-center gap-3">
                          <span className="font-medium tabular-nums">
                            {task.total_hours.toFixed(1)}h
                          </span>
                          <Badge variant="secondary" className="text-xs">
                            {task.logs.length} 筆
                          </Badge>
                        </div>
                      </div>
                      <div className="space-y-1">
                        {task.logs.slice(0, 5).map((log) => (
                          <div
                            key={log.id}
                            className="flex items-center justify-between py-1.5 px-2 rounded hover:bg-muted/30 cursor-pointer text-sm"
                            onClick={() => {
                              const workItem = { id: log.id, title: log.title, description: log.description, hours: log.hours, date: log.date, source: log.source, synced_to_tempo: log.synced_to_tempo } as WorkItem
                              openEditModal(workItem)
                            }}
                          >
                            <div className="flex-1 min-w-0">
                              <p className="truncate text-foreground">{log.title}</p>
                              <p className="text-xs text-muted-foreground">{log.date} • {log.source}</p>
                            </div>
                            <div className="flex items-center gap-2 ml-3">
                              <span className="text-xs tabular-nums text-muted-foreground">{log.hours.toFixed(1)}h</span>
                              {log.synced_to_tempo ? (
                                <CheckCircle2 className="w-3 h-3 text-sage" strokeWidth={1.5} />
                              ) : (
                                <XCircle className="w-3 h-3 text-muted-foreground/30" strokeWidth={1.5} />
                              )}
                            </div>
                          </div>
                        ))}
                        {task.logs.length > 5 && (
                          <p className="text-xs text-muted-foreground text-center py-2">
                            +{task.logs.length - 5} 筆紀錄
                          </p>
                        )}
                      </div>
                    </CardContent>
                  </Card>
                ))
              )}
            </div>
          ) : viewMode === 'timeline' ? (
            <Card>
              <CardContent className="p-6">
                <WorkGanttChart
                  sessions={timelineSessions}
                  date={timelineDate}
                  onDateChange={setTimelineDate}
                />
              </CardContent>
            </Card>
          ) : (
            <Card className="overflow-hidden">
              <Table>
                <TableHeader>
                  <TableRow className="hover:bg-transparent">
                    <TableHead className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground font-medium">
                      項目
                    </TableHead>
                    <TableHead className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground font-medium">
                      來源
                    </TableHead>
                    <TableHead className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground font-medium">
                      日期
                    </TableHead>
                    <TableHead className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground font-medium text-right">
                      工時
                    </TableHead>
                    <TableHead className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground font-medium">
                      Jira
                    </TableHead>
                    <TableHead className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground font-medium text-center">
                      狀態
                    </TableHead>
                    <TableHead className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground font-medium text-right">
                      操作
                    </TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {items.map((item, index) => (
                    <Fragment key={item.id}>
                      <TableRow
                        className={cn(
                          "animate-fade-up opacity-0",
                          `delay-${Math.min(index + 5, 6)}`,
                          item.child_count > 0 && "bg-muted/30"
                        )}
                      >
                        <TableCell className="max-w-xs">
                          <div className="flex items-start gap-2">
                            {item.child_count > 0 && (
                              <Button
                                variant="ghost"
                                size="icon"
                                className="h-6 w-6 shrink-0 mt-0.5"
                                onClick={() => toggleExpand(item.id)}
                              >
                                {loadingChildren.has(item.id) ? (
                                  <Loader2 className="w-4 h-4 animate-spin" strokeWidth={1.5} />
                                ) : expandedItems.has(item.id) ? (
                                  <ChevronUp className="w-4 h-4" strokeWidth={1.5} />
                                ) : (
                                  <ChevronDown className="w-4 h-4" strokeWidth={1.5} />
                                )}
                              </Button>
                            )}
                            <div className="min-w-0">
                              <p className="text-sm text-foreground truncate">{item.title}</p>
                              {item.child_count > 0 ? (
                                <p className="text-xs text-muted-foreground mt-0.5">
                                  {item.child_count} 個子項目
                                </p>
                              ) : item.description && (
                                <p className="text-xs text-muted-foreground truncate mt-0.5">{item.description}</p>
                              )}
                            </div>
                          </div>
                        </TableCell>
                        <TableCell>
                          <Badge variant="outline" className="font-normal">
                            {sourceLabels[item.source] || item.source}
                          </Badge>
                        </TableCell>
                        <TableCell>
                          <span className="text-sm text-muted-foreground tabular-nums">{item.date}</span>
                        </TableCell>
                        <TableCell className="text-right">
                          <span className="text-sm text-foreground tabular-nums">{item.hours.toFixed(1)}</span>
                          <span className="text-xs text-muted-foreground ml-0.5">h</span>
                        </TableCell>
                        <TableCell>
                          {item.jira_issue_key ? (
                            <button
                              className="text-sm text-accent hover:underline flex items-center gap-1"
                              onClick={() => openJiraModal(item)}
                            >
                              {item.jira_issue_key}
                              <ExternalLink className="w-3 h-3" strokeWidth={1.5} />
                            </button>
                          ) : (
                            <button
                              onClick={() => openJiraModal(item)}
                              className="text-xs text-muted-foreground hover:text-foreground flex items-center gap-1"
                            >
                              <Link2 className="w-3 h-3" strokeWidth={1.5} />
                              對應
                            </button>
                          )}
                        </TableCell>
                        <TableCell className="text-center">
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <span>
                                {item.synced_to_tempo ? (
                                  <CheckCircle2 className="w-4 h-4 text-sage mx-auto" strokeWidth={1.5} />
                                ) : (
                                  <XCircle className="w-4 h-4 text-muted-foreground/30 mx-auto" strokeWidth={1.5} />
                                )}
                              </span>
                            </TooltipTrigger>
                            <TooltipContent>
                              {item.synced_to_tempo ? '已同步至 Tempo' : '尚未同步至 Tempo'}
                            </TooltipContent>
                          </Tooltip>
                        </TableCell>
                        <TableCell className="text-right">
                          <div className="flex items-center justify-end gap-1">
                            <Tooltip>
                              <TooltipTrigger asChild>
                                <Button
                                  variant="ghost"
                                  size="icon"
                                  className="h-8 w-8"
                                  onClick={() => openEditModal(item)}
                                >
                                  <Edit2 className="w-4 h-4" strokeWidth={1.5} />
                                </Button>
                              </TooltipTrigger>
                              <TooltipContent>編輯</TooltipContent>
                            </Tooltip>
                            <Tooltip>
                              <TooltipTrigger asChild>
                                <Button
                                  variant="ghost"
                                  size="icon"
                                  className="h-8 w-8 hover:text-destructive"
                                  onClick={() => confirmDelete(item)}
                                >
                                  <Trash2 className="w-4 h-4" strokeWidth={1.5} />
                                </Button>
                              </TooltipTrigger>
                              <TooltipContent>刪除</TooltipContent>
                            </Tooltip>
                          </div>
                        </TableCell>
                      </TableRow>

                      {/* Child rows */}
                      {expandedItems.has(item.id) && childrenData[item.id]?.map((child) => (
                        <TableRow
                          key={child.id}
                          className="bg-muted/10 border-l-2 border-l-warm/40"
                        >
                          <TableCell className="max-w-xs pl-12">
                            <p className="text-sm text-foreground/80 truncate">{child.title}</p>
                            {child.description && (
                              <p className="text-xs text-muted-foreground truncate mt-0.5">{child.description}</p>
                            )}
                          </TableCell>
                          <TableCell>
                            <Badge variant="outline" className="font-normal text-muted-foreground">
                              {sourceLabels[child.source] || child.source}
                            </Badge>
                          </TableCell>
                          <TableCell>
                            <span className="text-sm text-muted-foreground tabular-nums">{child.date}</span>
                          </TableCell>
                          <TableCell className="text-right">
                            <span className="text-sm text-foreground/70 tabular-nums">{child.hours.toFixed(1)}</span>
                            <span className="text-xs text-muted-foreground ml-0.5">h</span>
                          </TableCell>
                          <TableCell>
                            {child.jira_issue_key && (
                              <span className="text-sm text-muted-foreground">{child.jira_issue_key}</span>
                            )}
                          </TableCell>
                          <TableCell className="text-center">
                            {child.synced_to_tempo ? (
                              <CheckCircle2 className="w-4 h-4 text-sage/60 mx-auto" strokeWidth={1.5} />
                            ) : (
                              <XCircle className="w-4 h-4 text-muted-foreground/20 mx-auto" strokeWidth={1.5} />
                            )}
                          </TableCell>
                          <TableCell />
                        </TableRow>
                      ))}
                    </Fragment>
                  ))}
                </TableBody>
              </Table>
            </Card>
          )}

          {/* Pagination - only show in list view */}
          {viewMode === 'list' && totalPages > 1 && (
            <div className="flex items-center justify-center gap-2 mt-6">
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setPage(page - 1)}
                disabled={page === 1}
              >
                <ChevronLeft className="w-4 h-4" strokeWidth={1.5} />
              </Button>
              <span className="text-sm text-muted-foreground tabular-nums">
                {page} / {totalPages}
              </span>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setPage(page + 1)}
                disabled={page === totalPages}
              >
                <ChevronRight className="w-4 h-4" strokeWidth={1.5} />
              </Button>
            </div>
          )}
        </section>

        {/* Create Modal */}
        <Dialog open={showCreateModal} onOpenChange={setShowCreateModal}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle className="font-display text-xl">新增工作項目</DialogTitle>
            </DialogHeader>
            <form onSubmit={handleCreate} className="space-y-4">
              <div className="space-y-2">
                <Label>標題</Label>
                <Input
                  value={formData.title}
                  onChange={(e) => setFormData({ ...formData, title: e.target.value })}
                  required
                />
              </div>
              <div className="space-y-2">
                <Label>描述</Label>
                <Textarea
                  value={formData.description}
                  onChange={(e) => setFormData({ ...formData, description: e.target.value })}
                  rows={3}
                />
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label>日期</Label>
                  <Input
                    type="date"
                    value={formData.date}
                    onChange={(e) => setFormData({ ...formData, date: e.target.value })}
                    required
                  />
                </div>
                <div className="space-y-2">
                  <Label>工時 (小時)</Label>
                  <Input
                    type="number"
                    step="0.5"
                    min="0"
                    value={formData.hours}
                    onChange={(e) => setFormData({ ...formData, hours: parseFloat(e.target.value) || 0 })}
                  />
                </div>
              </div>
              <div className="space-y-2">
                <Label>Jira Issue Key</Label>
                <Input
                  placeholder="ABC-123"
                  value={formData.jira_issue_key}
                  onChange={(e) => setFormData({ ...formData, jira_issue_key: e.target.value })}
                />
              </div>
              <DialogFooter>
                <Button type="button" variant="ghost" onClick={() => {
                  setShowCreateModal(false)
                  resetForm()
                }}>
                  取消
                </Button>
                <Button type="submit">
                  建立
                </Button>
              </DialogFooter>
            </form>
          </DialogContent>
        </Dialog>

        {/* Edit Modal */}
        <Dialog open={showEditModal} onOpenChange={(open) => {
          setShowEditModal(open)
          if (!open) {
            setSelectedItem(null)
            resetForm()
          }
        }}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle className="font-display text-xl">編輯工作項目</DialogTitle>
            </DialogHeader>
            <form onSubmit={handleUpdate} className="space-y-4">
              <div className="space-y-2">
                <Label>標題</Label>
                <Input
                  value={formData.title}
                  onChange={(e) => setFormData({ ...formData, title: e.target.value })}
                  required
                />
              </div>
              <div className="space-y-2">
                <Label>描述</Label>
                <Textarea
                  value={formData.description}
                  onChange={(e) => setFormData({ ...formData, description: e.target.value })}
                  rows={3}
                />
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label>日期</Label>
                  <Input
                    type="date"
                    value={formData.date}
                    onChange={(e) => setFormData({ ...formData, date: e.target.value })}
                    required
                  />
                </div>
                <div className="space-y-2">
                  <Label>工時 (小時)</Label>
                  <Input
                    type="number"
                    step="0.5"
                    min="0"
                    value={formData.hours}
                    onChange={(e) => setFormData({ ...formData, hours: parseFloat(e.target.value) || 0 })}
                  />
                </div>
              </div>
              <div className="space-y-2">
                <Label>Jira Issue Key</Label>
                <Input
                  placeholder="ABC-123"
                  value={formData.jira_issue_key}
                  onChange={(e) => setFormData({ ...formData, jira_issue_key: e.target.value })}
                />
              </div>
              <DialogFooter>
                <Button type="button" variant="ghost" onClick={() => {
                  setShowEditModal(false)
                  setSelectedItem(null)
                  resetForm()
                }}>
                  取消
                </Button>
                <Button type="submit">
                  儲存
                </Button>
              </DialogFooter>
            </form>
          </DialogContent>
        </Dialog>

        {/* Jira Mapping Modal */}
        <Dialog open={showJiraModal} onOpenChange={(open) => {
          setShowJiraModal(open)
          if (!open) {
            setSelectedItem(null)
            setJiraKey('')
            setJiraTitle('')
          }
        }}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle className="font-display text-xl">對應 Jira Issue</DialogTitle>
              <DialogDescription className="truncate">
                {selectedItem?.title}
              </DialogDescription>
            </DialogHeader>
            <form onSubmit={handleMapJira} className="space-y-4">
              <div className="space-y-2">
                <Label>Jira Issue Key</Label>
                <Input
                  placeholder="ABC-123"
                  value={jiraKey}
                  onChange={(e) => setJiraKey(e.target.value)}
                  required
                />
              </div>
              <div className="space-y-2">
                <Label>Issue 標題 (選填)</Label>
                <Input
                  placeholder="Issue title"
                  value={jiraTitle}
                  onChange={(e) => setJiraTitle(e.target.value)}
                />
              </div>
              {selectedItem?.jira_issue_suggested && (
                <div className="p-3 bg-muted border-l-2 border-l-accent">
                  <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-1">建議對應</p>
                  <button
                    type="button"
                    onClick={() => setJiraKey(selectedItem.jira_issue_suggested!)}
                    className="text-sm text-accent hover:underline"
                  >
                    {selectedItem.jira_issue_suggested}
                  </button>
                </div>
              )}
              <DialogFooter>
                <Button type="button" variant="ghost" onClick={() => {
                  setShowJiraModal(false)
                  setSelectedItem(null)
                  setJiraKey('')
                  setJiraTitle('')
                }}>
                  取消
                </Button>
                <Button type="submit">
                  <Link2 className="w-4 h-4 mr-2" strokeWidth={1.5} />
                  對應
                </Button>
              </DialogFooter>
            </form>
          </DialogContent>
        </Dialog>

        {/* Delete Confirmation Modal */}
        <Dialog open={showDeleteConfirm} onOpenChange={setShowDeleteConfirm}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle className="font-display text-xl">確認刪除</DialogTitle>
              <DialogDescription>
                確定要刪除工作項目「{itemToDelete?.title}」嗎？此操作無法復原。
              </DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button variant="ghost" onClick={() => {
                setShowDeleteConfirm(false)
                setItemToDelete(null)
              }}>
                取消
              </Button>
              <Button variant="destructive" onClick={handleDelete}>
                <Trash2 className="w-4 h-4 mr-2" strokeWidth={1.5} />
                刪除
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </div>
    </TooltipProvider>
  )
}
