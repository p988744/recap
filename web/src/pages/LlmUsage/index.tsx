import { useState } from 'react'
import { RefreshCw } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useLlmUsage } from './hooks/useLlmUsage'
import { UsageSummary } from './components/UsageSummary'
import { DailyChart } from './components/DailyChart'
import { ModelBreakdown } from './components/ModelBreakdown'
import { UsageLogs } from './components/UsageLogs'

function getDateRange(days: number): { start: string; end: string } {
  const end = new Date()
  const start = new Date()
  start.setDate(end.getDate() - days + 1)
  return {
    start: start.toISOString().slice(0, 10),
    end: end.toISOString().slice(0, 10),
  }
}

const RANGE_OPTIONS = [
  { label: '7 天', days: 7 },
  { label: '30 天', days: 30 },
  { label: '90 天', days: 90 },
]

export function LlmUsagePage() {
  const [rangeDays, setRangeDays] = useState(30)
  const { start, end } = getDateRange(rangeDays)
  const { stats, daily, models, logs, loading, refresh } = useLlmUsage(start, end)

  return (
    <div className="space-y-10">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="font-display text-2xl text-foreground tracking-tight">LLM 用量</h1>
          <p className="text-sm text-muted-foreground mt-1">
            追蹤 LLM API 呼叫次數、Token 用量與預估費用
          </p>
        </div>
        <div className="flex items-center gap-2">
          {RANGE_OPTIONS.map((opt) => (
            <Button
              key={opt.days}
              variant={rangeDays === opt.days ? 'default' : 'outline'}
              size="sm"
              onClick={() => setRangeDays(opt.days)}
              className="text-xs"
            >
              {opt.label}
            </Button>
          ))}
          <Button
            variant="ghost"
            size="icon"
            onClick={refresh}
            disabled={loading}
            className="h-8 w-8"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${loading ? 'animate-spin' : ''}`} strokeWidth={1.5} />
          </Button>
        </div>
      </div>

      {/* Divider */}
      <div className="h-px bg-charcoal/6" />

      {/* Summary Cards */}
      <UsageSummary stats={stats} />

      {/* Charts Row */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
        <div className="lg:col-span-2">
          <DailyChart data={daily} />
        </div>
        <div>
          <ModelBreakdown data={models} />
        </div>
      </div>

      {/* Divider */}
      <div className="h-px bg-charcoal/6" />

      {/* Logs Table */}
      <UsageLogs logs={logs} />
    </div>
  )
}
