import { antigravity } from '@/services/integrations'
import type { DataSourceAdapter, DiscoveredProject, SyncResult } from '../types'
import { GeminiIcon } from '../icons/GeminiIcon'

/**
 * Antigravity (Gemini Code) adapter.
 *
 * Uses the local Antigravity HTTP API when the app is running.
 * Requires Antigravity desktop app to be open.
 */
export function createAntigravityAdapter(): DataSourceAdapter {
  return {
    key: 'antigravity',
    label: 'Gemini Code',
    icon: <GeminiIcon className="w-4 h-4" />,
    colorClass: 'text-blue-600',
    badgeBgClass: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',

    async checkInstalled(): Promise<boolean> {
      return antigravity.checkInstalled()
    },

    async scanProjects(): Promise<DiscoveredProject[]> {
      const projects = await antigravity.listProjects()
      return projects.map((p) => ({
        name: p.name,
        path: p.path,
        sessionCount: p.sessions.length,
        sessions: p.sessions.map((s) => ({
          id: s.session_id,
          summary: s.summary || 'Gemini session',
          timestamp: s.first_timestamp,
          detail: `${s.step_count} steps${s.git_branch ? `, ${s.git_branch}` : ''}`,
        })),
      }))
    },

    async syncAll(): Promise<SyncResult> {
      const projects = await antigravity.listProjects()
      const paths = projects.map((p) => p.path)
      const result = await antigravity.sync({ project_paths: paths })
      return {
        sessionsProcessed: result.sessions_processed,
        sessionsSkipped: result.sessions_skipped,
        workItemsCreated: result.work_items_created,
        workItemsUpdated: result.work_items_updated,
      }
    },
  }
}
