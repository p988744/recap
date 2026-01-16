/**
 * Claude Code integration service
 */

import { invokeAuth } from '../client'
import type {
  ClaudeProject,
  ImportSessionsRequest,
  ImportResult,
  SummarizeRequest,
  SummarizeResult,
  SyncProjectsRequest,
  ClaudeSyncResult,
} from '@/types'

/**
 * List all Claude Code sessions from local machine
 */
export async function listSessions(): Promise<ClaudeProject[]> {
  return invokeAuth<ClaudeProject[]>('list_claude_sessions')
}

/**
 * Import selected sessions as work items
 */
export async function importSessions(request: ImportSessionsRequest): Promise<ImportResult> {
  return invokeAuth<ImportResult>('import_claude_sessions', { request })
}

/**
 * Summarize a session using LLM
 */
export async function summarizeSession(request: SummarizeRequest): Promise<SummarizeResult> {
  return invokeAuth<SummarizeResult>('summarize_claude_session', { request })
}

/**
 * Sync selected projects - aggregate sessions by project+date
 */
export async function syncProjects(request: SyncProjectsRequest): Promise<ClaudeSyncResult> {
  return invokeAuth<ClaudeSyncResult>('sync_claude_projects', { request })
}
