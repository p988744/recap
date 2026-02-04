import { useCallback, useState } from 'react'
import { ChevronDown, ChevronRight, Loader2, CheckCircle2, AlertCircle } from 'lucide-react'
import { Card } from '@/components/ui/card'
import type { DataSourceAdapter, DiscoveredProject, SyncResult } from './types'

interface DataSourceCardProps {
  adapter: DataSourceAdapter
}

const MAX_PREVIEW_SESSIONS = 3

export function DataSourceCard({ adapter }: DataSourceCardProps) {
  const [projects, setProjects] = useState<DiscoveredProject[]>([])
  const [installed, setInstalled] = useState<boolean | null>(null)
  const [scanning, setScanning] = useState(false)
  const [syncing, setSyncing] = useState(false)
  const [expanded, setExpanded] = useState<Set<string>>(new Set())
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null)
  const [scanned, setScanned] = useState(false)

  const totalSessions = projects.reduce((sum, p) => sum + p.sessionCount, 0)

  const handleScan = useCallback(async () => {
    setScanning(true)
    setMessage(null)
    try {
      const isInstalled = await adapter.checkInstalled()
      setInstalled(isInstalled)
      if (!isInstalled) {
        setProjects([])
        setScanned(true)
        setScanning(false)
        return
      }
      const discovered = await adapter.scanProjects()
      setProjects(discovered)
      setScanned(true)
    } catch (err) {
      setMessage({ type: 'error', text: `掃描失敗：${String(err)}` })
    } finally {
      setScanning(false)
    }
  }, [adapter])

  const handleSync = useCallback(async () => {
    setSyncing(true)
    setMessage(null)
    try {
      const result: SyncResult = await adapter.syncAll()
      setMessage({
        type: 'success',
        text: `同步完成：${result.workItemsCreated} 個新增、${result.workItemsUpdated} 個更新（處理 ${result.sessionsProcessed} 個 session，略過 ${result.sessionsSkipped} 個）`,
      })
    } catch (err) {
      setMessage({ type: 'error', text: `同步失敗：${String(err)}` })
    } finally {
      setSyncing(false)
    }
  }, [adapter])

  const toggleExpand = useCallback((name: string) => {
    setExpanded((prev) => {
      const next = new Set(prev)
      if (next.has(name)) next.delete(name)
      else next.add(name)
      return next
    })
  }, [])

  return (
    <Card className="p-4">
      {/* Header */}
      <div className="flex items-center gap-2.5">
        <span className={adapter.colorClass}>{adapter.icon}</span>
        <span className="text-sm font-medium text-foreground">{adapter.label}</span>
        <div className="ml-auto">
          <button
            onClick={handleScan}
            disabled={scanning}
            className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-md border border-border text-foreground hover:bg-foreground/5 transition-colors disabled:opacity-50"
          >
            {scanning && <Loader2 className="w-3 h-3 animate-spin" />}
            掃描
          </button>
        </div>
      </div>

      {/* Scanned content */}
      {scanned && (
        <div className="mt-3 space-y-3">
          {/* Not installed */}
          {installed === false && (
            <p className="text-xs text-muted-foreground/60 italic">
              未偵測到 {adapter.label}。
            </p>
          )}

          {/* Installed but no projects */}
          {installed === true && projects.length === 0 && (
            <p className="text-xs text-muted-foreground/60 italic">
              已偵測到 {adapter.label}，但未發現任何專案 session。
            </p>
          )}

          {/* Discovered projects */}
          {projects.length > 0 && (
            <>
              <p className="text-xs text-muted-foreground">
                已偵測 {projects.length} 個專案，{totalSessions} 個 session
              </p>

              <div className="space-y-1">
                {projects.map((project) => {
                  const isExpanded = expanded.has(project.name)
                  const previewSessions = project.sessions.slice(0, MAX_PREVIEW_SESSIONS)
                  const remaining = project.sessions.length - MAX_PREVIEW_SESSIONS
                  return (
                    <div key={project.name}>
                      <button
                        onClick={() => toggleExpand(project.name)}
                        className="w-full flex items-center gap-2 px-2 py-1.5 text-left rounded hover:bg-foreground/5 transition-colors"
                      >
                        {isExpanded ? (
                          <ChevronDown className="w-3 h-3 text-muted-foreground shrink-0" />
                        ) : (
                          <ChevronRight className="w-3 h-3 text-muted-foreground shrink-0" />
                        )}
                        <span className="text-xs font-medium text-foreground truncate">
                          {project.name}
                        </span>
                        <span className="text-[10px] text-muted-foreground/60 ml-auto shrink-0">
                          {project.sessionCount} sessions
                        </span>
                      </button>
                      {isExpanded && (
                        <div className="ml-7 space-y-0.5 pb-1">
                          {previewSessions.map((s) => (
                            <div key={s.id} className="flex items-start gap-2 px-2 py-1">
                              <span className="w-1 h-1 rounded-full bg-muted-foreground/30 mt-1.5 shrink-0" />
                              <div className="min-w-0 flex-1">
                                <p className="text-[11px] text-muted-foreground truncate">
                                  {s.summary}
                                </p>
                                {s.detail && (
                                  <p className="text-[10px] text-muted-foreground/50">
                                    {s.detail}
                                  </p>
                                )}
                              </div>
                            </div>
                          ))}
                          {remaining > 0 && (
                            <p className="text-[10px] text-muted-foreground/40 px-2 py-0.5">
                              +{remaining} more
                            </p>
                          )}
                        </div>
                      )}
                    </div>
                  )
                })}
              </div>

              {/* Sync button */}
              <button
                onClick={handleSync}
                disabled={syncing}
                className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-md bg-foreground text-background hover:bg-foreground/90 transition-colors disabled:opacity-50"
              >
                {syncing && <Loader2 className="w-3 h-3 animate-spin" />}
                同步 {totalSessions} 個 session 為工作項目
              </button>
            </>
          )}

          {/* Message */}
          {message && (
            <div className={`flex items-start gap-2 text-xs px-2 py-2 rounded ${
              message.type === 'success'
                ? 'bg-emerald-50 text-emerald-700 dark:bg-emerald-900/20 dark:text-emerald-400'
                : 'bg-red-50 text-red-700 dark:bg-red-900/20 dark:text-red-400'
            }`}>
              {message.type === 'success' ? (
                <CheckCircle2 className="w-3.5 h-3.5 shrink-0 mt-0.5" />
              ) : (
                <AlertCircle className="w-3.5 h-3.5 shrink-0 mt-0.5" />
              )}
              <span>{message.text}</span>
            </div>
          )}
        </div>
      )}
    </Card>
  )
}
