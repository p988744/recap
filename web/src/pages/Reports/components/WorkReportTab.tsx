import {
  FileText,
  Calendar,
  Download,
  Copy,
  Clock,
  FolderGit2,
} from 'lucide-react'
import { Card, CardContent } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Separator } from '@/components/ui/separator'
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip'
import type { AnalyzeResponse } from '@/types'
import { formatHours, formatDateFull, cn } from '@/lib/utils'
import { generateWorkReport, type ReportPeriod } from '../hooks/useReports'

interface WorkReportTabProps {
  data: AnalyzeResponse | null
  period: ReportPeriod
  setPeriod: (period: ReportPeriod) => void
}

export function WorkReportTab({ data, period, setPeriod }: WorkReportTabProps) {
  const handleCopy = () => {
    const report = generateWorkReport(data)
    navigator.clipboard.writeText(report)
  }

  const handleExportMarkdown = () => {
    if (!data?.start_date || !data?.end_date) return
    const markdown = generateWorkReport(data)
    const blob = new Blob([markdown], { type: 'text/markdown' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `report_${data.start_date}_${data.end_date}.md`
    a.click()
    URL.revokeObjectURL(url)
  }

  return (
    <div className="mt-8 space-y-8">
      {/* Period Selector */}
      <Tabs value={period} onValueChange={(v) => setPeriod(v as ReportPeriod)}>
        <TabsList>
          <TabsTrigger value="week">本週</TabsTrigger>
          <TabsTrigger value="last-week">上週</TabsTrigger>
          <TabsTrigger value="7days">近 7 天</TabsTrigger>
          <TabsTrigger value="30days">近 30 天</TabsTrigger>
        </TabsList>
      </Tabs>

      {/* Summary */}
      {data && (
        <Card className="border-l-2 border-l-warm/60">
          <CardContent className="p-8">
            <div className="flex items-center gap-2 mb-6">
              <Calendar className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
              <span className="text-sm text-muted-foreground">
                {formatDateFull(data.start_date)} — {formatDateFull(data.end_date)}
              </span>
            </div>
            <div className="grid grid-cols-3 gap-8">
              <div>
                <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
                  總工時
                </p>
                <p className="font-display text-4xl text-foreground">
                  {formatHours(data.total_minutes)}
                </p>
                <p className="text-sm text-muted-foreground mt-1">小時</p>
              </div>
              <div>
                <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
                  工作天數
                </p>
                <p className="font-display text-4xl text-foreground">
                  {data.dates_covered.length}
                </p>
                <p className="text-sm text-muted-foreground mt-1">天</p>
              </div>
              <div>
                <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
                  專案數
                </p>
                <p className="font-display text-4xl text-foreground">
                  {data.projects.length}
                </p>
                <p className="text-sm text-muted-foreground mt-1">個</p>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Project Details */}
      <div>
        <div className="flex items-center gap-2 mb-6">
          <FileText className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
          <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
            專案明細
          </p>
        </div>

        {data?.projects && data.projects.length > 0 ? (
          <div className="space-y-4">
            {data.projects.map((project, index) => (
              <Card
                key={project.project_name}
                className={cn(
                  "border-l-2 border-l-warm/40 animate-fade-up opacity-0",
                  index === 0 && "delay-4",
                  index === 1 && "delay-5",
                  index === 2 && "delay-6"
                )}
              >
                <CardContent className="p-6">
                  <div className="flex items-start justify-between mb-4">
                    <div className="flex items-center gap-3">
                      <FolderGit2 className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
                      <div>
                        <div className="flex items-center gap-2">
                          <span className="text-sm font-medium text-foreground">{project.project_name}</span>
                          {project.jira_id && (
                            <Badge variant="secondary" className="font-normal">
                              {project.jira_id}
                            </Badge>
                          )}
                        </div>
                        <p className="text-xs text-muted-foreground truncate">{project.project_path}</p>
                      </div>
                    </div>
                    <div className="text-right">
                      <p className="font-display text-2xl text-foreground">
                        {formatHours(project.total_minutes)}
                      </p>
                      <p className="text-xs text-muted-foreground">小時</p>
                    </div>
                  </div>

                  {/* Daily breakdown */}
                  <Separator className="my-4" />
                  <div className="space-y-2">
                    {project.daily_entries.map((entry) => (
                      <div
                        key={entry.date}
                        className="flex items-start gap-4 text-sm"
                      >
                        <span className="text-muted-foreground flex items-center gap-1 w-24 flex-shrink-0">
                          <Clock className="w-3 h-3" strokeWidth={1.5} />
                          {entry.date}
                        </span>
                        <span className="text-foreground tabular-nums w-12">{entry.hours.toFixed(1)}h</span>
                        <span className="text-muted-foreground flex-1 truncate">{entry.description}</span>
                      </div>
                    ))}
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        ) : (
          <Card>
            <CardContent className="py-16">
              <div className="text-center text-muted-foreground">
                <FileText className="w-8 h-8 mx-auto mb-3 opacity-50" strokeWidth={1} />
                <p className="text-sm">此期間沒有工作記錄</p>
              </div>
            </CardContent>
          </Card>
        )}
      </div>

      {/* Actions */}
      <div className="pt-8">
        <Separator className="mb-8" />
        <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-4">
          匯出報告
        </p>
        <div className="flex items-center gap-3">
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="outline" onClick={handleCopy}>
                <Copy className="w-4 h-4 mr-2" strokeWidth={1.5} />
                複製報告
              </Button>
            </TooltipTrigger>
            <TooltipContent>複製報告到剪貼簿</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="ghost" onClick={handleExportMarkdown}>
                <Download className="w-4 h-4 mr-2" strokeWidth={1.5} />
                匯出 Markdown
              </Button>
            </TooltipTrigger>
            <TooltipContent>下載 Markdown 檔案</TooltipContent>
          </Tooltip>
        </div>
      </div>
    </div>
  )
}
