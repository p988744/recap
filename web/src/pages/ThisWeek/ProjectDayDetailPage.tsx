import { useParams, useNavigate } from 'react-router-dom'
import { ArrowLeft, Clock, GitCommit, FileCode } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { useAuth } from '@/lib/auth'
import { useProjectDayDetail } from './hooks/useProjectDayDetail'
import { HourlyBreakdown } from '@/pages/Worklog/components/HourlyBreakdown'
import { MarkdownSummary } from '@/components/MarkdownSummary'

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

export function ProjectDayDetailPage() {
  const { date, projectPath: encodedProjectPath } = useParams<{
    date: string
    projectPath: string
  }>()
  const navigate = useNavigate()
  const { isAuthenticated } = useAuth()

  // Decode the project path
  const projectPath = encodedProjectPath ? decodeURIComponent(encodedProjectPath) : ''

  const { project, hourlyData, loading, hourlyLoading } = useProjectDayDetail(
    date ?? '',
    projectPath,
    isAuthenticated
  )

  if (!date || !projectPath) {
    return (
      <div className="p-8 text-center">
        <p className="text-muted-foreground">Invalid parameters</p>
      </div>
    )
  }

  const handleBack = () => {
    navigate(`/day/${date}`)
  }

  if (loading) {
    return (
      <div className="space-y-8">
        {/* Back button */}
        <Button variant="ghost" size="sm" onClick={handleBack}>
          <ArrowLeft className="w-4 h-4 mr-2" strokeWidth={1.5} />
          返回 {formatDateDisplay(date)}
        </Button>

        <div className="flex items-center justify-center h-48">
          <div className="w-6 h-6 border border-border border-t-foreground/60 rounded-full animate-spin" />
        </div>
      </div>
    )
  }

  if (!project) {
    return (
      <div className="space-y-8">
        {/* Back button */}
        <Button variant="ghost" size="sm" onClick={handleBack}>
          <ArrowLeft className="w-4 h-4 mr-2" strokeWidth={1.5} />
          返回 {formatDateDisplay(date)}
        </Button>

        <Card>
          <CardContent className="py-16 text-center">
            <p className="text-muted-foreground">找不到該專案紀錄</p>
          </CardContent>
        </Card>
      </div>
    )
  }

  return (
    <div className="space-y-8 animate-fade-up">
      {/* Back button */}
      <Button variant="ghost" size="sm" onClick={handleBack}>
        <ArrowLeft className="w-4 h-4 mr-2" strokeWidth={1.5} />
        返回 {formatDateDisplay(date)}
      </Button>

      {/* Header */}
      <div>
        <h1 className="text-2xl font-semibold text-foreground mb-1">
          {project.project_name}
        </h1>
        <p className="text-sm text-muted-foreground mb-3">
          {formatDateDisplay(date)}
        </p>
        <div className="flex items-center gap-6 text-sm text-muted-foreground">
          <span className="flex items-center gap-1.5">
            <Clock className="w-4 h-4" strokeWidth={1.5} />
            工時: {project.total_hours.toFixed(1)}h
          </span>
          <span className="flex items-center gap-1.5">
            <GitCommit className="w-4 h-4" strokeWidth={1.5} />
            Commits: {project.total_commits}
          </span>
          <span className="flex items-center gap-1.5">
            <FileCode className="w-4 h-4" strokeWidth={1.5} />
            Files: {project.total_files}
          </span>
        </div>
      </div>

      {/* Daily Summary */}
      {project.daily_summary && (
        <section>
          <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-3">
            工作摘要
          </h2>
          <Card>
            <CardContent className="py-4">
              <MarkdownSummary content={project.daily_summary} />
            </CardContent>
          </Card>
        </section>
      )}

      {/* Hourly Breakdown */}
      <section>
        <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-3">
          小時明細
        </h2>
        <Card>
          <HourlyBreakdown items={hourlyData} loading={hourlyLoading} />
        </Card>
      </section>
    </div>
  )
}
