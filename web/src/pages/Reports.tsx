import { useEffect, useState } from 'react'
import {
  FileText,
  Calendar,
  Download,
  Copy,
  RefreshCw,
  Clock,
  FolderGit2,
  Award,
  CheckCircle,
  Zap,
  Sparkles,
} from 'lucide-react'
import { Card, CardContent } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Label } from '@/components/ui/label'
import { Badge } from '@/components/ui/badge'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Separator } from '@/components/ui/separator'
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
import { reports } from '@/services'
import type { AnalyzeResponse, LegacyPersonalReport, PEReport, TempoReport, TempoReportPeriod } from '@/types'
import { formatHours, formatDateFull, cn } from '@/lib/utils'
import { useAuth } from '@/lib/auth'

type ReportPeriod = 'week' | 'last-week' | '7days' | '30days'
type ReportTab = 'work' | 'pe' | 'tempo'

export function Reports() {
  const { token, isAuthenticated } = useAuth()
  const [data, setData] = useState<AnalyzeResponse | null>(null)
  const [personalReport, setPersonalReport] = useState<LegacyPersonalReport | null>(null)
  const [peReport, setPEReport] = useState<PEReport | null>(null)
  const [loading, setLoading] = useState(true)
  const [period, setPeriod] = useState<ReportPeriod>('week')
  const [activeTab, setActiveTab] = useState<ReportTab>('work')
  const [peYear, setPEYear] = useState(new Date().getFullYear())
  const [peHalf, setPEHalf] = useState<1 | 2>(new Date().getMonth() < 6 ? 1 : 2)
  const [tempoReport, setTempoReport] = useState<TempoReport | null>(null)
  const [tempoPeriod, setTempoPeriod] = useState<TempoReportPeriod>('weekly')
  const [tempoLoading, setTempoLoading] = useState(false)

  const fetchReport = async (p: ReportPeriod) => {
    setLoading(true)
    try {
      let result: AnalyzeResponse
      switch (p) {
        case 'week':
          result = await reports.analyzeWeek()
          break
        case 'last-week':
          result = await reports.analyzeLastWeek()
          break
        case '7days':
          result = await reports.analyzeDays(7)
          break
        case '30days':
          result = await reports.analyzeDays(30)
          break
        default:
          result = await reports.analyzeWeek()
      }
      setData(result)

      // Also fetch personal report for the same date range
      if (result.start_date && result.end_date) {
        try {
          const personal = await reports.getLegacyPersonalReport(result.start_date, result.end_date)
          setPersonalReport(personal)
        } catch (err) {
          console.error('Failed to fetch personal report:', err)
        }
      }
    } catch (err) {
      console.error('Failed to fetch report:', err)
    } finally {
      setLoading(false)
    }
  }

  const fetchPEReport = async () => {
    setLoading(true)
    try {
      const result = await reports.getPEReport(peYear, peHalf)
      setPEReport(result)
    } catch (err) {
      console.error('Failed to fetch PE report:', err)
    } finally {
      setLoading(false)
    }
  }

  const fetchTempoReport = async (p: TempoReportPeriod) => {
    setTempoLoading(true)
    try {
      const result = await reports.generateTempoReport({ period: p })
      setTempoReport(result)
    } catch (err) {
      console.error('Failed to fetch tempo report:', err)
    } finally {
      setTempoLoading(false)
    }
  }

  useEffect(() => {
    // Only fetch when authenticated
    if (!isAuthenticated || !token) {
      return
    }
    if (activeTab === 'work') {
      fetchReport(period)
    } else if (activeTab === 'pe') {
      fetchPEReport()
    } else if (activeTab === 'tempo') {
      fetchTempoReport(tempoPeriod)
    }
  }, [period, activeTab, peYear, peHalf, tempoPeriod, isAuthenticated, token])

  const generateReport = () => {
    if (!data) return ''

    const lines = [
      `工作報告：${data.start_date} ~ ${data.end_date}`,
      '',
      `總工時：${formatHours(data.total_minutes)} 小時`,
      `工作天數：${data.dates_covered.length} 天`,
      `專案數：${data.projects.length}`,
      '',
      '## 專案明細',
      '',
    ]

    data.projects.forEach((project, index) => {
      lines.push(`### ${index + 1}. ${project.project_name}`)
      lines.push(`- 總時數：${formatHours(project.total_minutes)} 小時`)
      if (project.jira_id) {
        lines.push(`- Jira Issue：${project.jira_id}`)
      }
      lines.push('')

      project.daily_entries.forEach((entry) => {
        lines.push(`  - ${entry.date}：${entry.hours.toFixed(1)}h`)
        if (entry.description) {
          lines.push(`    ${entry.description}`)
        }
      })
      lines.push('')
    })

    return lines.join('\n')
  }

  const handleCopy = () => {
    const report = generateReport()
    navigator.clipboard.writeText(report)
  }

  const handleExportMarkdown = async () => {
    if (!data?.start_date || !data?.end_date) return
    try {
      const markdown = await reports.exportMarkdownReport(data.start_date, data.end_date)
      const blob = new Blob([markdown], { type: 'text/markdown' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = `report_${data.start_date}_${data.end_date}.md`
      a.click()
      URL.revokeObjectURL(url)
    } catch (err) {
      console.error('Export failed:', err)
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-96">
        <div className="w-6 h-6 border border-border border-t-foreground/60 rounded-full animate-spin" />
      </div>
    )
  }

  return (
    <TooltipProvider>
      <div className="space-y-12">
        {/* Header */}
        <header className="flex items-start justify-between animate-fade-up opacity-0 delay-1">
          <div>
            <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
              工作報告
            </p>
            <h1 className="font-display text-4xl text-foreground tracking-tight">報告中心</h1>
          </div>
          <Button variant="ghost" onClick={() => {
            if (activeTab === 'work') fetchReport(period)
            else if (activeTab === 'pe') fetchPEReport()
            else if (activeTab === 'tempo') fetchTempoReport(tempoPeriod)
          }}>
            <RefreshCw className="w-4 h-4 mr-2" strokeWidth={1.5} />
            重新整理
          </Button>
        </header>

        {/* Report Type Tabs */}
        <section className="animate-fade-up opacity-0 delay-2">
          <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as ReportTab)}>
            <TabsList>
              <TabsTrigger value="work" className="gap-2">
                <FileText className="w-4 h-4" strokeWidth={1.5} />
                工作報告
              </TabsTrigger>
              <TabsTrigger value="pe" className="gap-2">
                <Award className="w-4 h-4" strokeWidth={1.5} />
                績效考核
              </TabsTrigger>
              <TabsTrigger value="tempo" className="gap-2">
                <Zap className="w-4 h-4" strokeWidth={1.5} />
                Tempo 報告
              </TabsTrigger>
            </TabsList>

            {/* Work Report Tab */}
            <TabsContent value="work" className="mt-8 space-y-8">
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

              {/* Category Breakdown from Personal Report */}
              {personalReport && Object.keys(personalReport.category_breakdown).length > 0 && (
                <div>
                  <div className="flex items-center gap-2 mb-4">
                    <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
                      類別統計
                    </p>
                  </div>
                  <div className="grid grid-cols-4 gap-3">
                    {Object.entries(personalReport.category_breakdown)
                      .sort((a, b) => b[1] - a[1])
                      .slice(0, 4)
                      .map(([category, hours]) => (
                        <Card key={category}>
                          <CardContent className="p-4">
                            <p className="text-xs text-muted-foreground truncate">{category}</p>
                            <p className="font-display text-xl text-foreground mt-1">{hours.toFixed(1)}h</p>
                          </CardContent>
                        </Card>
                      ))}
                  </div>
                </div>
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
            </TabsContent>

            {/* PE Report Tab */}
            <TabsContent value="pe" className="mt-8 space-y-8">
              {/* Year/Half Selector */}
              <div className="flex items-center gap-4">
                <div className="space-y-2">
                  <Label className="text-xs">年度</Label>
                  <Select
                    value={peYear.toString()}
                    onValueChange={(v) => setPEYear(parseInt(v))}
                  >
                    <SelectTrigger className="w-[120px]">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="2024">2024</SelectItem>
                      <SelectItem value="2025">2025</SelectItem>
                      <SelectItem value="2026">2026</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                <div className="space-y-2">
                  <Label className="text-xs">期間</Label>
                  <Select
                    value={peHalf.toString()}
                    onValueChange={(v) => setPEHalf(parseInt(v) as 1 | 2)}
                  >
                    <SelectTrigger className="w-[160px]">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="1">上半年 (1-6月)</SelectItem>
                      <SelectItem value="2">下半年 (7-12月)</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>

              {/* PE Summary */}
              {peReport && (
                <>
                  <Card className="border-l-2 border-l-warm/60">
                    <CardContent className="p-8">
                      <div className="flex items-center gap-2 mb-6">
                        <Award className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
                        <span className="text-sm text-muted-foreground">
                          {peReport.evaluation_period}
                        </span>
                      </div>
                      <div className="grid grid-cols-4 gap-6">
                        <div>
                          <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
                            總工時
                          </p>
                          <p className="font-display text-3xl text-foreground">
                            {peReport.total_hours.toFixed(0)}
                          </p>
                          <p className="text-sm text-muted-foreground mt-1">小時</p>
                        </div>
                        <div>
                          <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
                            Jira Issues
                          </p>
                          <p className="font-display text-3xl text-foreground">
                            {peReport.jira_issues_count}
                          </p>
                          <p className="text-sm text-muted-foreground mt-1">個</p>
                        </div>
                        <div>
                          <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
                            Commits
                          </p>
                          <p className="font-display text-3xl text-foreground">
                            {peReport.commits_count}
                          </p>
                          <p className="text-sm text-muted-foreground mt-1">筆</p>
                        </div>
                        <div>
                          <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
                            Merge Requests
                          </p>
                          <p className="font-display text-3xl text-foreground">
                            {peReport.merge_requests_count}
                          </p>
                          <p className="text-sm text-muted-foreground mt-1">筆</p>
                        </div>
                      </div>
                    </CardContent>
                  </Card>

                  {/* Work Results */}
                  {peReport.work_results.length > 0 && (
                    <div>
                      <div className="flex items-center gap-2 mb-4">
                        <FileText className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
                        <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
                          工作成果
                        </p>
                      </div>
                      <Card className="overflow-hidden">
                        <Table>
                          <TableHeader>
                            <TableRow className="hover:bg-transparent">
                              <TableHead className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground font-medium">
                                工作項目
                              </TableHead>
                              <TableHead className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground font-medium">
                                期間
                              </TableHead>
                              <TableHead className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground font-medium">
                                成果說明
                              </TableHead>
                              <TableHead className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground font-medium text-right">
                                權重
                              </TableHead>
                            </TableRow>
                          </TableHeader>
                          <TableBody>
                            {peReport.work_results.map((result, index) => (
                              <TableRow key={index}>
                                <TableCell className="text-sm text-foreground">{result.title}</TableCell>
                                <TableCell className="text-sm text-muted-foreground">{result.period}</TableCell>
                                <TableCell className="text-sm text-muted-foreground">{result.result_description}</TableCell>
                                <TableCell className="text-sm text-foreground text-right tabular-nums">
                                  {(result.weight * 100).toFixed(0)}%
                                </TableCell>
                              </TableRow>
                            ))}
                          </TableBody>
                        </Table>
                      </Card>
                    </div>
                  )}

                  {/* Skills */}
                  {peReport.skills.length > 0 && (
                    <div>
                      <div className="flex items-center gap-2 mb-4">
                        <CheckCircle className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
                        <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
                          技能發展
                        </p>
                      </div>
                      <div className="grid grid-cols-2 gap-4">
                        {peReport.skills.map((skill, index) => (
                          <Card key={index}>
                            <CardContent className="p-4">
                              <p className="text-sm font-medium text-foreground mb-1">{skill.name}</p>
                              <p className="text-xs text-muted-foreground">{skill.description}</p>
                            </CardContent>
                          </Card>
                        ))}
                      </div>
                    </div>
                  )}

                  {/* Goal Progress */}
                  {peReport.goal_progress.length > 0 && (
                    <div>
                      <div className="flex items-center gap-2 mb-4">
                        <Award className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
                        <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
                          年度目標達成
                        </p>
                      </div>
                      <div className="space-y-3">
                        {peReport.goal_progress.map((goal) => (
                          <Card key={goal.goal_id} className="border-l-2 border-l-sage/40">
                            <CardContent className="p-4">
                              <div className="flex items-start justify-between">
                                <div>
                                  <p className="text-sm font-medium text-foreground">{goal.goal_title}</p>
                                  <p className="text-xs text-muted-foreground mt-1">
                                    <Badge variant="outline" className="mr-2 font-normal">{goal.category}</Badge>
                                    {goal.work_item_count} 項工作 · {goal.total_hours.toFixed(1)}h
                                  </p>
                                </div>
                                <span className="text-sm text-muted-foreground tabular-nums">
                                  權重 {(goal.weight * 100).toFixed(0)}%
                                </span>
                              </div>
                            </CardContent>
                          </Card>
                        ))}
                      </div>
                    </div>
                  )}
                </>
              )}

              {!peReport && (
                <Card>
                  <CardContent className="py-16">
                    <div className="text-center text-muted-foreground">
                      <Award className="w-8 h-8 mx-auto mb-3 opacity-50" strokeWidth={1} />
                      <p className="text-sm">此期間沒有績效資料</p>
                    </div>
                  </CardContent>
                </Card>
              )}
            </TabsContent>

            {/* Tempo Report Tab */}
            <TabsContent value="tempo" className="mt-8 space-y-8">
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
                          <Button
                            variant="outline"
                            onClick={() => {
                              if (!tempoReport) return
                              const lines = [
                                `Tempo 工作報告：${tempoReport.period}`,
                                `期間：${tempoReport.start_date} ~ ${tempoReport.end_date}`,
                                '',
                                `總工時：${tempoReport.total_hours.toFixed(1)} 小時`,
                                `工作項目：${tempoReport.total_items} 筆`,
                                '',
                                '## 專案明細',
                                '',
                              ]
                              tempoReport.projects.forEach((project) => {
                                lines.push(`### ${project.project}`)
                                lines.push(`- 工時：${project.hours.toFixed(1)} 小時`)
                                lines.push(`- 項目數：${project.item_count}`)
                                if (project.summaries.length > 0) {
                                  lines.push('- 主要工作：')
                                  project.summaries.forEach(s => lines.push(`  - ${s}`))
                                }
                                lines.push('')
                              })
                              navigator.clipboard.writeText(lines.join('\n'))
                            }}
                          >
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
            </TabsContent>
          </Tabs>
        </section>
      </div>
    </TooltipProvider>
  )
}
