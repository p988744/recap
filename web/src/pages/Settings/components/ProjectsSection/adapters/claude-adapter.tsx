import { claude } from '@/services/integrations'
import type { DataSourceAdapter, DiscoveredProject, SyncResult } from '../types'
import { ClaudeIcon } from '../icons/ClaudeIcon'

export function createClaudeAdapter(): DataSourceAdapter {
  return {
    key: 'claude_code',
    label: 'Claude Code',
    icon: <ClaudeIcon className="w-4 h-4" />,
    colorClass: 'text-amber-600',
    badgeBgClass: 'bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400',

    async checkInstalled() {
      return true
    },

    async scanProjects(): Promise<DiscoveredProject[]> {
      const projects = await claude.listSessions()
      return projects.map((p) => ({
        name: p.name,
        path: p.path,
        sessionCount: p.sessions.length,
        sessions: p.sessions.map((s) => ({
          id: s.session_id,
          summary: s.first_message || `Session ${s.session_id.slice(0, 8)}`,
          timestamp: s.first_timestamp,
          detail: `${s.message_count} messages, ${s.tool_usage.length} tools`,
        })),
      }))
    },

    async syncAll(): Promise<SyncResult> {
      const projects = await claude.listSessions()
      const paths = projects.map((p) => p.path)
      const result = await claude.syncProjects({ project_paths: paths })
      return {
        sessionsProcessed: result.sessions_processed,
        sessionsSkipped: result.sessions_skipped,
        workItemsCreated: result.work_items_created,
        workItemsUpdated: result.work_items_updated,
      }
    },
  }
}
