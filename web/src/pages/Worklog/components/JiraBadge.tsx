import { useState, useCallback, useRef, useEffect, useSyncExternalStore } from 'react'
import { Loader2 } from 'lucide-react'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { tempo } from '@/services'
import * as jiraIssueCache from '@/services/jiraIssueCache'

interface JiraBadgeProps {
  issueKey: string
}

export function JiraBadge({ issueKey }: JiraBadgeProps) {
  // Subscribe to cache updates so we re-render when prefetch completes
  const cacheVersion = useSyncExternalStore(
    jiraIssueCache.subscribe,
    () => jiraIssueCache.has(issueKey),
  )

  const cached = jiraIssueCache.get(issueKey)

  const [summary, setSummary] = useState<string | null>(cached?.summary ?? null)
  const [description, setDescription] = useState<string | null>(cached?.description ?? null)
  const [assignee, setAssignee] = useState<string | null>(cached?.assignee ?? null)
  const [issueType, setIssueType] = useState<string | null>(cached?.issueType ?? null)
  const [loading, setLoading] = useState(false)
  const fetched = useRef(false)

  // When cache updates externally (prefetch), sync local state
  useEffect(() => {
    if (cached && !fetched.current) {
      setSummary(cached.summary)
      setDescription(cached.description ?? null)
      setAssignee(cached.assignee ?? null)
      setIssueType(cached.issueType ?? null)
      fetched.current = true
    }
  }, [cacheVersion, cached])

  const fetchDetails = useCallback(async () => {
    if (fetched.current) return

    // Check cache again (may have been populated between render and hover)
    const entry = jiraIssueCache.get(issueKey)
    if (entry) {
      setSummary(entry.summary)
      setDescription(entry.description ?? null)
      setAssignee(entry.assignee ?? null)
      setIssueType(entry.issueType ?? null)
      fetched.current = true
      return
    }

    setLoading(true)
    try {
      const result = await tempo.validateIssue(issueKey)
      const s = result.valid ? (result.summary ?? '') : ''
      setSummary(s)
      setDescription(result.description ?? null)
      setAssignee(result.assignee ?? null)
      setIssueType(result.issue_type ?? null)
      // Store in shared cache
      jiraIssueCache.set(issueKey, {
        summary: s,
        description: result.description,
        assignee: result.assignee,
        issueType: result.issue_type,
      })
      fetched.current = true
    } catch {
      setSummary('')
      fetched.current = true
    } finally {
      setLoading(false)
    }
  }, [issueKey])

  // Show summary inline if already fetched
  const inlineTitle = summary ? ` ${summary}` : ''

  return (
    <TooltipProvider delayDuration={200}>
      <Tooltip onOpenChange={(open) => { if (open) fetchDetails() }}>
        <TooltipTrigger asChild>
          <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[11px] font-medium bg-blue-50 text-blue-700 border border-blue-200 cursor-default max-w-[260px]">
            <svg className="w-3 h-3 shrink-0" viewBox="0 0 24 24" fill="currentColor">
              <path d="M11.53 2c-.55 0-1.06.23-1.42.6l-1.38 1.4a2 2 0 0 0 0 2.83l6.14 6.14a2 2 0 0 0 2.83 0l1.38-1.4a2 2 0 0 0 0-2.83L12.95 2.6A2 2 0 0 0 11.53 2zm-4.13 6.1a2 2 0 0 0-2.82 0L3.2 9.48a2 2 0 0 0 0 2.83l6.14 6.14a2 2 0 0 0 2.83 0l1.38-1.38a2 2 0 0 0 0-2.83L7.4 8.1z" />
            </svg>
            <span className="shrink-0">{issueKey}</span>
            {inlineTitle && (
              <span className="truncate text-blue-600/70">{inlineTitle}</span>
            )}
          </span>
        </TooltipTrigger>
        <TooltipContent side="bottom" align="start" className="max-w-xs">
          {loading ? (
            <div className="flex items-center gap-1.5">
              <Loader2 className="w-3 h-3 animate-spin" />
              <span>Loading...</span>
            </div>
          ) : (
            <div className="space-y-1">
              <p className="font-medium">{issueKey}</p>
              {issueType && (
                <p className="text-primary-foreground/60 text-[11px]">{issueType}</p>
              )}
              {summary && (
                <p className="text-primary-foreground/80">{summary}</p>
              )}
              {assignee && (
                <p className="text-primary-foreground/60 text-[11px]">
                  Assignee: {assignee}
                </p>
              )}
              {description && (
                <p className="text-primary-foreground/60 text-[11px] line-clamp-3">
                  {description}
                </p>
              )}
              {summary === '' && !loading && (
                <p className="text-primary-foreground/50 italic">No details available</p>
              )}
            </div>
          )}
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  )
}
