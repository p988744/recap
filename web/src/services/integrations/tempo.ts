/**
 * Tempo/Jira integration service
 */

import { invokeAuth } from '../client'
import type {
  TempoSuccessResponse,
  WorklogEntryRequest,
  WorklogEntryResponse,
  SyncWorklogsRequest,
  SyncWorklogsResponse,
  GetWorklogsRequest,
  ValidateIssueResponse,
  JiraIssueDetail,
  SearchIssuesRequest,
  SearchIssuesResponse,
} from '@/types'

/**
 * Test Jira/Tempo connection
 */
export async function testConnection(): Promise<TempoSuccessResponse> {
  return invokeAuth<TempoSuccessResponse>('test_tempo_connection')
}

/**
 * Validate a Jira issue key
 */
export async function validateIssue(issueKey: string): Promise<ValidateIssueResponse> {
  return invokeAuth<ValidateIssueResponse>('validate_jira_issue', { issueKey })
}

/**
 * Sync multiple worklogs to Tempo/Jira
 */
export async function syncWorklogs(request: SyncWorklogsRequest): Promise<SyncWorklogsResponse> {
  return invokeAuth<SyncWorklogsResponse>('sync_worklogs_to_tempo', { request })
}

/**
 * Upload a single worklog entry
 */
export async function uploadWorklog(request: WorklogEntryRequest): Promise<WorklogEntryResponse> {
  return invokeAuth<WorklogEntryResponse>('upload_single_worklog', { request })
}

/**
 * Get worklogs from Tempo for a date range
 */
export async function getWorklogs(request: GetWorklogsRequest): Promise<unknown[]> {
  return invokeAuth<unknown[]>('get_tempo_worklogs', { request })
}

/**
 * Search Jira issues by summary or key
 */
export async function searchIssues(request: SearchIssuesRequest): Promise<SearchIssuesResponse> {
  return invokeAuth<SearchIssuesResponse>('search_jira_issues', { request })
}

/**
 * Batch get full issue details for multiple issue keys
 */
export async function batchGetIssues(issueKeys: string[]): Promise<JiraIssueDetail[]> {
  return invokeAuth<JiraIssueDetail[]>('batch_get_jira_issues', { issueKeys })
}
