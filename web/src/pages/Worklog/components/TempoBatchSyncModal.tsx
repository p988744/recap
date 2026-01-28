import { useState, useEffect, useCallback } from 'react'
import { Check, AlertCircle, Loader2 } from 'lucide-react'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { tempo } from '@/services'
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

  // Initialize rows when modal opens
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
    } catch {
      setValidation((prev) => ({
        ...prev,
        [`${index}`]: { valid: false, summary: 'Validation failed', loading: false },
      }))
    }
  }, [rows])

  const totalHours = rows.reduce((sum, r) => sum + r.hours, 0)
  const filledRows = rows.filter((r) => r.issueKey.trim() !== '')
  const canSync = filledRows.length > 0 && !syncing

  const handlePreview = () => onSync(rows, true)
  const handleSync = () => onSync(rows, false)

  const showResult = syncResult !== null

  return (
    <Dialog open={open} onOpenChange={(o) => { if (!o) onClose() }}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>
            Sync Day: {date.slice(5).replace('-', '/')} ({weekday})
          </DialogTitle>
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
                          <Input
                            value={row.issueKey}
                            onChange={(e) => updateRow(i, 'issueKey', e.target.value)}
                            onBlur={() => validateIssue(i)}
                            placeholder="PROJ-123"
                            className="h-8 text-xs"
                          />
                          {v?.loading && <Loader2 className="w-3 h-3 animate-spin text-muted-foreground shrink-0" />}
                          {v?.valid === true && <Check className="w-3 h-3 text-green-600 shrink-0" />}
                          {v?.valid === false && <AlertCircle className="w-3 h-3 text-destructive shrink-0" />}
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
                  {syncResult.successful} synced, {syncResult.failed} failed
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
            Sync All
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
