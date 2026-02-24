import { useState } from 'react'
import { List } from 'lucide-react'
import type { LlmUsageLog } from '@/types'

interface UsageLogsProps {
  logs: LlmUsageLog[]
}

function formatTokens(n: number | null): string {
  if (n == null) return '-'
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`
  return `${n}`
}

function formatCost(n: number | null): string {
  if (n == null) return '-'
  if (n < 0.01) return `$${n.toFixed(4)}`
  return `$${n.toFixed(2)}`
}

function formatDuration(ms: number | null): string {
  if (ms == null) return '-'
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
}

function formatPurpose(purpose: string): string {
  const map: Record<string, string> = {
    'hourly_compaction': '小時壓縮',
    'daily_compaction': '每日壓縮',
    'weekly_compaction': '每週壓縮',
    'monthly_compaction': '每月壓縮',
    'session_summary': 'Session 摘要',
    'project_summary': '專案摘要',
    'daily_summary': '每日摘要',
  }
  return map[purpose] || purpose
}

export function UsageLogs({ logs }: UsageLogsProps) {
  const [expandedId, setExpandedId] = useState<string | null>(null)

  return (
    <div>
      <div className="flex items-center gap-2 mb-6">
        <List className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
        <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
          最近呼叫記錄
        </h2>
      </div>

      {logs.length > 0 ? (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-border text-left">
                <th className="pb-2 pr-4 text-[10px] uppercase tracking-[0.15em] text-muted-foreground font-normal">時間</th>
                <th className="pb-2 pr-4 text-[10px] uppercase tracking-[0.15em] text-muted-foreground font-normal">用途</th>
                <th className="pb-2 pr-4 text-[10px] uppercase tracking-[0.15em] text-muted-foreground font-normal">模型</th>
                <th className="pb-2 pr-4 text-[10px] uppercase tracking-[0.15em] text-muted-foreground font-normal text-right">輸入</th>
                <th className="pb-2 pr-4 text-[10px] uppercase tracking-[0.15em] text-muted-foreground font-normal text-right">輸出</th>
                <th className="pb-2 pr-4 text-[10px] uppercase tracking-[0.15em] text-muted-foreground font-normal text-right">費用</th>
                <th className="pb-2 pr-4 text-[10px] uppercase tracking-[0.15em] text-muted-foreground font-normal text-right">耗時</th>
                <th className="pb-2 text-[10px] uppercase tracking-[0.15em] text-muted-foreground font-normal">狀態</th>
              </tr>
            </thead>
            <tbody>
              {logs.map((log) => (
                <>
                  <tr key={log.id} className="border-b border-border/50 hover:bg-muted/30">
                    <td className="py-2 pr-4 text-muted-foreground tabular-nums whitespace-nowrap">
                      {log.created_at.replace('T', ' ').slice(0, 16)}
                    </td>
                    <td className="py-2 pr-4">{formatPurpose(log.purpose)}</td>
                    <td className="py-2 pr-4 text-muted-foreground">{log.model}</td>
                    <td className="py-2 pr-4 text-right tabular-nums">{formatTokens(log.prompt_tokens)}</td>
                    <td className="py-2 pr-4 text-right tabular-nums">{formatTokens(log.completion_tokens)}</td>
                    <td className="py-2 pr-4 text-right tabular-nums">{formatCost(log.estimated_cost)}</td>
                    <td className="py-2 pr-4 text-right tabular-nums text-muted-foreground">{formatDuration(log.duration_ms)}</td>
                    <td className="py-2">
                      {log.status === 'success' ? (
                        <span className="text-green-600">OK</span>
                      ) : (
                        <button
                          onClick={() => setExpandedId(expandedId === log.id ? null : log.id)}
                          className="text-red-500 hover:text-red-400 cursor-pointer underline decoration-dotted underline-offset-2"
                        >
                          ERR
                        </button>
                      )}
                    </td>
                  </tr>
                  {expandedId === log.id && log.error_message && (
                    <tr key={`${log.id}-err`} className="border-b border-border/50">
                      <td colSpan={8} className="py-2 px-4">
                        <pre className="text-xs text-red-400 bg-red-500/5 border border-red-500/20 rounded px-3 py-2 whitespace-pre-wrap break-all max-h-40 overflow-auto font-mono">
                          {log.error_message}
                        </pre>
                      </td>
                    </tr>
                  )}
                </>
              ))}
            </tbody>
          </table>
        </div>
      ) : (
        <div className="h-32 flex items-center justify-center text-muted-foreground text-sm">
          暫無呼叫記錄
        </div>
      )}
    </div>
  )
}
