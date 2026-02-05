/**
 * AccountCard component
 *
 * Displays account information including email, display name,
 * subscription plan, and subscription status.
 */

import { User, Mail, Crown, CheckCircle2, XCircle } from 'lucide-react'
import { Card, CardContent } from '@/components/ui/card'
import type { AccountInfo } from '@/types/quota'

interface AccountCardProps {
  accountInfo: AccountInfo
}

export function AccountCard({ accountInfo }: AccountCardProps) {
  // Format plan name for display
  const formatPlan = (plan: string | null) => {
    if (!plan) return 'Free'
    // Handle both "claude_max" and "max" formats
    const planLower = plan.toLowerCase().replace('claude_', '')
    switch (planLower) {
      case 'max':
        return 'Claude Max'
      case 'pro':
        return 'Claude Pro'
      case 'team':
        return 'Claude Team'
      case 'enterprise':
        return 'Enterprise'
      default:
        return plan.charAt(0).toUpperCase() + plan.slice(1)
    }
  }

  // Get plan color
  const getPlanColor = (plan: string | null) => {
    if (!plan) return 'text-muted-foreground'
    const planLower = plan.toLowerCase().replace('claude_', '')
    switch (planLower) {
      case 'max':
        return 'text-purple-500'
      case 'pro':
        return 'text-blue-500'
      case 'team':
        return 'text-emerald-500'
      case 'enterprise':
        return 'text-amber-500'
      default:
        return 'text-muted-foreground'
    }
  }

  return (
    <Card className="border-l-2 border-l-purple-500/50">
      <CardContent className="pt-6">
        <div className="flex items-center justify-between flex-wrap gap-4">
          {/* Left: User info */}
          <div className="flex items-center gap-4">
            <div className="w-10 h-10 rounded-full bg-purple-500/10 flex items-center justify-center">
              <User className="w-5 h-5 text-purple-500" />
            </div>
            <div>
              {/* Display name */}
              {accountInfo.display_name && (
                <div className="text-lg font-semibold">{accountInfo.display_name}</div>
              )}
              {/* Email */}
              {accountInfo.email && (
                <div className="flex items-center gap-1.5 text-muted-foreground">
                  <Mail className="w-3.5 h-3.5" />
                  <span className="text-sm">{accountInfo.email}</span>
                </div>
              )}
            </div>
          </div>

          {/* Right: Plan and status */}
          <div className="flex items-center gap-6">
            {/* Plan */}
            <div className="flex items-center gap-2">
              <Crown className={`w-5 h-5 ${getPlanColor(accountInfo.plan)}`} />
              <span className={`text-base font-medium ${getPlanColor(accountInfo.plan)}`}>
                {formatPlan(accountInfo.plan)}
              </span>
            </div>

            {/* Status */}
            <div className="flex items-center gap-1.5">
              {accountInfo.is_active ? (
                <CheckCircle2 className="w-4 h-4 text-green-500" />
              ) : (
                <XCircle className="w-4 h-4 text-red-500" />
              )}
              <span className={`text-sm ${accountInfo.is_active ? 'text-green-600' : 'text-red-600'}`}>
                {accountInfo.is_active ? '訂閱有效' : '訂閱無效'}
              </span>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  )
}

export default AccountCard
