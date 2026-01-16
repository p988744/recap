import {
  Calendar,
  Copy,
  Zap,
  Sparkles,
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
import type { TempoReport, TempoReportPeriod } from '@/types'
import { formatDateFull, cn } from '@/lib/utils'
import { generateTempoReportText } from '../hooks/useReports'

interface TempoReportTabProps {
  tempoReport: TempoReport | null
  tempoPeriod: TempoReportPeriod
  setTempoPeriod: (period: TempoReportPeriod) => void
  tempoLoading: boolean
}

export function TempoReportTab({ tempoReport, tempoPeriod, setTempoPeriod, tempoLoading }: TempoReportTabProps) {
  const handleCopy = () => {
    if (!tempoReport) return
    const text = generateTempoReportText(tempoReport)
    navigator.clipboard.writeText(text)
  }

  return (
    <div className="mt-8 space-y-8">
      {/* Period Selector */}
      <Tabs value={tempoPeriod} onValueChange={(v) => setTempoPeriod(v as TempoReportPeriod)}>
        <TabsList>
          <TabsTrigger value="daily">今日</TabsTrigger>
          <TabsTrigger value="weekly">本週</TabsTrigger>
          <TabsTrigger value="monthly">本月</TabsTrigger>
          <TabsTrigger value="quarterly">本季</TabsTrigger>
          <TabsTrigger value="semi_annual">半年</TabsTrigger>
        </TabsList>
      </Tabs>

      {/* Loading State */}
      {tempoLoading && (
        <div className="flex items-center justify-center h-48">
          <div className="w-6 h-6 border border-border border-t-foreground/60 rounded-full animate-spin" />
        </div>
      )}

      {/* Tempo Report Content */}
      {!tempoLoading && tempoReport && (
        <>
          {/* Summary Card */}
          <Card className="border-l-2 border-l-warm/60">
            <CardContent className="p-8">
              <div className="flex items-center gap-2 mb-6">
                <Calendar className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
                <span className="text-sm text-muted-foreground">
                  {tempoReport.period}
                </span>
                {tempoReport.used_llm && (
                  <Badge variant="secondary" className="ml-2 gap-1">
                    <Sparkles className="w-3 h-3" />
                    AI 摘要
                  </Badge>
                )}
              </div>
              <div className="flex items-center gap-2 mb-6 text-xs text-muted-foreground">
                <span>{formatDateFull(tempoReport.start_date)}</span>
                <span>—</span>
                <span>{formatDateFull(tempoReport.end_date)}</span>
              </div>
              <div className="grid grid-cols-3 gap-8">
                <div>
                  <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
                    總工時
                  </p>
                  <p className="font-display text-4xl text-foreground">
                    {tempoReport.total_hours.toFixed(1)}
                  </p>
                  <p className="text-sm text-muted-foreground mt-1">小時</p>
                </div>
                <div>
                  <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
                    工作項目
                  </p>
                  <p className="font-display text-4xl text-foreground">
                    {tempoReport.total_items}
                  </p>
                  <p className="text-sm text-muted-foreground mt-1">筆</p>
                </div>
                <div>
                  <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
                    專案數
                  </p>
                  <p className="font-display text-4xl text-foreground">
                    {tempoReport.projects.length}
                  </p>
                  <p className="text-sm text-muted-foreground mt-1">個</p>
                </div>
              </div>
            </CardContent>
          </Card>

          {/* Project Details */}
          <div>
            <div className="flex items-center gap-2 mb-6">
              <FolderGit2 className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
              <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
                專案明細
              </p>
            </div>

            {tempoReport.projects.length > 0 ? (
              <div className="space-y-4">
                {tempoReport.projects.map((project, index) => (
                  <Card
                    key={project.project}
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
                            <span className="text-sm font-medium text-foreground">{project.project}</span>
                            <p className="text-xs text-muted-foreground mt-1">
                              {project.item_count} 項工作
                            </p>
                          </div>
                        </div>
                        <div className="text-right">
                          <p className="font-display text-2xl text-foreground">
                            {project.hours.toFixed(1)}
                          </p>
                          <p className="text-xs text-muted-foreground">小時</p>
                        </div>
                      </div>

                      {/* Smart Summaries */}
                      {project.summaries.length > 0 && (
                        <>
                          <Separator className="my-4" />
                          <div className="space-y-2">
                            {project.summaries.map((summary, i) => (
                              <div
                                key={i}
                                className="flex items-start gap-2 text-sm"
                              >
                                <span className="text-muted-foreground">•</span>
                                <span className="text-muted-foreground">{summary}</span>
                              </div>
                            ))}
                          </div>
                        </>
                      )}
                    </CardContent>
                  </Card>
                ))}
              </div>
            ) : (
              <Card>
                <CardContent className="py-16">
                  <div className="text-center text-muted-foreground">
                    <Zap className="w-8 h-8 mx-auto mb-3 opacity-50" strokeWidth={1} />
                    <p className="text-sm">此期間沒有工作記錄</p>
                  </div>
                </CardContent>
              </Card>
            )}
          </div>

          {/* Copy Report Button */}
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
            </div>
          </div>
        </>
      )}

      {/* No Report */}
      {!tempoLoading && !tempoReport && (
        <Card>
          <CardContent className="py-16">
            <div className="text-center text-muted-foreground">
              <Zap className="w-8 h-8 mx-auto mb-3 opacity-50" strokeWidth={1} />
              <p className="text-sm">無法載入 Tempo 報告</p>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  )
}
