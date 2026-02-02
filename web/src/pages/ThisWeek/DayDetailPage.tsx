import { useParams, useNavigate } from 'react-router-dom'
import { ArrowLeft, Clock, FolderKanban, GitCommit, Copy, Download } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
  TooltipProvider,
} from '@/components/ui/tooltip'
import { useAuth } from '@/lib/auth'
import { useDayDetail } from './hooks/useDayDetail'
import { ProjectCard } from '@/pages/Worklog/components/ProjectCard'
import { ManualItemCard } from '@/pages/Worklog/components/ManualItemCard'
import { DayGanttChart } from './components'

import type { WorklogDay } from '@/types/worklog'

// Get weekday label in Chinese based on actual day of week (0=Sunday, 1=Monday, ...)
function getWeekdayLabel(dayOfWeek: number): string {
  const labels = ['週日', '週一', '週二', '週三', '週四', '週五', '週六']
  return labels[dayOfWeek] || ''
}

// Format date for display
function formatDateDisplay(dateStr: string): string {
  const d = new Date(dateStr + 'T00:00:00')
  const weekday = getWeekdayLabel(d.getDay())
  return `${weekday} ${d.getFullYear()}/${String(d.getMonth() + 1).padStart(2, '0')}/${String(d.getDate()).padStart(2, '0')}`
}

// Generate day report in Markdown format
function generateDayReport(
  date: string,
  day: WorklogDay | null,
  totalHours: number,
  totalCommits: number
): string {
  const lines: string[] = []
  const formattedDate = formatDateDisplay(date)

  lines.push(`# ${formattedDate} 工作報告`)
  lines.push('')
  lines.push(`- **總工時**: ${totalHours.toFixed(1)} 小時`)
  lines.push(`- **Commits**: ${totalCommits}`)
  lines.push('')

  if (day?.projects && day.projects.length > 0) {
    lines.push('## 專案工作')
    lines.push('')
    for (const project of day.projects) {
      const projectName = project.project_path.split(/[/\\]/).pop() || project.project_path
      lines.push(`### ${projectName}`)
      lines.push('')
      lines.push(`- 工時: ${project.total_hours.toFixed(1)}h`)
      lines.push(`- Commits: ${project.total_commits}`)
      if (project.daily_summary) {
        lines.push('')
        lines.push(project.daily_summary)
      }
      lines.push('')
    }
  }

  if (day?.manual_items && day.manual_items.length > 0) {
    lines.push('## 手動項目')
    lines.push('')
    for (const item of day.manual_items) {
      lines.push(`- **${item.title}** (${item.hours.toFixed(1)}h)`)
      if (item.description) {
        lines.push(`  ${item.description}`)
      }
    }
    lines.push('')
  }

  return lines.join('\n')
}

export function DayDetailPage() {
  const { date } = useParams<{ date: string }>()
  const navigate = useNavigate()
  const { isAuthenticated } = useAuth()
  const {
    day,
    loading,
    totalHours,
    totalCommits,
    projectCount,
    expandedProject,
    hourlyData,
    hourlyLoading,
    toggleHourlyBreakdown,
  } = useDayDetail(date ?? '', isAuthenticated)

  // Copy report to clipboard
  const handleCopy = () => {
    if (!date) return
    const report = generateDayReport(date, day, totalHours, totalCommits)
    navigator.clipboard.writeText(report)
  }

  // Export report as Markdown file
  const handleExportMarkdown = () => {
    if (!date) return
    const report = generateDayReport(date, day, totalHours, totalCommits)
    const blob = new Blob([report], { type: 'text/markdown' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `worklog_${date}.md`
    a.click()
    URL.revokeObjectURL(url)
  }

  if (!date) {
    return (
      <div className="p-8 text-center">
        <p className="text-muted-foreground">Invalid date</p>
      </div>
    )
  }

  if (loading) {
    return (
      <div className="space-y-8">
        {/* Back button */}
        <Button variant="ghost" size="sm" onClick={() => navigate('/')}>
          <ArrowLeft className="w-4 h-4 mr-2" strokeWidth={1.5} />
          返回本週工作
        </Button>

        <div className="flex items-center justify-center h-48">
          <div className="w-6 h-6 border border-border border-t-foreground/60 rounded-full animate-spin" />
        </div>
      </div>
    )
  }

  const hasProjects = day && day.projects.length > 0
  const hasManualItems = day && day.manual_items.length > 0
  const isEmpty = !hasProjects && !hasManualItems

  return (
    <div className="space-y-8 animate-fade-up">
      {/* Back button */}
      <Button variant="ghost" size="sm" onClick={() => navigate('/')}>
        <ArrowLeft className="w-4 h-4 mr-2" strokeWidth={1.5} />
        返回本週工作
      </Button>

      {/* Header */}
      <div>
        <div className="flex items-start justify-between">
          <h1 className="text-2xl font-semibold text-foreground mb-2">
            {formatDateDisplay(date)}
          </h1>
          {!isEmpty && (
            <TooltipProvider>
              <div className="flex items-center gap-2">
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button variant="outline" size="sm" onClick={handleCopy}>
                      <Copy className="w-4 h-4 mr-1.5" strokeWidth={1.5} />
                      複製
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>複製報告到剪貼簿</TooltipContent>
                </Tooltip>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button variant="ghost" size="sm" onClick={handleExportMarkdown}>
                      <Download className="w-4 h-4 mr-1.5" strokeWidth={1.5} />
                      匯出
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>下載 Markdown 檔案</TooltipContent>
                </Tooltip>
              </div>
            </TooltipProvider>
          )}
        </div>
        {!isEmpty && (
          <div className="flex items-center gap-6 text-sm text-muted-foreground">
            <span className="flex items-center gap-1.5">
              <Clock className="w-4 h-4" strokeWidth={1.5} />
              總工時: {totalHours.toFixed(1)}h
            </span>
            <span className="flex items-center gap-1.5">
              <FolderKanban className="w-4 h-4" strokeWidth={1.5} />
              專案數: {projectCount}
            </span>
            <span className="flex items-center gap-1.5">
              <GitCommit className="w-4 h-4" strokeWidth={1.5} />
              Commits: {totalCommits}
            </span>
          </div>
        )}
      </div>

      {/* Content */}
      {isEmpty ? (
        <Card>
          <CardContent className="py-16 text-center">
            <p className="text-muted-foreground">當日無工作紀錄</p>
          </CardContent>
        </Card>
      ) : (
        <div className="space-y-6">
          {/* Gantt Chart - hourly timeline */}
          {hasProjects && (
            <Card>
              <CardContent className="py-4">
                <DayGanttChart date={date} projects={day.projects} />
              </CardContent>
            </Card>
          )}

          {/* Projects */}
          {hasProjects && (
            <section className="space-y-3">
              <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
                專案工作
              </h2>
              {day.projects.map((project) => {
                const isExpanded = expandedProject === project.project_path
                return (
                  <ProjectCard
                    key={project.project_path}
                    project={project}
                    date={date}
                    isExpanded={isExpanded}
                    hourlyData={isExpanded ? hourlyData : []}
                    hourlyLoading={isExpanded ? hourlyLoading : false}
                    onToggleHourly={() => toggleHourlyBreakdown(project.project_path)}
                  />
                )
              })}
            </section>
          )}

          {/* Manual items */}
          {hasManualItems && (
            <section className="space-y-3">
              <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
                手動項目
              </h2>
              {day.manual_items.map((item) => (
                <ManualItemCard key={item.id} item={item} />
              ))}
            </section>
          )}
        </div>
      )}
    </div>
  )
}
