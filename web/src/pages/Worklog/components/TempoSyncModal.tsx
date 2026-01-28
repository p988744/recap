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
import { Label } from '@/components/ui/label'
import { tempo } from '@/services'
import type { TempoSyncTarget, SyncWorklogsResponse } from '@/types'

interface TempoSyncModalProps {
  target: TempoSyncTarget | null
  defaultIssueKey: string
  syncing: boolean
  syncResult: SyncWorklogsResponse | null
  onSync: (issueKey: string, hours: number, description: string, dryRun: boolean) => Promise<SyncWorklogsResponse | null>
  onClose: () => void
}

export function TempoSyncModal({
  target,
  defaultIssueKey,
  syncing,
  syncResult,
  onSync,
  onClose,
}: TempoSyncModalProps) {
  const [issueKey, setIssueKey] = useState('')
  const [hours, setHours] = useState(0)
  const [description, setDescription] = useState('')
  const [validating, setValidating] = useState(false)
  const [issueValid, setIssueValid] = useState<boolean | null>(null)
  const [issueSummary, setIssueSummary] = useState('')

  // Initialize form when target changes
  useEffect(() => {
    if (target) {
      setIssueKey(defaultIssueKey)
      setHours(target.hours)
      setDescription(target.description)
      setIssueValid(null)
      setIssueSummary('')
    }
  }, [target, defaultIssueKey])

  // Validate issue key on blur
  const validateIssue = useCallback(async () => {
    const key = issueKey.trim()
    if (!key) {
      setIssueValid(null)
      setIssueSummary('')
      return
    }
    setValidating(true)
    try {
      const result = await tempo.validateIssue(key)
      setIssueValid(result.valid)
      setIssueSummary(result.valid ? (result.summary ?? '') : result.message)
    } catch {
      setIssueValid(false)
      setIssueSummary('Validation failed')
    } finally {
      setValidating(false)
    }
  }, [issueKey])

  const handlePreview = () => onSync(issueKey.trim(), hours, description, true)
  const handleSync = () => onSync(issueKey.trim(), hours, description, false)

  const canSync = issueKey.trim() !== '' && hours > 0 && !syncing
  const showResult = syncResult !== null

  if (!target) return null

  return (
    <Dialog open={!!target} onOpenChange={(open) => { if (!open) onClose() }}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Sync to Tempo</DialogTitle>
        </DialogHeader>

        <div className="space-y-4 py-2">
          {/* Project info */}
          <div className="flex gap-4 text-sm">
            <div>
              <span className="text-muted-foreground">Project: </span>
              <span className="font-medium">{target.projectName}</span>
            </div>
            <div>
              <span className="text-muted-foreground">Date: </span>
              <span className="font-medium">{target.date} ({target.weekday})</span>
            </div>
          </div>

          {/* Issue Key */}
          <div className="space-y-1.5">
            <Label htmlFor="issue-key">Issue Key</Label>
            <div className="flex items-center gap-2">
              <Input
                id="issue-key"
                value={issueKey}
                onChange={(e) => { setIssueKey(e.target.value); setIssueValid(null) }}
                onBlur={validateIssue}
                placeholder="e.g. PROJ-123"
                className="flex-1"
              />
              {validating && <Loader2 className="w-4 h-4 animate-spin text-muted-foreground" />}
              {issueValid === true && <Check className="w-4 h-4 text-green-600" />}
              {issueValid === false && <AlertCircle className="w-4 h-4 text-destructive" />}
            </div>
            {issueSummary && (
              <p className={`text-xs ${issueValid ? 'text-muted-foreground' : 'text-destructive'}`}>
                {issueSummary}
              </p>
            )}
          </div>

          {/* Hours */}
          <div className="space-y-1.5">
            <Label htmlFor="hours">Hours</Label>
            <Input
              id="hours"
              type="number"
              step="0.25"
              min="0"
              value={hours}
              onChange={(e) => setHours(parseFloat(e.target.value) || 0)}
            />
          </div>

          {/* Description */}
          <div className="space-y-1.5">
            <Label htmlFor="description">Description</Label>
            <textarea
              id="description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              rows={3}
              className="flex w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
            />
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
                <p>Preview: {syncResult.total_entries} entry ready to sync ({hours}h to {issueKey})</p>
              ) : syncResult.success ? (
                <p>Synced successfully! {syncResult.successful} worklog uploaded.</p>
              ) : (
                <p>Failed: {syncResult.results[0]?.error_message ?? 'Unknown error'}</p>
              )}
            </div>
          )}
        </div>

        <DialogFooter className="gap-2">
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="outline" onClick={handlePreview} disabled={!canSync}>
            {syncing ? <Loader2 className="w-4 h-4 animate-spin mr-2" /> : null}
            Preview
          </Button>
          <Button onClick={handleSync} disabled={!canSync}>
            {syncing ? <Loader2 className="w-4 h-4 animate-spin mr-2" /> : null}
            Sync
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
