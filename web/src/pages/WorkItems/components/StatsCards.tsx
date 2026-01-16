import { Card, CardContent } from '@/components/ui/card'
import type { WorkItemStatsResponse } from '@/types'

interface StatsCardsProps {
  stats: WorkItemStatsResponse
}

export function StatsCards({ stats }: StatsCardsProps) {
  return (
    <section className="grid grid-cols-4 gap-4 animate-fade-up opacity-0 delay-2">
      <Card className="border-l-2 border-l-warm/60">
        <CardContent className="p-5">
          <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-1">總項目數</p>
          <p className="font-display text-3xl text-foreground">{stats.total_items}</p>
        </CardContent>
      </Card>
      <Card className="border-l-2 border-l-warm/60">
        <CardContent className="p-5">
          <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-1">總工時</p>
          <p className="font-display text-3xl text-foreground">
            {stats.total_hours.toFixed(1)}
            <span className="text-base text-muted-foreground ml-1">hrs</span>
          </p>
        </CardContent>
      </Card>
      <Card className="border-l-2 border-l-sage/60">
        <CardContent className="p-5">
          <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-1">已對應 Jira</p>
          <p className="font-display text-3xl text-foreground">
            {stats.jira_mapping.percentage.toFixed(0)}%
            <span className="text-sm text-muted-foreground ml-1">({stats.jira_mapping.mapped})</span>
          </p>
        </CardContent>
      </Card>
      <Card className="border-l-2 border-l-sage/60">
        <CardContent className="p-5">
          <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-1">已同步 Tempo</p>
          <p className="font-display text-3xl text-foreground">
            {stats.tempo_sync.percentage.toFixed(0)}%
            <span className="text-sm text-muted-foreground ml-1">({stats.tempo_sync.synced})</span>
          </p>
        </CardContent>
      </Card>
    </section>
  )
}
