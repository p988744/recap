/**
 * Quota Page (Overview)
 *
 * Overview page showing all quota tools with their status.
 * Each tool card shows account info, current quota, and links to detail page.
 */

import { Button } from '@/components/ui/button'
import { RefreshCw, Gauge } from 'lucide-react'
import { useQuotaPage, DEFAULT_QUOTA_SETTINGS } from './hooks'
import { ClaudeAuthConfig, ToolOverviewCard } from './components'
import { QUOTA_TOOLS, TOOL_IDS } from './tools'
import { cn } from '@/lib/utils'

export function QuotaPage() {
  const {
    currentQuota,
    providerAvailable,
    accountInfo,
    loading,
    error,
    refresh,
  } = useQuotaPage()

  // If provider not available, show auth config
  if (!providerAvailable) {
    return (
      <div className="space-y-6">
        <header className="animate-fade-up opacity-0 delay-1">
          <div className="flex items-center gap-2 mb-2">
            <Gauge
              className="w-4 h-4 text-muted-foreground"
              strokeWidth={1.5}
            />
            <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
              配額
            </p>
          </div>
          <h1 className="font-display text-3xl text-foreground tracking-tight">
            配額使用量
          </h1>
        </header>

        {/* Tool cards - show disabled state for all */}
        <div className="space-y-4 animate-fade-up opacity-0 delay-2">
          {TOOL_IDS.map((toolId) => {
            const tool = QUOTA_TOOLS[toolId]
            return (
              <ToolOverviewCard
                key={toolId}
                tool={{ ...tool, disabled: true }}
                settings={DEFAULT_QUOTA_SETTINGS}
              />
            )
          })}
        </div>

        {/* Manual OAuth token configuration */}
        <ClaudeAuthConfig onAuthStatusChange={refresh} />
      </div>
    )
  }

  return (
    <div className="space-y-8">
      {/* Header */}
      <header className="animate-fade-up opacity-0 delay-1">
        <div className="flex items-center justify-between">
          <div>
            <div className="flex items-center gap-2 mb-2">
              <Gauge
                className="w-4 h-4 text-muted-foreground"
                strokeWidth={1.5}
              />
              <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
                配額
              </p>
            </div>
            <h1 className="font-display text-3xl text-foreground tracking-tight">
              配額使用量
            </h1>
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={refresh}
            disabled={loading}
            className="gap-2"
          >
            <RefreshCw className={cn('h-4 w-4', loading && 'animate-spin')} />
            重新整理
          </Button>
        </div>
      </header>

      {/* Error display */}
      {error && (
        <div className="p-4 border-l-2 border-l-destructive bg-destructive/5 text-destructive text-sm animate-fade-up">
          {error}
        </div>
      )}

      {/* Loading state */}
      {loading && currentQuota.length === 0 && (
        <div className="flex items-center justify-center h-48">
          <div className="w-6 h-6 border border-border border-t-foreground/60 rounded-full animate-spin" />
        </div>
      )}

      {/* Tool Cards */}
      <div className="space-y-4 animate-fade-up opacity-0 delay-2">
        {TOOL_IDS.map((toolId) => {
          const tool = QUOTA_TOOLS[toolId]
          // Filter snapshots for this tool
          const toolSnapshots = currentQuota.filter(
            (s) => s.provider === toolId
          )
          // Account info is only available for claude currently
          const toolAccountInfo = toolId === 'claude' ? accountInfo : null

          return (
            <ToolOverviewCard
              key={toolId}
              tool={tool}
              accountInfo={toolAccountInfo}
              snapshots={toolSnapshots}
              settings={DEFAULT_QUOTA_SETTINGS}
            />
          )
        })}
      </div>
    </div>
  )
}

export default QuotaPage

// Re-export ToolDetailPage for routing
export { ToolDetailPage } from './ToolDetailPage'
