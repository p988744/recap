import { useState, useCallback } from 'react'
import { workItems } from '@/services'
import { deriveProjectName } from './useWorkItems'

export interface QuickPickItem {
  title: string
  description: string
  hours: number
  project_name: string
  jira_issue_key: string
}

export function useRecentManualItems() {
  const [recentItems, setRecentItems] = useState<QuickPickItem[]>([])

  const refreshRecent = useCallback(async () => {
    try {
      const response = await workItems.list({ source: 'manual', per_page: 50 })
      // Deduplicate by title, keeping the most recent (list is sorted by date DESC)
      const seen = new Map<string, QuickPickItem>()
      for (const item of response.items) {
        const key = item.title.trim()
        if (!seen.has(key)) {
          seen.set(key, {
            title: item.title,
            description: item.description || '',
            hours: item.hours,
            project_name: deriveProjectName(item),
            jira_issue_key: item.jira_issue_key || '',
          })
        }
      }
      setRecentItems(Array.from(seen.values()).slice(0, 5))
    } catch (err) {
      console.error('Failed to fetch recent manual items:', err)
    }
  }, [])

  return { recentItems, refreshRecent }
}
