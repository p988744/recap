/**
 * CostCard component
 *
 * Displays cost summary information including today's cost,
 * 30-day total, and model breakdown.
 */

import { DollarSign, Coins, Cpu } from 'lucide-react'
import { Card, CardContent } from '@/components/ui/card'
import type { CostSummary } from '@/types/quota'
import { formatCost, formatTokens } from '@/types/quota'

interface CostCardProps {
  costSummary: CostSummary
}

export function CostCard({ costSummary }: CostCardProps) {
  // Get top 3 models by cost
  const topModels = costSummary.model_breakdown.slice(0, 3)

  // Format model name for display
  const formatModelName = (model: string) => {
    if (model.includes('opus-4-5') || model.includes('opus-4')) return 'Opus 4.5'
    if (model.includes('opus-3') || model.includes('opus-20240229')) return 'Opus 3'
    if (model.includes('sonnet-4') || model.includes('sonnet-20250514')) return 'Sonnet 4'
    if (model.includes('sonnet-3-5') || model.includes('sonnet-20241022')) return 'Sonnet 3.5'
    if (model.includes('haiku')) return 'Haiku'
    // Fallback: extract version from model name
    const match = model.match(/claude[- ]?([\w.-]+)/i)
    return match ? match[1] : model
  }

  return (
    <Card className="border-l-2 border-l-amber-500/50">
      <CardContent className="pt-6">
        <div className="flex items-start justify-between">
          {/* Left side - Cost summary */}
          <div className="space-y-4">
            {/* Today */}
            <div>
              <div className="flex items-center gap-2 text-muted-foreground mb-1">
                <DollarSign className="w-4 h-4" />
                <span className="text-xs">今日費用</span>
              </div>
              <div className="flex items-baseline gap-2">
                <span className="text-2xl font-semibold tabular-nums">
                  {formatCost(costSummary.today_cost)}
                </span>
                <span className="text-xs text-muted-foreground">
                  {formatTokens(costSummary.today_tokens)} tokens
                </span>
              </div>
            </div>

            {/* 30 days */}
            <div>
              <div className="flex items-center gap-2 text-muted-foreground mb-1">
                <Coins className="w-4 h-4" />
                <span className="text-xs">30 天累計</span>
              </div>
              <div className="flex items-baseline gap-2">
                <span className="text-2xl font-semibold tabular-nums">
                  {formatCost(costSummary.last_30_days_cost)}
                </span>
                <span className="text-xs text-muted-foreground">
                  {formatTokens(costSummary.last_30_days_tokens)} tokens
                </span>
              </div>
            </div>
          </div>

          {/* Right side - Model breakdown */}
          <div className="text-right">
            <div className="flex items-center gap-2 text-muted-foreground mb-2 justify-end">
              <Cpu className="w-4 h-4" />
              <span className="text-xs">模型用量 (30天)</span>
            </div>
            <div className="space-y-1.5">
              {topModels.map((model) => (
                <div key={model.model} className="flex items-center justify-end gap-3">
                  <span className="text-xs text-muted-foreground">
                    {formatModelName(model.model)}
                  </span>
                  <span className="text-sm font-medium tabular-nums w-16 text-right">
                    {formatCost(model.total_cost)}
                  </span>
                </div>
              ))}
              {costSummary.model_breakdown.length > 3 && (
                <div className="text-[10px] text-muted-foreground">
                  +{costSummary.model_breakdown.length - 3} 其他模型
                </div>
              )}
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  )
}

export default CostCard
