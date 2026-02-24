/**
 * Work Items service
 */

import { invokeAuth } from './client'
import type {
  WorkItem,
  WorkItemWithChildren,
  PaginatedResponse,
  WorkItemFilters,
  CreateWorkItemRequest,
  UpdateWorkItemRequest,
  GroupedWorkItemsResponse,
  WorkItemStatsResponse,
  TimelineResponse,
  BatchSyncRequest,
  BatchSyncResponse,
  AggregateRequest,
  AggregateResponse,
  CommitCentricWorklogResponse,
} from '@/types'

// ============ CRUD Operations ============

/**
 * List work items with filters
 */
export async function list(filters: WorkItemFilters = {}): Promise<PaginatedResponse<WorkItemWithChildren>> {
  return invokeAuth<PaginatedResponse<WorkItemWithChildren>>('list_work_items', { filters })
}

/**
 * Create a new work item
 */
export async function create(request: CreateWorkItemRequest): Promise<WorkItem> {
  return invokeAuth<WorkItem>('create_work_item', { request })
}

/**
 * Get a single work item
 */
export async function get(id: string): Promise<WorkItem> {
  return invokeAuth<WorkItem>('get_work_item', { id })
}

/**
 * Update a work item
 */
export async function update(id: string, request: UpdateWorkItemRequest): Promise<WorkItem> {
  return invokeAuth<WorkItem>('update_work_item', { id, request })
}

/**
 * Delete a work item
 */
export async function remove(id: string): Promise<void> {
  return invokeAuth<void>('delete_work_item', { id })
}

// ============ Stats & Views ============

/**
 * Get work item statistics summary
 */
export async function getStats(query: { start_date?: string; end_date?: string } = {}): Promise<WorkItemStatsResponse> {
  return invokeAuth<WorkItemStatsResponse>('get_stats_summary', { query })
}

/**
 * Get work items grouped by project and date
 */
export async function getGrouped(query: { start_date?: string; end_date?: string } = {}): Promise<GroupedWorkItemsResponse> {
  return invokeAuth<GroupedWorkItemsResponse>('get_grouped_work_items', { query })
}

/**
 * Get timeline data for Gantt chart visualization
 * @param date - The date in YYYY-MM-DD format
 * @param sources - Optional array of sources to filter by (e.g., ['claude_code'])
 */
export async function getTimeline(date: string, sources?: string[]): Promise<TimelineResponse> {
  return invokeAuth<TimelineResponse>('get_timeline_data', { query: { date, sources } })
}

// ============ Jira Mapping ============

/**
 * Map a work item to a Jira issue
 */
export async function mapToJira(
  workItemId: string,
  jiraIssueKey: string,
  jiraIssueTitle?: string
): Promise<WorkItem> {
  return invokeAuth<WorkItem>('map_work_item_jira', {
    work_item_id: workItemId,
    jira_issue_key: jiraIssueKey,
    jira_issue_title: jiraIssueTitle,
  })
}

// ============ Batch Operations ============

/**
 * Batch sync work items to Tempo
 */
export async function batchSyncToTempo(request: BatchSyncRequest): Promise<BatchSyncResponse> {
  return invokeAuth<BatchSyncResponse>('batch_sync_tempo', { request })
}

/**
 * Aggregate work items by project + date
 */
export async function aggregate(request: AggregateRequest = {}): Promise<AggregateResponse> {
  return invokeAuth<AggregateResponse>('aggregate_work_items', { request })
}

// ============ Commit-centric View ============

/**
 * Get commit-centric worklog data for Tempo sync
 */
export async function getCommitCentricWorklog(
  query: { start_date?: string; end_date?: string } = {}
): Promise<CommitCentricWorklogResponse> {
  return invokeAuth<CommitCentricWorklogResponse>('get_commit_centric_worklog', { query })
}
