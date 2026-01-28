import { Fragment, useState, useEffect, useCallback } from 'react'
import { Check, AlertCircle, Loader2 } from 'lucide-react'
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
import { IssueKeyCombobox } from './IssueKeyCombobox'
import type { BatchSyncRow, SyncWorklogsResponse } from '@/types'

interface TempoWeekSyncModalProps {
  open: boolean
  startDate: string
  endDate: string
  initialRows: BatchSyncRow[]
  syncing: boolean
  syncResult: SyncWorklogsResponse | null
  onSync: (rows: BatchSyncRow[], dryRun: boolean) => Promise<SyncWorklogsResponse | null>
  onClose: () => void
}

type ValidationState = Record<string, { valid: boolean | null; summary: string; loading: boolean }>

const WEEKDAY_NAMES = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat']

function formatDayHeader(dateStr: string): string {
  const d = new Date(dateStr + 'T00:00:00')
  const mm = String(d.getMonth() + 1).padStart(2, '0')
  const dd = String(d.getDate()).padStart(2, '0')
  const weekday = WEEKDAY_NAMES[d.getDay()]
  return `${mm}/${dd} (${weekday})`
}

export function TempoWeekSyncModal({
  open,
  startDate,
  endDate,
  initialRows,
  syncing,
  syncResult,
  onSync,
  onClose,
}: TempoWeekSyncModalProps) {
  const [rows, setRows] = useState<BatchSyncRow[]>([])
  const [validation, setValidation] = useState<ValidationState>({})

  useEffect(() => {
    if (open) {
      setRows(initialRows)
      setValidation({})
    }
  }, [open, initialRows])

  const updateRow = useCallback((index: number, field: keyof BatchSyncRow, value: string | number) => {
    setRows((prev) =>
      prev.map((r, i) => (i === index ? { ...r, [field]: value } : r)),
    )
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

  // Group rows by date
  const groupedByDate = rows.reduce<Record<string, { rows: BatchSyncRow[]; startIndex: number }[]>>((acc, row, i) => {
    const date = row.date ?? 'unknown'
    if (!acc[date]) acc[date] = []
    acc[date].push({ rows: [row], startIndex: i })
    return acc
  }, {})

  // Flatten grouped structure: sorted dates with their row indices
  const sortedDates = Object.keys(groupedByDate).sort()

  const totalHours = rows.reduce((sum, r) => sum + r.hours, 0)
  const filledRows = rows.filter((r) => r.issueKey.trim() !== '')
  const canSync = filledRows.length > 0 && !syncing

  const handlePreview = () => onSync(rows, true)
  const handleSync = () => onSync(rows, false)

  const showResult = syncResult !== null

  const formatDateRange = () => {
    const s = startDate.slice(5).replace('-', '/')
    const e = endDate.slice(5).replace('-', '/')
    return `${s} — ${e}`
  }

  return (
    <Dialog open={open} onOpenChange={(o) => { if (!o) onClose() }}>
      <DialogContent className="sm:max-w-2xl max-h-[80vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>
            Export Week: {formatDateRange()}
          </DialogTitle>
          <DialogDescription className="sr-only">
            Export all worklog entries for the selected week to Tempo
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-3 py-2 overflow-y-auto flex-1 min-h-0">
          {rows.length === 0 ? (
            <div className="py-8 text-center text-sm text-muted-foreground">
              No unsynchronized items this week.
            </div>
          ) : (
            <div className="border rounded-md overflow-hidden">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b bg-muted/50">
                    <th className="text-left px-3 py-2 font-medium text-xs">Project</th>
                    <th className="text-left px-3 py-2 font-medium text-xs">Issue Key</th>
                    <th className="text-left px-3 py-2 font-medium text-xs w-20">Hours</th>
                    <th className="text-left px-3 py-2 font-medium text-xs">Description</th>
                  </tr>
                </thead>
                <tbody>
                  {sortedDates.map((date) => {
                    const entries = groupedByDate[date]
                    return entries.map(({ startIndex }, entryIdx) => {
                      const row = rows[startIndex]
                      const v = validation[`${startIndex}`]
                      return (
                        <Fragment key={startIndex}>
                          {entryIdx === 0 && (
                            <tr className="border-b bg-muted/30">
                              <td colSpan={4} className="px-3 py-1.5 text-xs font-semibold text-muted-foreground uppercase tracking-wide">
                                {formatDayHeader(date)}
                              </td>
                            </tr>
                          )}
                          <tr className="border-b last:border-0">
                            <td className="px-3 py-1.5">
                              <span className="font-medium truncate block max-w-[140px] text-xs">
                                {row.isManual ? `(manual) ${row.projectName}` : row.projectName}
                              </span>
                            </td>
                            <td className="px-3 py-1.5">
                              <div className="flex items-center gap-1">
                                <IssueKeyCombobox
                                  value={row.issueKey}
                                  onChange={(val) => updateRow(startIndex, 'issueKey', val)}
                                  onBlur={() => validateIssue(startIndex)}
                                  placeholder="PROJ-123"
                                  compact
                                />
                                {v?.loading && <Loader2 className="w-3 h-3 animate-spin text-muted-foreground shrink-0" />}
                                {v?.valid === true && <span title={v.summary}><Check className="w-3 h-3 text-green-600 shrink-0" /></span>}
                                {v?.valid === false && <span title={v.summary}><AlertCircle className="w-3 h-3 text-destructive shrink-0" /></span>}
                              </div>
                            </td>
                            <td className="px-3 py-1.5">
                              <Input
                                type="number"
                                step="0.25"
                                min="0"
                                value={row.hours}
                                onChange={(e) => updateRow(startIndex, 'hours', parseFloat(e.target.value) || 0)}
                                className="h-7 text-xs w-20"
                              />
                            </td>
                            <td className="px-3 py-1.5">
                              <Input
                                value={row.description}
                                onChange={(e) => updateRow(startIndex, 'description', e.target.value)}
                                className="h-7 text-xs"
                              />
                            </td>
                          </tr>
                        </Fragment>
                      )
                    })
                  })}
                </tbody>
              </table>
            </div>
          )}

          {/* Total */}
          {rows.length > 0 && (
            <div className="text-sm text-muted-foreground text-right">
              Total: <span className="font-medium text-foreground">{totalHours.toFixed(1)}h</span>
              {' '}({filledRows.length}/{rows.length} entries with issue keys)
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
                <p>Preview: {syncResult.total_entries} entries ready ({totalHours.toFixed(1)}h total)</p>
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
          <Button variant="outline" onClick={handlePreview} disabled={!canSync}>
            {syncing ? <Loader2 className="w-4 h-4 animate-spin mr-2" /> : null}
            Preview All
          </Button>
          <Button onClick={handleSync} disabled={!canSync}>
            {syncing ? <Loader2 className="w-4 h-4 animate-spin mr-2" /> : null}
            Export All
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
