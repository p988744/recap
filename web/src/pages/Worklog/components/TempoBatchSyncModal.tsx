import { useState, useEffect, useCallback } from 'react'
import { Check, AlertCircle, Loader2, Sparkles } from 'lucide-react'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { tempo } from '@/services'
import * as jiraIssueCache from '@/services/jiraIssueCache'
import { IssueKeyCombobox } from './IssueKeyCombobox'
import type { BatchSyncRow, SyncWorklogsResponse } from '@/types'

interface TempoBatchSyncModalProps {
  open: boolean
  date: string
  weekday: string
  initialRows: BatchSyncRow[]
  syncing: boolean
  syncResult: SyncWorklogsResponse | null
  onSync: (rows: BatchSyncRow[], dryRun: boolean) => Promise<SyncWorklogsResponse | null>
  onClose: () => void
}

type ValidationState = Record<string, { valid: boolean | null; summary: string; loading: boolean }>

export function TempoBatchSyncModal({
  open,
  date,
  weekday,
  initialRows,
  syncing,
  syncResult,
  onSync,
  onClose,
}: TempoBatchSyncModalProps) {
  const [rows, setRows] = useState<BatchSyncRow[]>([])
  const [validation, setValidation] = useState<ValidationState>({})
  const [summarizing, setSummarizing] = useState(false)
  const [summarizeLog, setSummarizeLog] = useState<string[]>([])

  // Initialize rows when modal opens
  useEffect(() => {
    if (open) {
      setRows(initialRows)
      setValidation({})
      setSummarizeLog([])
    }
  }, [open, initialRows])

  const updateRow = useCallback((index: number, field: keyof BatchSyncRow, value: string | number) => {
    setRows((prev) =>
      prev.map((r, i) => (i === index ? { ...r, [field]: value } : r)),
    )
    // Clear validation when issue key changes
    if (field === 'issueKey') {
      setValidation((prev) => {
        const next = { ...prev }
        delete next[`${index}`]
        return next
      })
    }
  }, [])

  const validateIssue = useCallback(async (index: number) => {
    const key = rows[index]?.issueKey.trim()
    if (!key) return

    setValidation((prev) => ({
      ...prev,
      [`${index}`]: { valid: null, summary: '', loading: true },
    }))

    try {
      const result = await tempo.validateIssue(key)
      setValidation((prev) => ({
        ...prev,
        [`${index}`]: {
          valid: result.valid,
          summary: result.valid ? (result.summary ?? '') : result.message,
          loading: false,
        },
      }))
    } catch (err) {
      console.error('Issue validation error:', key, err)
      setValidation((prev) => ({
        ...prev,
        [`${index}`]: { valid: false, summary: String(err), loading: false },
      }))
    }
  }, [rows])

  const handleAction = useCallback(async (dryRun: boolean) => {
    const filled = rows.filter((r) => r.issueKey.trim() !== '')
    if (filled.length === 0) return

    // Step 1: Summarize descriptions via LLM
    setSummarizing(true)
    setSummarizeLog([`Summarizing ${filled.length} descriptions with LLM...`])
    const summarizedRows = [...rows]
    let successCount = 0
    let fallbackCount = 0

    for (let i = 0; i < summarizedRows.length; i++) {
      const row = summarizedRows[i]
      if (!row.issueKey.trim() || !row.description.trim()) continue
      try {
        const summary = await tempo.summarizeDescription(row.description)
        summarizedRows[i] = { ...row, description: summary }
        successCount++
        setSummarizeLog(prev => [...prev, `✓ ${row.projectName}: "${summary}"`])
      } catch {
        fallbackCount++
        setSummarizeLog(prev => [...prev, `⚠ ${row.projectName}: fallback`])
      }
    }

    setSummarizeLog(prev => [...prev,
      `Done: ${successCount} summarized, ${fallbackCount} fallback`,
      dryRun ? 'Generating preview...' : 'Uploading to Tempo...',
    ])
    setSummarizing(false)

    // Step 2: Send to sync
    await onSync(summarizedRows, dryRun)
  }, [rows, onSync])

  const totalHours = rows.reduce((sum, r) => sum + r.hours, 0)
  const filledRows = rows.filter((r) => r.issueKey.trim() !== '')
  const canSync = filledRows.length > 0 && !syncing && !summarizing

  const showResult = syncResult !== null

  return (
    <Dialog open={open} onOpenChange={(o) => { if (!o) onClose() }}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>
            Export Day: {date.slice(5).replace('-', '/')} ({weekday})
          </DialogTitle>
          <DialogDescription className="sr-only">
            Export worklog entries for the selected day to Tempo
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-2">
          {/* Table */}
          <div className="border rounded-md overflow-hidden">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b bg-muted/50">
                  <th className="text-left px-3 py-2 font-medium">Project</th>
                  <th className="text-left px-3 py-2 font-medium">Issue Key</th>
                  <th className="text-left px-3 py-2 font-medium w-20">Hours</th>
                  <th className="text-left px-3 py-2 font-medium">Description</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((row, i) => {
                  const v = validation[`${i}`]
                  return (
                    <tr key={i} className="border-b last:border-0">
                      <td className="px-3 py-2">
                        <span className="font-medium truncate block max-w-[140px]">
                          {row.isManual ? `(manual) ${row.projectName}` : row.projectName}
                        </span>
                      </td>
                      <td className="px-3 py-2">
                        <div className="flex items-center gap-1">
                          <IssueKeyCombobox
                            value={row.issueKey}
                            onChange={(v) => updateRow(i, 'issueKey', v)}
                            onBlur={() => validateIssue(i)}
                            placeholder="PROJ-123"
                            compact
                          />
                          {v?.loading && <Loader2 className="w-3 h-3 animate-spin text-muted-foreground shrink-0" />}
                          {v?.valid === true && <span title={v.summary}><Check className="w-3 h-3 text-green-600 shrink-0" /></span>}
                          {v?.valid === false && <span title={v.summary}><AlertCircle className="w-3 h-3 text-destructive shrink-0" /></span>}
                        </div>
                      </td>
                      <td className="px-3 py-2">
                        <Input
                          type="number"
                          step="0.25"
                          min="0"
                          value={row.hours}
                          onChange={(e) => updateRow(i, 'hours', parseFloat(e.target.value) || 0)}
                          className="h-8 text-xs w-20"
                        />
                      </td>
                      <td className="px-3 py-2">
                        <Input
                          value={row.description}
                          onChange={(e) => updateRow(i, 'description', e.target.value)}
                          className="h-8 text-xs"
                        />
                      </td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
          </div>

          {/* Total */}
          <div className="text-sm text-muted-foreground text-right">
            Total: <span className="font-medium text-foreground">{totalHours.toFixed(1)}h</span>
            {' '}({filledRows.length}/{rows.length} entries with issue keys)
          </div>

          {/* Summarization progress */}
          {summarizeLog.length > 0 && !showResult && (
            <div className="rounded-md p-3 text-xs bg-amber-50 text-amber-800 border border-amber-200 space-y-1 max-h-32 overflow-y-auto">
              <div className="flex items-center gap-1.5 font-medium">
                <Sparkles className="w-3.5 h-3.5" />
                LLM Processing
              </div>
              {summarizeLog.map((msg, i) => (
                <div key={i} className="flex items-center gap-1.5">
                  {i === summarizeLog.length - 1 && (summarizing || syncing) && (
                    <Loader2 className="w-3 h-3 animate-spin shrink-0" />
                  )}
                  <span>{msg}</span>
                </div>
              ))}
            </div>
          )}

          {/* Result */}
          {showResult && (
            <div className={`rounded-md p-3 text-sm ${
              syncResult.dry_run
                ? 'bg-blue-50 text-blue-800 border border-blue-200'
                : syncResult.success
                  ? 'bg-green-50 text-green-800 border border-green-200'
                  : 'bg-red-50 text-red-800 border border-red-200'
            }`}>
              {syncResult.dry_run ? (
                <div className="space-y-2">
                  <p className="font-medium">Preview — will send to Tempo ({syncResult.total_entries} entries, {totalHours.toFixed(1)}h):</p>
                  <div className="bg-white/60 rounded overflow-hidden">
                    <table className="w-full text-xs">
                      <thead>
                        <tr className="border-b">
                          <th className="text-left px-2 py-1 font-medium text-blue-600">Issue</th>
                          <th className="text-left px-2 py-1 font-medium text-blue-600">Time</th>
                          <th className="text-left px-2 py-1 font-medium text-blue-600">Description</th>
                        </tr>
                      </thead>
                      <tbody>
                        {syncResult.results.map((r, i) => {
                          const cached = jiraIssueCache.get(r.issue_key)
                          return (
                            <tr key={i} className="border-b last:border-0">
                              <td className="px-2 py-1">
                                <span className="font-mono">{r.issue_key}</span>
                                {cached?.summary && (
                                  <span className="ml-1 text-blue-600/70">{cached.summary}</span>
                                )}
                              </td>
                              <td className="px-2 py-1">{r.hours}h</td>
                              <td className="px-2 py-1 break-all">{r.description}</td>
                            </tr>
                          )
                        })}
                      </tbody>
                    </table>
                  </div>
                </div>
              ) : (
                <p>
                  {syncResult.successful} exported, {syncResult.failed} failed
                  {syncResult.failed > 0 && (
                    <> — {syncResult.results.filter(r => r.status === 'error').map(r => `${r.issue_key}: ${r.error_message}`).join(', ')}</>
                  )}
                </p>
              )}
            </div>
          )}
        </div>

        <DialogFooter className="gap-2">
          <Button variant="outline" onClick={onClose}>Cancel</Button>
          <Button variant="outline" onClick={() => handleAction(true)} disabled={!canSync}>
            {(summarizing || syncing) ? <Loader2 className="w-4 h-4 animate-spin mr-2" /> : null}
            Preview All
          </Button>
          <Button onClick={() => handleAction(false)} disabled={!canSync}>
            {(summarizing || syncing) ? <Loader2 className="w-4 h-4 animate-spin mr-2" /> : null}
            Export All
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
