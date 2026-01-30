import { Activity, Coins, Hash, Clock } from 'lucide-react'
import type { LlmUsageStats } from '@/types'

interface UsageSummaryProps {
  stats: LlmUsageStats | null
}

function formatNumber(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`
  return n.toFixed(0)
}

function formatCost(n: number): string {
  if (n < 0.01) return `$${n.toFixed(4)}`
  return `$${n.toFixed(2)}`
}

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms.toFixed(0)}ms`
  return `${(ms / 1000).toFixed(1)}s`
}

export function UsageSummary({ stats }: UsageSummaryProps) {
  const cards = [
    {
      icon: Activity,
      label: 'API 呼叫次數',
      value: stats ? `${stats.total_calls}` : '-',
      sub: stats ? `成功 ${stats.success_calls} / 失敗 ${stats.error_calls}` : '',
    },
    {
      icon: Hash,
      label: '總 Token 數',
      value: stats ? formatNumber(stats.total_tokens) : '-',
      sub: stats
        ? `輸入 ${formatNumber(stats.total_prompt_tokens)} / 輸出 ${formatNumber(stats.total_completion_tokens)}`
        : '',
    },
    {
      icon: Coins,
      label: '預估費用',
      value: stats ? formatCost(stats.total_cost) : '-',
      sub: stats && stats.total_calls > 0
        ? `平均 ${formatCost(stats.total_cost / stats.total_calls)} / 次`
        : '',
    },
    {
      icon: Clock,
      label: '平均回應時間',
      value: stats ? formatDuration(stats.avg_duration_ms) : '-',
      sub: stats && stats.total_calls > 0
        ? `平均 ${formatNumber(stats.avg_tokens_per_call)} tokens / 次`
        : '',
    },
  ]

  return (
    <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
      {cards.map((card) => (
        <div key={card.label} className="p-4 border border-border bg-white/40">
          <div className="flex items-center gap-2 mb-3">
            <card.icon className="w-3.5 h-3.5 text-muted-foreground" strokeWidth={1.5} />
            <span className="text-[10px] uppercase tracking-[0.15em] text-muted-foreground">
              {card.label}
            </span>
          </div>
          <p className="text-xl font-display text-foreground tabular-nums">{card.value}</p>
          {card.sub && (
            <p className="text-[11px] text-muted-foreground mt-1">{card.sub}</p>
          )}
        </div>
      ))}
    </div>
  )
}
