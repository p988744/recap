import { useEffect, useState, useCallback } from 'react'
import { worklogSync, tempo } from '@/services'
import type {
  WorklogSyncRecord,
  TempoSyncTarget,
  BatchSyncRow,
  WorklogDay,
  SyncWorklogsResponse,
} from '@/types'

export function useTempoSync(
  isAuthenticated: boolean,
  startDate: string,
  endDate: string,
  _days: WorklogDay[],
  onSyncComplete: () => void,
) {
  // Persistent mappings: projectPath → issueKey
  const [mappings, setMappings] = useState<Record<string, string>>({})
  // Sync records for current date range
  const [syncRecords, setSyncRecords] = useState<WorklogSyncRecord[]>([])
  // Modal state
  const [syncTarget, setSyncTarget] = useState<TempoSyncTarget | null>(null)
  const [batchSyncDate, setBatchSyncDate] = useState<string | null>(null)
  const [batchSyncWeekday, setBatchSyncWeekday] = useState<string>('')
  // Loading / result
  const [syncing, setSyncing] = useState(false)
  const [syncResult, setSyncResult] = useState<SyncWorklogsResponse | null>(null)

  // ---- Load mappings and sync records ----

  const loadMappings = useCallback(async () => {
    try {
      const list = await worklogSync.getMappings()
      const map: Record<string, string> = {}
      for (const m of list) {
        map[m.project_path] = m.jira_issue_key
      }
      setMappings(map)
    } catch {
      // ignore — Jira may not be configured
    }
  }, [])

  const loadSyncRecords = useCallback(async () => {
    try {
      const records = await worklogSync.getSyncRecords({
        date_from: startDate,
        date_to: endDate,
      })
      setSyncRecords(records)
    } catch {
      setSyncRecords([])
    }
  }, [startDate, endDate])

  useEffect(() => {
    if (!isAuthenticated) return
    loadMappings()
    loadSyncRecords()
  }, [isAuthenticated, loadMappings, loadSyncRecords])

  // ---- Lookup helpers ----

  /** Get sync record for a project + date */
  const getSyncRecord = useCallback(
    (projectPath: string, date: string): WorklogSyncRecord | undefined => {
      return syncRecords.find(
        (r) => r.project_path === projectPath && r.date === date,
      )
    },
    [syncRecords],
  )

  /** Get saved issue key for a project path */
  const getMappedIssueKey = useCallback(
    (projectPath: string): string => {
      return mappings[projectPath] ?? ''
    },
    [mappings],
  )

  // ---- Single project sync modal ----

  const openSyncModal = useCallback(
    (target: TempoSyncTarget) => {
      setSyncTarget(target)
      setSyncResult(null)
    },
    [],
  )

  const closeSyncModal = useCallback(() => {
    setSyncTarget(null)
    setSyncResult(null)
  }, [])

  const executeSingleSync = useCallback(
    async (issueKey: string, hours: number, description: string, dryRun: boolean) => {
      if (!syncTarget) return null
      setSyncing(true)
      setSyncResult(null)
      try {
        const minutes = Math.round(hours * 60)
        const result = await tempo.syncWorklogs({
          entries: [
            {
              issue_key: issueKey,
              date: syncTarget.date,
              minutes,
              description,
            },
          ],
          dry_run: dryRun,
        })
        setSyncResult(result)

        if (!dryRun && result.success) {
          // Save mapping
          await worklogSync.saveMapping({
            project_path: syncTarget.projectPath,
            jira_issue_key: issueKey,
          })
          // Save sync record
          const tempoWorklogId = result.results[0]?.id ?? undefined
          await worklogSync.saveSyncRecord({
            project_path: syncTarget.projectPath,
            date: syncTarget.date,
            jira_issue_key: issueKey,
            hours,
            description,
            tempo_worklog_id: tempoWorklogId,
          })
          // Refresh
          await loadMappings()
          await loadSyncRecords()
          onSyncComplete()
        }
        return result
      } catch (err) {
        console.error('Tempo sync failed:', err)
        return null
      } finally {
        setSyncing(false)
      }
    },
    [syncTarget, loadMappings, loadSyncRecords, onSyncComplete],
  )

  // ---- Batch sync modal ----

  const openBatchSyncModal = useCallback(
    (date: string, weekday: string) => {
      setBatchSyncDate(date)
      setBatchSyncWeekday(weekday)
      setSyncResult(null)
    },
    [],
  )

  const closeBatchSyncModal = useCallback(() => {
    setBatchSyncDate(null)
    setBatchSyncWeekday('')
    setSyncResult(null)
  }, [])

  const executeBatchSync = useCallback(
    async (rows: BatchSyncRow[], dryRun: boolean) => {
      if (!batchSyncDate) return null
      setSyncing(true)
      setSyncResult(null)
      try {
        const entries = rows
          .filter((r) => r.issueKey.trim() !== '')
          .map((r) => ({
            issue_key: r.issueKey.trim(),
            date: batchSyncDate,
            minutes: Math.round(r.hours * 60),
            description: r.description,
          }))

        if (entries.length === 0) return null

        const result = await tempo.syncWorklogs({
          entries,
          dry_run: dryRun,
        })
        setSyncResult(result)

        if (!dryRun && result.success) {
          // Save mappings and sync records for each successful entry
          for (let i = 0; i < rows.length; i++) {
            const row = rows[i]
            const entryResult = result.results[i]
            if (!row.issueKey.trim() || entryResult?.status !== 'success') continue

            await worklogSync.saveMapping({
              project_path: row.projectPath,
              jira_issue_key: row.issueKey.trim(),
            })
            await worklogSync.saveSyncRecord({
              project_path: row.projectPath,
              date: batchSyncDate,
              jira_issue_key: row.issueKey.trim(),
              hours: row.hours,
              description: row.description,
              tempo_worklog_id: entryResult.id ?? undefined,
            })
          }
          await loadMappings()
          await loadSyncRecords()
          onSyncComplete()
        }
        return result
      } catch (err) {
        console.error('Batch sync failed:', err)
        return null
      } finally {
        setSyncing(false)
      }
    },
    [batchSyncDate, loadMappings, loadSyncRecords, onSyncComplete],
  )

  return {
    // State
    mappings,
    syncRecords,
    syncing,
    syncResult,
    // Lookups
    getSyncRecord,
    getMappedIssueKey,
    // Single sync modal
    syncTarget,
    openSyncModal,
    closeSyncModal,
    executeSingleSync,
    // Batch sync modal
    batchSyncDate,
    batchSyncWeekday,
    openBatchSyncModal,
    closeBatchSyncModal,
    executeBatchSync,
  }
}
