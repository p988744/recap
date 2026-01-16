import { Link2, CheckCircle2, XCircle } from 'lucide-react'
import { Card, CardContent } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import type { WorkItem } from '@/types'
import type { ProjectGroup } from '../hooks/types'

interface TaskViewProps {
  taskGroups: ProjectGroup['issues']
  onItemClick: (item: WorkItem) => void
}

export function TaskView({ taskGroups, onItemClick }: TaskViewProps) {
  return (
    <>
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-2">
          <Link2 className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
          <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
            依 Jira 任務分組
          </p>
          <Badge variant="secondary" className="ml-2">{taskGroups.length} 任務</Badge>
        </div>
      </div>

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
                        const workItem = {
                          id: log.id,
                          title: log.title,
                          description: log.description,
                          hours: log.hours,
                          date: log.date,
                          source: log.source,
                          synced_to_tempo: log.synced_to_tempo,
                        } as WorkItem
                        onItemClick(workItem)
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
    </>
  )
}
