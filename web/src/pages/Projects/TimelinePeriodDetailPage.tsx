import { useState, useEffect, useMemo, useCallback } from 'react'
import { useParams, useSearchParams, useNavigate } from 'react-router-dom'
import { ArrowLeft, Clock, Calendar, GitCommit, FileCode, ChevronDown, ChevronRight, FileText, Pencil, Trash2 } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useProjectDetail } from './hooks/useProjectDetail'
import { worklog, workItems } from '@/services'
import type { HourlyBreakdownItem } from '@/types/worklog'
import type { WorkItem } from '@/types'
import { MarkdownSummary } from '@/components/MarkdownSummary'
import { ClaudeIcon } from '@/pages/Settings/components/ProjectsSection/icons/ClaudeIcon'
import { CommitDiffModal } from './components/Modals/CommitDiffModal'
import { EditModal, DeleteModal } from '@/pages/WorkItems/components/Modals'
import type { WorkItemFormData } from '@/pages/WorkItems/hooks/useWorkItems'

// Check if a path is a manual project path
function isManualProjectPath(path: string | null): boolean {
  return path ? path.includes('.recap') && path.includes('manual-projects') : false
}

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

// Generate date range array from start to end
function getDateRange(start: string, end: string): string[] {
  const dates: string[] = []
  const startDate = new Date(start + 'T00:00:00')
  const endDate = new Date(end + 'T00:00:00')

  const current = new Date(startDate)
  while (current <= endDate) {
    dates.push(current.toISOString().split('T')[0])
    current.setDate(current.getDate() + 1)
  }

  return dates
}

// Format date for display
function formatDateDisplay(dateStr: string): string {
  const d = new Date(dateStr + 'T00:00:00')
  const weekdays = ['週日', '週一', '週二', '週三', '週四', '週五', '週六']
  return `${weekdays[d.getDay()]} ${d.getMonth() + 1}/${d.getDate()}`
}

// Source configuration
const SOURCE_CONFIG: Record<string, { icon: React.ReactNode; label: string; headerBgClass: string }> = {
  claude_code: {
    icon: <ClaudeIcon className="w-3.5 h-3.5" />,
    label: 'Claude Code',
    headerBgClass: 'bg-amber-50 dark:bg-amber-900/20',
  },
}

// Hourly item with expandable commits
interface HourlyItemProps {
  item: HourlyBreakdownItem
  projectPath: string | null
}

