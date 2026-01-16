import { Link } from 'react-router-dom'
import { Briefcase, Link2, CheckCircle2, ArrowRight } from 'lucide-react'
import { Card, CardContent } from '@/components/ui/card'
import type { WorkItemStatsResponse } from '@/types'

interface WorkItemsStatusProps {
  stats: WorkItemStatsResponse
}

export function WorkItemsStatus({ stats }: WorkItemsStatusProps) {
  return (
    <section className="animate-fade-up opacity-0 delay-3">
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-2">
          <Briefcase className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
          <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
            工作項目狀態
          </h2>
        </div>
        <Link to="/work-items" className="text-xs text-muted-foreground hover:text-foreground transition-colors flex items-center gap-1">
          管理項目
          <ArrowRight className="w-3 h-3" strokeWidth={1.5} />
        </Link>
      </div>
      <div className="grid grid-cols-3 gap-4">
        <Card>
          <CardContent className="p-5">
            <div className="flex items-center gap-3">
              <Briefcase className="w-5 h-5 text-muted-foreground/50" strokeWidth={1.5} />
              <div>
                <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">總項目</p>
                <p className="font-display text-2xl text-foreground">{stats.total_items}</p>
              </div>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="p-5">
            <div className="flex items-center gap-3">
              <Link2 className="w-5 h-5 text-muted-foreground/50" strokeWidth={1.5} />
              <div>
                <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">已對應 Jira</p>
                <p className="font-display text-2xl text-foreground">
                  {stats.jira_mapping.percentage.toFixed(0)}%
                  <span className="text-sm text-muted-foreground ml-1">({stats.jira_mapping.mapped})</span>
                </p>
              </div>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="p-5">
            <div className="flex items-center gap-3">
              <CheckCircle2 className="w-5 h-5 text-muted-foreground/50" strokeWidth={1.5} />
              <div>
                <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">已同步 Tempo</p>
                <p className="font-display text-2xl text-foreground">
                  {stats.tempo_sync.percentage.toFixed(0)}%
                  <span className="text-sm text-muted-foreground ml-1">({stats.tempo_sync.synced})</span>
                </p>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </section>
  )
}
