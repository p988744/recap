import { Fragment } from 'react'
import {
  Briefcase,
  Plus,
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
} from 'lucide-react'
import { Card, CardContent } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
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
  TooltipTrigger,
} from '@/components/ui/tooltip'
import { cn } from '@/lib/utils'
import type { WorkItem, WorkItemWithChildren } from '@/types'
import { SOURCE_LABELS } from '../hooks/useWorkItems'

interface ListViewProps {
  items: WorkItemWithChildren[]
  total: number
  page: number
  totalPages: number
  expandedItems: Set<string>
  childrenData: Record<string, WorkItem[]>
  loadingChildren: Set<string>
  aggregating: boolean
  aggregateResult: string | null
  onAggregate: () => void
  onClearAggregateResult: () => void
  onToggleExpand: (itemId: string) => void
  onEdit: (item: WorkItem) => void
  onDelete: (item: WorkItem) => void
  onJiraMap: (item: WorkItem) => void
  onCreateNew: () => void
  setPage: (page: number) => void
}

export function ListView({
  items,
  total,
  page,
  totalPages,
  expandedItems,
  childrenData,
  loadingChildren,
  aggregating,
  aggregateResult,
  onAggregate,
  onClearAggregateResult,
  onToggleExpand,
  onEdit,
  onDelete,
  onJiraMap,
  onCreateNew,
  setPage,
}: ListViewProps) {
  if (items.length === 0) {
    return (
      <Card>
        <CardContent className="p-16">
          <div className="text-center">
            <Briefcase className="w-12 h-12 mx-auto mb-4 text-muted-foreground/30" strokeWidth={1} />
            <h3 className="font-display text-xl text-foreground mb-2">尚無工作項目</h3>
            <p className="text-sm text-muted-foreground mb-6">
              從 Claude Code 或 GitLab 同步工作紀錄，或手動新增項目
            </p>
            <Button variant="outline" onClick={onCreateNew}>
              <Plus className="w-4 h-4 mr-2" strokeWidth={1.5} />
              新增第一個項目
            </Button>
          </div>
        </CardContent>
      </Card>
    )
  }

  return (
    <>
      {/* Header */}
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
              onClick={onAggregate}
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
          <Button variant="ghost" size="sm" onClick={onClearAggregateResult}>
            關閉
          </Button>
        </div>
      )}

      {/* Table */}
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
                          onClick={() => onToggleExpand(item.id)}
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
                      {SOURCE_LABELS[item.source] || item.source}
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
                        onClick={() => onJiraMap(item)}
                      >
                        {item.jira_issue_key}
                        <ExternalLink className="w-3 h-3" strokeWidth={1.5} />
                      </button>
                    ) : (
                      <button
                        onClick={() => onJiraMap(item)}
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
                            onClick={() => onEdit(item)}
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
                            onClick={() => onDelete(item)}
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
                        {SOURCE_LABELS[child.source] || child.source}
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

      {/* Pagination */}
      {totalPages > 1 && (
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
    </>
  )
}
