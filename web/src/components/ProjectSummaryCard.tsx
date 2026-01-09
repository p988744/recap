import { useState } from 'react'
import { ChevronDown, ChevronRight, Link2, CheckCircle2, XCircle } from 'lucide-react'
import { Card, CardContent } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'

export interface WorkLogItem {
  id: string
  title: string
  description?: string
  hours: number
  date: string
  source: string
  synced_to_tempo: boolean
}

export interface JiraIssueGroup {
  jira_key?: string
  jira_title?: string
  total_hours: number
  logs: WorkLogItem[]
}

export interface ProjectGroup {
  project_name: string
  total_hours: number
  issues: JiraIssueGroup[]
}

interface ProjectSummaryCardProps {
  project: ProjectGroup
  onItemClick?: (item: WorkLogItem) => void
}

export function ProjectSummaryCard({ project, onItemClick }: ProjectSummaryCardProps) {
  const [expanded, setExpanded] = useState(false)

  const mappedCount = project.issues.filter(i => i.jira_key).length
  const totalIssues = project.issues.length

  return (
    <Card className="overflow-hidden">
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full text-left"
      >
        <CardContent className="p-5">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              {expanded ? (
                <ChevronDown className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
              ) : (
                <ChevronRight className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
              )}
              <div>
                <h3 className="font-medium text-foreground">{project.project_name}</h3>
                <p className="text-xs text-muted-foreground mt-0.5">
                  {totalIssues} 個任務群組
                </p>
              </div>
            </div>
            <div className="flex items-center gap-4">
              <div className="text-right">
                <p className="font-display text-xl text-foreground">
                  {project.total_hours.toFixed(1)}
                  <span className="text-sm text-muted-foreground ml-1">hrs</span>
                </p>
              </div>
              <Badge variant="secondary" className="text-xs">
                {mappedCount}/{totalIssues} 已對應
              </Badge>
            </div>
          </div>
        </CardContent>
      </button>

      {expanded && (
        <div className="border-t border-border">
          {project.issues.map((issue, idx) => (
            <IssueGroup
              key={issue.jira_key || `unmapped-${idx}`}
              issue={issue}
              onItemClick={onItemClick}
            />
          ))}
        </div>
      )}
    </Card>
  )
}

interface IssueGroupProps {
  issue: JiraIssueGroup
  onItemClick?: (item: WorkLogItem) => void
}

function IssueGroup({ issue, onItemClick }: IssueGroupProps) {
  const [expanded, setExpanded] = useState(false)

  return (
    <div className="border-b border-border last:border-b-0">
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full text-left px-5 py-3 hover:bg-muted/30 transition-colors"
      >
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            {expanded ? (
              <ChevronDown className="w-3 h-3 text-muted-foreground" strokeWidth={1.5} />
            ) : (
              <ChevronRight className="w-3 h-3 text-muted-foreground" strokeWidth={1.5} />
            )}
            {issue.jira_key ? (
              <div className="flex items-center gap-2">
                <Link2 className="w-3 h-3 text-blue-500" strokeWidth={1.5} />
                <span className="text-sm font-medium text-blue-600">{issue.jira_key}</span>
                {issue.jira_title && (
                  <span className="text-sm text-muted-foreground truncate max-w-[300px]">
                    {issue.jira_title}
                  </span>
                )}
              </div>
            ) : (
              <span className="text-sm text-muted-foreground">未對應 Jira</span>
            )}
          </div>
          <div className="flex items-center gap-3">
            <span className="text-sm font-medium tabular-nums">
              {issue.total_hours.toFixed(1)}h
            </span>
            <Badge variant="outline" className="text-xs">
              {issue.logs.length} 筆
            </Badge>
          </div>
        </div>
      </button>

      {expanded && (
        <div className="bg-muted/20">
          {issue.logs.map((log) => (
            <div
              key={log.id}
              onClick={() => onItemClick?.(log)}
              className={cn(
                'px-5 py-2 pl-10 border-b border-border/50 last:border-b-0',
                onItemClick && 'cursor-pointer hover:bg-muted/40'
              )}
            >
              <div className="flex items-center justify-between">
                <div className="flex-1 min-w-0">
                  <p className="text-sm text-foreground truncate">{log.title}</p>
                  <div className="flex items-center gap-2 mt-0.5">
                    <span className="text-xs text-muted-foreground">{log.date}</span>
                    <span className="text-xs text-muted-foreground">•</span>
                    <span className="text-xs text-muted-foreground">{log.source}</span>
                  </div>
                </div>
                <div className="flex items-center gap-3">
                  <span className="text-xs tabular-nums text-muted-foreground">
                    {log.hours.toFixed(1)}h
                  </span>
                  {log.synced_to_tempo ? (
                    <CheckCircle2 className="w-3 h-3 text-sage" strokeWidth={1.5} />
                  ) : (
                    <XCircle className="w-3 h-3 text-muted-foreground/50" strokeWidth={1.5} />
                  )}
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
