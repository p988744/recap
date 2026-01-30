import { useState, useEffect, useCallback } from 'react'
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
import { Label } from '@/components/ui/label'
import { tempo } from '@/services'
import * as jiraIssueCache from '@/services/jiraIssueCache'
import { IssueKeyCombobox } from './IssueKeyCombobox'
import { SummarizationProgress } from './SummarizationProgress'
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
  // Summarization progress
  const [summarizing, setSummarizing] = useState(false)
  const [summarizeLog, setSummarizeLog] = useState<string[]>([])

  // Initialize form when target changes
  useEffect(() => {
    if (target) {
      setIssueKey(defaultIssueKey)
      setHours(target.hours)
      setDescription(target.description)
      setIssueValid(null)
      setIssueSummary('')
      setSummarizeLog([])
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
    } catch (err) {
      console.error('Issue validation error:', key, err)
      setIssueValid(false)
      setIssueSummary(String(err))
    } finally {
      setValidating(false)
    }
  }, [issueKey])

  const handleAction = useCallback(async (dryRun: boolean) => {
    const key = issueKey.trim()
    if (!key || hours <= 0) return

    // Step 1: Summarize description via LLM
    setSummarizing(true)
    setSummarizeLog(['Summarizing description with LLM...'])
    let finalDesc = description
    try {
      const summary = await tempo.summarizeDescription(description)
      finalDesc = summary
      setSummarizeLog(prev => [...prev, `✓ "${summary}"`])
    } catch {
      setSummarizeLog(prev => [...prev, '⚠ LLM unavailable, using fallback'])
    }

    // Step 2: Send to sync
    setSummarizeLog(prev => [...prev, dryRun ? 'Generating preview...' : 'Uploading to Tempo...'])
    setSummarizing(false)
    await onSync(key, hours, finalDesc, dryRun)
  }, [issueKey, hours, description, onSync])

  const canSync = issueKey.trim() !== '' && hours > 0 && !syncing && !summarizing
  const showResult = syncResult !== null

  if (!target) return null

  return (
    <Dialog open={!!target} onOpenChange={(open) => { if (!open) onClose() }}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Export to Jira Tempo</DialogTitle>
          <DialogDescription className="sr-only">
            Export a single worklog entry to Tempo
          </DialogDescription>
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
              <IssueKeyCombobox
                value={issueKey}
                onChange={(v) => { setIssueKey(v); setIssueValid(null) }}
                onBlur={validateIssue}
                placeholder="e.g. PROJ-123"
              />
              {validating && <Loader2 className="w-4 h-4 animate-spin text-muted-foreground" />}
              {issueValid === true && <span title={issueSummary}><Check className="w-4 h-4 text-green-600" /></span>}
              {issueValid === false && <span title={issueSummary}><AlertCircle className="w-4 h-4 text-destructive" /></span>}
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

          {/* Summarization progress */}
          {!showResult && (
            <SummarizationProgress log={summarizeLog} active={summarizing || syncing} />
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
                  <p className="font-medium">Preview — will send to Tempo:</p>
                  {(() => {
                    const cached = jiraIssueCache.get(issueKey)
                    return (
                      <div className="bg-white/60 rounded p-2 space-y-1 text-xs">
                        <div className="flex gap-2">
                          <span className="text-blue-600 font-medium shrink-0">Issue:</span>
                          <span className="font-mono">{issueKey}</span>
                          {cached?.summary && (
                            <span className="text-blue-600/70 truncate">{cached.summary}</span>
                          )}
                        </div>
                        {cached?.issueType && (
                          <div className="flex gap-2">
                            <span className="text-blue-600 font-medium shrink-0">Type:</span>
                            <span>{cached.issueType}</span>
                          </div>
                        )}
                        {cached?.assignee && (
                          <div className="flex gap-2">
                            <span className="text-blue-600 font-medium shrink-0">Assignee:</span>
                            <span>{cached.assignee}</span>
                          </div>
                        )}
                        <div className="flex gap-2">
                          <span className="text-blue-600 font-medium shrink-0">Date:</span>
                          <span>{syncResult.results[0]?.date}</span>
                        </div>
                        <div className="flex gap-2">
                          <span className="text-blue-600 font-medium shrink-0">Time:</span>
                          <span>{hours}h ({syncResult.results[0]?.minutes}min)</span>
                        </div>
                        <div className="flex gap-2">
                          <span className="text-blue-600 font-medium shrink-0">Desc:</span>
                          <span className="break-all">{syncResult.results[0]?.description}</span>
                        </div>
                      </div>
                    )
                  })()}
                </div>
              ) : syncResult.success ? (
                <p>Exported successfully! {syncResult.successful} worklog uploaded.</p>
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
          <Button variant="outline" onClick={() => handleAction(true)} disabled={!canSync}>
            {(summarizing || syncing) ? <Loader2 className="w-4 h-4 animate-spin mr-2" /> : null}
            Preview
          </Button>
          <Button onClick={() => handleAction(false)} disabled={!canSync}>
            {(summarizing || syncing) ? <Loader2 className="w-4 h-4 animate-spin mr-2" /> : null}
            Export
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
