import { useCallback, useEffect, useState } from 'react'
import { CheckCircle2, XCircle, RefreshCw, Loader2 } from 'lucide-react'
import { Card } from '@/components/ui/card'
import { antigravity } from '@/services/integrations'
import type { AntigravityApiStatus } from '@/types'
import { GeminiIcon } from './icons/GeminiIcon'

export function AntigravityPathSetting() {
  const [status, setStatus] = useState<AntigravityApiStatus | null>(null)
  const [loading, setLoading] = useState(true)
  const [checking, setChecking] = useState(false)

  const checkStatus = useCallback(async () => {
    try {
      setChecking(true)
      const data = await antigravity.checkApiStatus()
      setStatus(data)
    } catch (err) {
      console.error('Failed to check Antigravity API status:', err)
      setStatus({
        running: false,
        healthy: false,
      })
    } finally {
      setLoading(false)
      setChecking(false)
    }
  }, [])

  useEffect(() => {
    checkStatus()
  }, [checkStatus])

  if (loading) {
    return (
      <Card className="p-4">
        <div className="flex items-center gap-2.5">
          <GeminiIcon className="w-4 h-4 text-blue-500" />
          <span className="text-sm font-medium text-foreground">Antigravity API</span>
          <Loader2 className="w-3.5 h-3.5 animate-spin text-muted-foreground ml-auto" />
        </div>
      </Card>
    )
  }

  const isHealthy = status?.running && status?.healthy

  return (
    <Card className="p-4">
      <div className="flex items-center gap-2.5 mb-2">
        <GeminiIcon className="w-4 h-4 text-blue-500" />
        <span className="text-sm font-medium text-foreground">Antigravity API</span>
        {isHealthy ? (
          <span className="inline-flex items-center gap-1 text-[10px] px-1.5 py-0.5 rounded bg-sage/10 text-sage">
            <CheckCircle2 className="w-2.5 h-2.5" />
            連線正常
          </span>
        ) : (
          <span className="inline-flex items-center gap-1 text-[10px] px-1.5 py-0.5 rounded bg-destructive/10 text-destructive">
            <XCircle className="w-2.5 h-2.5" />
            未連線
          </span>
        )}
        <button
          onClick={checkStatus}
          disabled={checking}
          className="ml-auto p-1 rounded hover:bg-foreground/5 transition-colors disabled:opacity-50"
          title="重新檢查"
        >
          <RefreshCw className={`w-3.5 h-3.5 text-muted-foreground ${checking ? 'animate-spin' : ''}`} />
        </button>
      </div>

      <div className="ml-[26px] space-y-1.5">
        {isHealthy && status?.api_url ? (
          <>
            <div className="flex items-center gap-2">
              <span className="text-[10px] text-muted-foreground/60 w-16">API 端點</span>
              <code className="text-xs text-foreground font-mono bg-foreground/5 px-1.5 py-0.5 rounded">
                {status.api_url}
              </code>
            </div>
            {status.session_count !== undefined && (
              <div className="flex items-center gap-2">
                <span className="text-[10px] text-muted-foreground/60 w-16">Session 數</span>
                <span className="text-xs text-foreground">{status.session_count}</span>
              </div>
            )}
          </>
        ) : (
          <p className="text-[10px] text-muted-foreground/50">
            請先開啟 Antigravity 應用程式，才能取得 API 資料並同步工作記錄。
          </p>
        )}
      </div>
    </Card>
  )
}
