import { useState, useCallback } from 'react'
import { tempo } from '@/services'
import type { BatchSyncRow, SyncWorklogsResponse } from '@/types'

interface UseSummarizeActionOptions {
  rows: BatchSyncRow[]
  onSync: (rows: BatchSyncRow[], dryRun: boolean) => Promise<SyncWorklogsResponse | null>
}

export function useSummarizeAction({ rows, onSync }: UseSummarizeActionOptions) {
  const [summarizing, setSummarizing] = useState(false)
  const [summarizeLog, setSummarizeLog] = useState<string[]>([])

  const resetLog = useCallback(() => setSummarizeLog([]), [])

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

  return { summarizing, summarizeLog, resetLog, handleAction }
}
