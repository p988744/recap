/**
 * ToolOverviewCard Component
 *
 * A card that displays tool overview including:
 * - Tool name and icon
 * - Account info (email, plan, status)
 * - Quota status (mini progress bars)
 * - Link to detail page
 */

import { Link } from 'react-router-dom'
import { Card, CardContent } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { ChevronRight, CheckCircle2, XCircle, Crown } from 'lucide-react'
import { QuotaMiniCard } from './QuotaMiniCard'
import { QuotaTool } from '../tools'
import { QuotaSnapshot, AccountInfo, QuotaSettings } from '@/types/quota'
import { cn } from '@/lib/utils'

interface ToolOverviewCardProps {
  tool: QuotaTool
  accountInfo?: AccountInfo | null
  snapshots?: QuotaSnapshot[]
  settings: QuotaSettings
}

// Format plan name for display
function formatPlan(plan: string | null): string {
  if (!plan) return 'Free'
  const planLower = plan.toLowerCase().replace('claude_', '')
  switch (planLower) {
    case 'max':
      return 'Max'
    case 'pro':
      return 'Pro'
    case 'team':
      return 'Team'
    case 'enterprise':
      return 'Enterprise'
    default:
      return plan.charAt(0).toUpperCase() + plan.slice(1)
  }
}

// Get plan badge color
function getPlanBadgeClass(plan: string | null): string {
  if (!plan) return 'bg-muted text-muted-foreground'
  const planLower = plan.toLowerCase().replace('claude_', '')
  switch (planLower) {
    case 'max':
      return 'bg-purple-500/10 text-purple-600 dark:text-purple-400'
    case 'pro':
      return 'bg-blue-500/10 text-blue-600 dark:text-blue-400'
    case 'team':
      return 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
    case 'enterprise':
      return 'bg-amber-500/10 text-amber-600 dark:text-amber-400'
    default:
      return 'bg-muted text-muted-foreground'
  }
}

export function ToolOverviewCard({
  tool,
  accountInfo,
  snapshots = [],
  settings,
}: ToolOverviewCardProps) {
  const Icon = tool.icon

  // Filter snapshots for main display (5_hour and 7_day)
  const primarySnapshots = snapshots.filter(
    (s) => s.window_type === '5_hour' || s.window_type === '7_day'
  )

  // Disabled tool state
  if (tool.disabled) {
    return (
      <Card className="border-dashed opacity-60">
        <CardContent className="pt-6">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-lg bg-muted/50 flex items-center justify-center">
                <Icon className="w-5 h-5 text-muted-foreground" />
              </div>
              <div>
                <h3 className="font-medium text-muted-foreground">
                  {tool.name}
                </h3>
                <p className="text-xs text-muted-foreground/70">
                  {tool.description}
                </p>
              </div>
            </div>
            <Badge variant="outline" className="text-xs">
              即將推出
            </Badge>
          </div>
        </CardContent>
      </Card>
    )
  }

  return (
    <Card className="hover:border-primary/30 transition-colors">
      <CardContent className="pt-6 space-y-4">
        {/* Header: Tool name and icon */}
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center">
            <Icon className="w-5 h-5 text-primary" />
          </div>
          <div>
            <h3 className="font-medium">{tool.name}</h3>
            <p className="text-xs text-muted-foreground">{tool.description}</p>
          </div>
        </div>

        {/* Account info (simplified) */}
        {accountInfo && (
          <div className="flex items-center gap-2 text-sm">
            <span className="text-muted-foreground truncate max-w-[180px]">
              {accountInfo.email || accountInfo.display_name || '-'}
            </span>
            <span className="text-muted-foreground/50">·</span>
            <span
              className={cn(
                'inline-flex items-center gap-1',
                getPlanBadgeClass(accountInfo.plan)
              )}
            >
              <Crown className="w-3 h-3" />
              {formatPlan(accountInfo.plan)}
            </span>
            <span className="text-muted-foreground/50">·</span>
            {accountInfo.is_active ? (
              <span className="inline-flex items-center gap-1 text-green-600 dark:text-green-500">
                <CheckCircle2 className="w-3 h-3" />
                有效
              </span>
            ) : (
              <span className="inline-flex items-center gap-1 text-red-600 dark:text-red-500">
                <XCircle className="w-3 h-3" />
                無效
              </span>
            )}
          </div>
        )}

        {/* Quota mini cards */}
        {primarySnapshots.length > 0 && (
          <div className="grid grid-cols-2 gap-4">
            {primarySnapshots.map((snapshot) => (
              <QuotaMiniCard
                key={snapshot.window_type}
                snapshot={snapshot}
                settings={settings}
              />
            ))}
          </div>
        )}

        {/* Link to detail page */}
        <Link
          to={`/quota/${tool.id}`}
          className="flex items-center justify-between py-2 px-3 -mx-3 rounded-md hover:bg-muted/50 transition-colors group"
        >
          <span className="text-sm text-muted-foreground group-hover:text-foreground transition-colors">
            檢視費用與歷史用量
          </span>
          <ChevronRight className="w-4 h-4 text-muted-foreground group-hover:text-foreground group-hover:translate-x-0.5 transition-all" />
        </Link>
      </CardContent>
    </Card>
  )
}

export default ToolOverviewCard
