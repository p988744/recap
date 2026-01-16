import {
  FileText,
  Award,
  CheckCircle,
} from 'lucide-react'
import { Card, CardContent } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Label } from '@/components/ui/label'
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
import type { PEReport } from '@/types'

interface PEReportTabProps {
  peReport: PEReport | null
  peYear: number
  setPEYear: (year: number) => void
  peHalf: 1 | 2
  setPEHalf: (half: 1 | 2) => void
}

export function PEReportTab({ peReport, peYear, setPEYear, peHalf, setPEHalf }: PEReportTabProps) {
  return (
    <div className="mt-8 space-y-8">
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
    </div>
  )
}
