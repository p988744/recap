import { useState, useCallback, useEffect } from 'react'
import { httpExport } from '@/services'
import type { HttpExportConfig, HttpExportResponse, InlineExportItem, WorkItem } from '@/types'

export function useHttpExport(isAuthenticated: boolean) {
  const [configs, setConfigs] = useState<HttpExportConfig[]>([])
  const [selectedConfigId, setSelectedConfigId] = useState<string>('')
  const [showModal, setShowModal] = useState(false)
  const [itemsToExport, setItemsToExport] = useState<WorkItem[]>([])
  const [result, setResult] = useState<HttpExportResponse | null>(null)
  const [exporting, setExporting] = useState(false)
  // Track which items have already been exported to the selected config
  const [exportedIds, setExportedIds] = useState<Set<string>>(new Set())

  const loadConfigs = useCallback(async () => {
    try {
      const list = await httpExport.listConfigs()
      console.log('[useHttpExport] loaded configs:', list.length, 'enabled:', list.filter((c) => c.enabled).length)
      const enabled = list.filter((c) => c.enabled)
      setConfigs(enabled)
      if (enabled.length > 0 && !selectedConfigId) {
        setSelectedConfigId(enabled[0].id)
      }
    } catch (e) {
      console.error('[useHttpExport] loadConfigs failed:', e)
    }
  }, [selectedConfigId])

  useEffect(() => {
    if (isAuthenticated) loadConfigs()
  }, [isAuthenticated, loadConfigs])

  const openExport = useCallback(
    async (items: WorkItem[]) => {
      setItemsToExport(items)
      setResult(null)
      setExportedIds(new Set())
      setShowModal(true)
      // Load export history for the selected config
      if (selectedConfigId && items.length > 0) {
        try {
          const history = await httpExport.getExportHistory(
            selectedConfigId,
            items.map((i) => i.id),
          )
          setExportedIds(new Set(history.map((h) => h.work_item_id)))
        } catch {
          // Ignore — just won't show history
        }
      }
    },
    [selectedConfigId]
  )

  const executeExport = useCallback(
    async (dryRun: boolean, includeExported = false) => {
      if (!selectedConfigId || itemsToExport.length === 0) return
      // Filter out already-exported items unless explicitly included
      const toExport = includeExported
        ? itemsToExport
        : itemsToExport.filter((i) => !exportedIds.has(i.id))
      if (toExport.length === 0) return
      setExporting(true)
      setResult(null)
      try {
        const inlineItems: InlineExportItem[] = toExport.map((i) => ({
          id: i.id,
          title: i.title,
          description: i.description,
          hours: i.hours,
          date: i.date,
          source: i.source,
          jira_issue_key: i.jira_issue_key,
          category: i.category,
        }))
        const res = await httpExport.executeExport({
          config_id: selectedConfigId,
          work_item_ids: toExport.map((i) => i.id),
          inline_items: inlineItems,
          dry_run: dryRun,
        })
        setResult(res)
        // Refresh history after successful export
        if (!dryRun && res.successful > 0) {
          try {
            const history = await httpExport.getExportHistory(
              selectedConfigId,
              itemsToExport.map((i) => i.id),
            )
            setExportedIds(new Set(history.map((h) => h.work_item_id)))
          } catch { /* ignore */ }
        }
      } catch (e) {
        setResult({
          total: toExport.length,
          successful: 0,
          failed: toExport.length,
          results: [],
          dry_run: dryRun,
        })
      } finally {
        setExporting(false)
      }
    },
    [selectedConfigId, itemsToExport, exportedIds]
  )

  const closeModal = useCallback(() => {
    setShowModal(false)
    setResult(null)
  }, [])

  const hasConfigs = configs.length > 0

  return {
    configs,
    selectedConfigId,
    setSelectedConfigId,
    showModal,
    setShowModal,
    itemsToExport,
    result,
    exporting,
    exportedIds,
    hasConfigs,
    loadConfigs,
    openExport,
    executeExport,
    closeModal,
  }
}