function HourlyItem({ item, projectPath }: HourlyItemProps) {
  const [isExpanded, setIsExpanded] = useState(false)
  const [selectedCommit, setSelectedCommit] = useState<{ hash: string; message: string } | null>(null)

  const fileNames = useMemo(() => {
    const seen = new Set<string>()
    const result: string[] = []
    for (const f of item.files_modified) {
      const name = f.split(/[/\\]/).pop() || f
      if (!seen.has(name)) {
        seen.add(name)
        result.push(name)
      }
    }
    return result
  }, [item.files_modified])

  const hasCommits = item.git_commits.length > 0

  return (
    <>
      <CommitDiffModal
        open={selectedCommit !== null}
        onOpenChange={(open) => !open && setSelectedCommit(null)}
        projectPath={projectPath}
        commitHash={selectedCommit?.hash || ''}
        commitMessage={selectedCommit?.message}
      />
      <div className="px-4 py-3 pl-11">
        {/* Time label */}
        <span className="flex items-center gap-1 text-xs text-muted-foreground mb-1.5">
          <Clock className="w-3 h-3" strokeWidth={1.5} />
          {item.hour_start}–{item.hour_end}
        </span>

        {/* Summary */}
        <MarkdownSummary content={item.summary} />

        {/* Files modified */}
        {fileNames.length > 0 && (
          <div className="mt-2">
            <div className="flex items-center gap-1 mb-1">
              <FileCode className="w-3 h-3 text-muted-foreground" strokeWidth={1.5} />
              <span className="text-[10px] uppercase tracking-wider text-muted-foreground">
                修改檔案
              </span>
            </div>
            <div className="flex flex-wrap gap-1">
              {fileNames.slice(0, 8).map((name, j) => (
                <span
                  key={j}
                  className="text-xs text-muted-foreground bg-muted/50 px-1.5 py-0.5 rounded font-mono"
                >
                  {name}
                </span>
              ))}
              {fileNames.length > 8 && (
                <span className="text-xs text-muted-foreground">
                  +{fileNames.length - 8}
                </span>
              )}
            </div>
          </div>
        )}

        {/* Git commits - expandable */}
        {hasCommits && (
          <div className="mt-2">
            <button
              onClick={() => setIsExpanded(!isExpanded)}
              className="flex items-center gap-1 mb-1 hover:text-foreground transition-colors"
            >
              {isExpanded ? (
                <ChevronDown className="w-3 h-3 text-muted-foreground" strokeWidth={1.5} />
              ) : (
                <ChevronRight className="w-3 h-3 text-muted-foreground" strokeWidth={1.5} />
              )}
              <GitCommit className="w-3 h-3 text-muted-foreground" strokeWidth={1.5} />
              <span className="text-[10px] uppercase tracking-wider text-muted-foreground">
                Commits ({item.git_commits.length})
              </span>
            </button>
            {isExpanded && (
              <div className="space-y-0.5 ml-4">
                {item.git_commits.map((commit, j) => (
                  <div
                    key={j}
                    className="flex items-baseline gap-2 py-0.5 px-1 -mx-1 rounded cursor-pointer hover:bg-muted/50 transition-colors"
                    onClick={() => setSelectedCommit({ hash: commit.hash, message: commit.message })}
                    title="Click to view diff"
                  >
                    <span className="text-xs font-mono text-muted-foreground shrink-0 bg-muted/50 px-1.5 py-0.5 rounded">
                      {commit.hash.slice(0, 7)}
                    </span>
                    <span className="text-xs text-foreground truncate">{commit.message}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </>
  )
}

// Source section with header
interface SourceSectionProps {
  source: string
  items: HourlyBreakdownItem[]
  projectPath: string | null
}

function SourceSection({ source, items, projectPath }: SourceSectionProps) {
  const config = SOURCE_CONFIG[source]
  if (!config) return null

  return (
    <div>
      {/* Source header */}
      <div className={`flex items-center gap-1.5 px-4 py-2 pl-11 ${config.headerBgClass}`}>
        {config.icon}
        <span className="text-xs font-medium text-foreground/80">{config.label}</span>
        <span className="text-xs text-muted-foreground">({items.length})</span>
      </div>
      {/* Items */}
      <div className="divide-y divide-border/50">
        {items.map((item, i) => (
          <HourlyItem key={i} item={item} projectPath={projectPath} />
        ))}
      </div>
    </div>
  )
}

// Day section with date header
interface DaySectionProps {
  date: string
  items: HourlyBreakdownItem[]
  projectPath: string | null
}

function DaySection({ date, items, projectPath }: DaySectionProps) {
  const claudeItems = items.filter(item => item.source === 'claude_code')
  const totalHours = items.length // Each item represents approximately 1 hour
  const totalCommits = items.reduce((sum, item) => sum + item.git_commits.length, 0)

  return (
    <div className="border border-border rounded-lg bg-white/60 dark:bg-white/5 overflow-hidden">
      {/* Date header */}
      <div className="px-4 py-3 bg-muted/30 border-b border-border">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <span className="text-sm font-medium text-foreground">
              {formatDateDisplay(date)}
            </span>
          </div>
          <div className="flex items-center gap-3 text-xs text-muted-foreground">
            <span className="flex items-center gap-1">
              <Clock className="w-3 h-3" strokeWidth={1.5} />
              {totalHours}h
            </span>
            {totalCommits > 0 && (
              <span className="flex items-center gap-1">
                <GitCommit className="w-3 h-3" strokeWidth={1.5} />
                {totalCommits}
              </span>
            )}
          </div>
        </div>
      </div>

      {/* Source sections */}
      <div className="divide-y divide-border">
        {claudeItems.length > 0 && (
          <SourceSection source="claude_code" items={claudeItems} projectPath={projectPath} />
        )}
      </div>
    </div>
  )
}

// Manual item card
interface ManualItemCardProps {
  item: WorkItem
  onEdit?: () => void
  onDelete?: () => void
}

function ManualItemCard({ item, onEdit, onDelete }: ManualItemCardProps) {
  return (
    <div className="group/card border border-border rounded-lg bg-white/60 dark:bg-white/5 overflow-hidden cursor-pointer hover:border-amber-300 dark:hover:border-amber-700 transition-colors">
      <div className="px-4 py-3 bg-amber-50/50 dark:bg-amber-900/10 border-b border-border">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <FileText className="w-4 h-4 text-amber-600 dark:text-amber-400" strokeWidth={1.5} />
            <span className="text-sm font-medium text-foreground">
              {formatDateDisplay(item.date)}
            </span>
            <span className="text-xs px-1.5 py-0.5 rounded bg-amber-100 dark:bg-amber-900/30 text-amber-700 dark:text-amber-300">
              手動
            </span>
          </div>
          <div className="flex items-center gap-2">
            {/* Edit/Delete buttons - show on hover */}
            {(onEdit || onDelete) && (
              <div className="flex items-center gap-1 opacity-0 group-hover/card:opacity-100 transition-opacity">
                {onEdit && (
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-7 w-7"
                    onClick={(e) => { e.stopPropagation(); onEdit(); }}
                    title="編輯"
                  >
                    <Pencil className="w-3 h-3" strokeWidth={1.5} />
                  </Button>
                )}
                {onDelete && (
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-7 w-7 text-destructive"
                    onClick={(e) => { e.stopPropagation(); onDelete(); }}
                    title="刪除"
                  >
                    <Trash2 className="w-3 h-3" strokeWidth={1.5} />
                  </Button>
                )}
              </div>
            )}
            <span className="flex items-center gap-1 text-xs text-muted-foreground">
              <Clock className="w-3 h-3" strokeWidth={1.5} />
              {item.hours}h
            </span>
          </div>
        </div>
      </div>
      <div className="px-4 py-3" onClick={onEdit}>
        <h3 className="text-sm font-medium text-foreground mb-1">{item.title}</h3>
        {item.description && (
          <p className="text-sm text-muted-foreground">{item.description}</p>
        )}
      </div>
    </div>
  )
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
  const projectPath = detail?.project_path ?? null
  const isManual = isManualProjectPath(projectPath)

  // State for hourly breakdown data (for non-manual projects)
  const [hourlyData, setHourlyData] = useState<Record<string, HourlyBreakdownItem[]>>({})
  // State for manual work items
  const [manualItems, setManualItems] = useState<WorkItem[]>([])
  const [isLoading, setIsLoading] = useState(true)

  // Edit/Delete state for manual items
  const [selectedItem, setSelectedItem] = useState<WorkItem | null>(null)
  const [editOpen, setEditOpen] = useState(false)
  const [deleteOpen, setDeleteOpen] = useState(false)
  const [formData, setFormData] = useState<WorkItemFormData>({
    title: '',
    description: '',
    hours: 0,
    date: '',
    jira_issue_key: '',
    category: '',
    project_name: '',
  })

  // Fetch data based on project type
  useEffect(() => {
    if (!projectPath || !periodStart || !periodEnd) {
      setIsLoading(false)
      return
    }

    const fetchData = async () => {
      setIsLoading(true)

      if (isManual) {
        // Fetch manual work items for this project
        try {
          const response = await workItems.list({
            source: 'manual',
            start_date: periodStart,
            end_date: periodEnd,
            per_page: 100,
            show_all: true,
          })
          // Filter by project name (derive from project_path)
          const filtered = response.items.filter((item: WorkItem) => {
            if (item.project_path && isManualProjectPath(item.project_path)) {
              const itemProjectName = item.project_path.split(/[/\\]/).pop() || ''
              return itemProjectName === decodedProjectName
            }
            return false
          })
          setManualItems(filtered)
        } catch (err) {
          console.error('Failed to fetch manual items:', err)
          setManualItems([])
        }
      } else {
        // Fetch hourly breakdown for non-manual projects
        const dates = getDateRange(periodStart, periodEnd)
        const dataByDate: Record<string, HourlyBreakdownItem[]> = {}

        await Promise.all(
          dates.map(async (date) => {
            try {
              const items = await worklog.getHourlyBreakdown(date, projectPath)
              if (items.length > 0) {
                dataByDate[date] = items
              }
            } catch (err) {
              console.error(`Failed to fetch hourly breakdown for ${date}:`, err)
            }
          })
        )

        setHourlyData(dataByDate)
      }

      setIsLoading(false)
    }

    fetchData()
  }, [projectPath, periodStart, periodEnd, isManual, decodedProjectName])

  // Refetch manual items
  const refetchManualItems = useCallback(async () => {
    if (!isManual || !periodStart || !periodEnd) return
    try {
      const response = await workItems.list({
        source: 'manual',
        start_date: periodStart,
        end_date: periodEnd,
        per_page: 100,
        show_all: true,
      })
      const filtered = response.items.filter((item: WorkItem) => {
        if (item.project_path && isManualProjectPath(item.project_path)) {
          const itemProjectName = item.project_path.split(/[/\\]/).pop() || ''
          return itemProjectName === decodedProjectName
        }
        return false
      })
      setManualItems(filtered)
    } catch (err) {
      console.error('Failed to refetch manual items:', err)
    }
  }, [isManual, periodStart, periodEnd, decodedProjectName])

  // Open edit modal for a manual item
  const openEditModal = useCallback((item: WorkItem) => {
    setSelectedItem(item)
    // Derive project_name from project_path
    let project_name = ''
    if (item.project_path) {
      const segments = item.project_path.split(/[/\\]/)
      project_name = segments[segments.length - 1] || ''
    }
    setFormData({
      title: item.title,
      description: item.description || '',
      hours: item.hours,
      date: item.date,
      jira_issue_key: item.jira_issue_key || '',
      category: item.category || '',
      project_name,
    })
    setEditOpen(true)
  }, [])

  // Handle update
  const handleUpdate = useCallback(async (e: React.FormEvent) => {
    e.preventDefault()
    if (!selectedItem) return

    try {
      await workItems.update(selectedItem.id, {
        title: formData.title,
        description: formData.description || undefined,
        hours: formData.hours,
        date: formData.date,
        jira_issue_key: formData.jira_issue_key || undefined,
        category: formData.category || undefined,
        project_name: formData.project_name || undefined,
      })
      setEditOpen(false)
      setSelectedItem(null)
      refetchManualItems()
    } catch (err) {
      console.error('Failed to update work item:', err)
    }
  }, [selectedItem, formData, refetchManualItems])

  // Open delete confirmation
  const openDeleteConfirm = useCallback((item: WorkItem) => {
    setSelectedItem(item)
    setDeleteOpen(true)
  }, [])

  // Handle delete
  const handleDelete = useCallback(async () => {
    if (!selectedItem) return

    try {
      await workItems.remove(selectedItem.id)
      setDeleteOpen(false)
      setSelectedItem(null)
      refetchManualItems()
    } catch (err) {
      console.error('Failed to delete work item:', err)
    }
  }, [selectedItem, refetchManualItems])

  // Calculate stats
  const { totalHours, totalCommits, datesWithData } = useMemo(() => {
    if (isManual) {
      // For manual items
      const hours = manualItems.reduce((sum, item) => sum + item.hours, 0)
      const sortedItems = [...manualItems].sort((a, b) => b.date.localeCompare(a.date))
      const dates = [...new Set(sortedItems.map((item) => item.date))]
      return { totalHours: hours, totalCommits: 0, datesWithData: dates }
    }

    // For non-manual projects
    const sortedDates = Object.keys(hourlyData).sort((a, b) => b.localeCompare(a))
    let hours = 0
    let commits = 0

    for (const items of Object.values(hourlyData)) {
      hours += items.length
      commits += items.reduce((sum, item) => sum + item.git_commits.length, 0)
    }

    return { totalHours: hours, totalCommits: commits, datesWithData: sortedDates }
  }, [hourlyData, manualItems, isManual])

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
      <Button variant="ghost" size="sm" onClick={() => navigate(`/projects?project=${encodeURIComponent(decodedProjectName)}`)}>
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
            {totalHours}h total
          </span>
          {!isManual && (
            <span className="flex items-center gap-1.5">
              <GitCommit className="w-4 h-4" strokeWidth={1.5} />
              {totalCommits} commits
            </span>
          )}
        </div>
      </div>

      {/* Work records */}
      {isManual ? (
        // Manual items display
        manualItems.length > 0 ? (
          <div className="space-y-4">
            <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
              工作紀錄 ({manualItems.length} 項)
            </h2>
            <div className="space-y-4">
              {manualItems.map((item) => (
                <ManualItemCard
                  key={item.id}
                  item={item}
                  onEdit={() => openEditModal(item)}
                  onDelete={() => openDeleteConfirm(item)}
                />
              ))}
            </div>
          </div>
        ) : (
          <div className="py-16 text-center">
            <p className="text-muted-foreground">此期間無工作紀錄</p>
          </div>
        )
      ) : (
        // Hourly breakdown display for non-manual projects
        datesWithData.length > 0 ? (
          <div className="space-y-4">
            <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
              工作紀錄 ({datesWithData.length} 天)
            </h2>
            <div className="space-y-4">
              {datesWithData.map((date) => (
                <DaySection
                  key={date}
                  date={date}
                  items={hourlyData[date]}
                  projectPath={projectPath}
                />
              ))}
            </div>
          </div>
        ) : (
          <div className="py-16 text-center">
            <p className="text-muted-foreground">此期間無工作紀錄</p>
          </div>
        )
      )}

      {/* Edit Modal */}
      <EditModal
        open={editOpen}
        onOpenChange={setEditOpen}
        formData={formData}
        setFormData={setFormData}
        onSubmit={handleUpdate}
        onCancel={() => { setEditOpen(false); setSelectedItem(null); }}
      />

      {/* Delete Confirmation Modal */}
      <DeleteModal
        open={deleteOpen}
        onOpenChange={setDeleteOpen}
        itemToDelete={selectedItem}
        onConfirm={handleDelete}
        onCancel={() => { setDeleteOpen(false); setSelectedItem(null); }}
      />
    </div>
  )
}
