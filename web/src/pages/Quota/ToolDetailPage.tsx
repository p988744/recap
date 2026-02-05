/**
 * ToolDetailPage Component
 *
 * Detail page for a specific quota tool showing:
 * - Cost statistics and chart
 * - Usage history chart
 */

import { useParams, Link, Navigate } from 'react-router-dom'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { ArrowLeft, RefreshCw, History, DollarSign } from 'lucide-react'
import { useToolDetail, DEFAULT_QUOTA_SETTINGS } from './hooks'
import {
  CostCard,
  CostChart,
  QuotaChart,
  QuotaStats,
  ClaudeAuthConfig,
  CostSkeleton,
  HistorySkeleton,
} from './components'
import { getToolById } from './tools'
import { cn } from '@/lib/utils'

export function ToolDetailPage() {
  const { toolId } = useParams<{ toolId: string }>()

  // Early return for invalid toolId
  if (!toolId) {
    return <Navigate to="/quota" replace />
  }

  const tool = getToolById(toolId)

  // Unknown tool - redirect to overview
  if (!tool) {
    return <Navigate to="/quota" replace />
  }

  // Disabled tool - redirect to overview
  if (tool.disabled) {
    return <Navigate to="/quota" replace />
  }

  return <ToolDetailContent toolId={toolId} />
}

// Separate component to use hook after validation
function ToolDetailContent({ toolId }: { toolId: string }) {
  const {
    tool,
    snapshots,
    providerAvailable,
    history,
    costSummary,
    loading,
    error,
    days,
    setDays,
    refresh,
  } = useToolDetail(toolId)

  const Icon = tool?.icon

  // Auth not available - show auth config
  if (!providerAvailable) {
    return (
      <div className="space-y-6">
        {/* Header */}
        <header className="animate-fade-up opacity-0 delay-1">
          <div className="flex items-center gap-4 mb-4">
            <Button variant="ghost" size="sm" asChild className="-ml-2">
              <Link to="/quota">
                <ArrowLeft className="w-4 h-4 mr-1" />
                返回
              </Link>
            </Button>
          </div>
          <div className="flex items-center gap-3">
            {Icon && (
              <div className="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center">
                <Icon className="w-5 h-5 text-primary" />
              </div>
            )}
            <h1 className="font-display text-2xl text-foreground tracking-tight">
              {tool?.name}
            </h1>
          </div>
        </header>

        <Card className="border-l-2 border-l-muted">
          <CardContent className="pt-6 space-y-3">
            <p className="text-muted-foreground">
              尚未設定 Claude Code OAuth 認證。
            </p>
            <p className="text-sm text-muted-foreground">
              此功能需要 Claude Max 訂閱用戶的 OAuth token。請在終端機執行{' '}
              <code className="bg-muted px-1.5 py-0.5 rounded text-foreground">
                claude /login
              </code>{' '}
              進行認證，或在下方手動輸入 Token。
            </p>
          </CardContent>
        </Card>

        <ClaudeAuthConfig onAuthStatusChange={refresh} />
      </div>
    )
  }

  // Determine loading states for each section
  const isCostLoading = loading && !costSummary
  const isHistoryLoading = loading && history.length === 0

  return (
    <div className="space-y-8">
      {/* Header */}
      <header className="animate-fade-up opacity-0 delay-1">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <Button variant="ghost" size="sm" asChild className="-ml-2">
              <Link to="/quota">
                <ArrowLeft className="w-4 h-4 mr-1" />
                返回
              </Link>
            </Button>
            {Icon && (
              <div className="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center">
                <Icon className="w-5 h-5 text-primary" />
              </div>
            )}
            <h1 className="font-display text-2xl text-foreground tracking-tight">
              {tool?.name}
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

      {/* Cost Summary Section */}
      {tool?.hasCost && (
        <>
          {isCostLoading ? (
            <CostSkeleton />
          ) : costSummary ? (
            <section className="animate-fade-up opacity-0 delay-2">
              <div className="flex items-center gap-2 mb-4">
                <DollarSign
                  className="w-4 h-4 text-muted-foreground"
                  strokeWidth={1.5}
                />
                <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
                  費用統計
                </h2>
              </div>

              <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
                {/* Cost Card */}
                <CostCard costSummary={costSummary} />

                {/* Cost Chart */}
                <Card>
                  <CardHeader className="pb-2">
                    <CardTitle className="text-xs font-medium text-muted-foreground">
                      每日費用趨勢 (30天)
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <CostChart costSummary={costSummary} />
                  </CardContent>
                </Card>
              </div>
            </section>
          ) : null}
        </>
      )}

      {/* History Chart Section */}
      {isHistoryLoading ? (
        <HistorySkeleton />
      ) : (
        <section className="animate-fade-up opacity-0 delay-3">
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between flex-wrap gap-4">
                <div className="flex items-center gap-2">
                  <History
                    className="w-4 h-4 text-muted-foreground"
                    strokeWidth={1.5}
                  />
                  <CardTitle className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground font-normal">
                    使用歷史
                  </CardTitle>
                </div>
                <div className="flex gap-2 flex-wrap">
                  {/* Days filter */}
                  <Select
                    value={days.toString()}
                    onValueChange={(v) => setDays(parseInt(v, 10))}
                  >
                    <SelectTrigger className="w-28 h-8 text-xs">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="1">1 天</SelectItem>
                      <SelectItem value="3">3 天</SelectItem>
                      <SelectItem value="7">7 天</SelectItem>
                      <SelectItem value="14">14 天</SelectItem>
                      <SelectItem value="30">30 天</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>
            </CardHeader>
            <CardContent className="space-y-4">
              {/* Statistics */}
              {snapshots.length > 0 && (
                <QuotaStats currentQuota={snapshots} historyData={history} />
              )}

              {/* Chart */}
              {history.length > 0 ? (
                <QuotaChart data={history} settings={DEFAULT_QUOTA_SETTINGS} />
              ) : (
                <div className="flex flex-col items-center justify-center h-[300px] text-muted-foreground">
                  <History className="w-8 h-8 mb-2 opacity-50" />
                  <p>尚無歷史資料</p>
                  <p className="text-xs mt-1">開始追蹤後，配額資料將顯示在此</p>
                </div>
              )}
            </CardContent>
          </Card>
        </section>
      )}
    </div>
  )
}

export default ToolDetailPage
