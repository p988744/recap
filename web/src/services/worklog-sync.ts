/**
 * Worklog Sync service — project-issue mappings and sync records
 */

import { invokeAuth } from './client'
import type {
  ProjectIssueMapping,
  WorklogSyncRecord,
  SaveMappingRequest,
  GetSyncRecordsRequest,
  SaveSyncRecordRequest,
} from '@/types'

/** Get all project-to-issue mappings for current user */
export async function getMappings(): Promise<ProjectIssueMapping[]> {
  return invokeAuth<ProjectIssueMapping[]>('get_project_issue_mappings')
}

/** Save or update a project-to-issue mapping */
export async function saveMapping(request: SaveMappingRequest): Promise<ProjectIssueMapping> {
  return invokeAuth<ProjectIssueMapping>('save_project_issue_mapping', { request })
}

/** Get worklog sync records for a date range */
export async function getSyncRecords(request: GetSyncRecordsRequest): Promise<WorklogSyncRecord[]> {
  return invokeAuth<WorklogSyncRecord[]>('get_worklog_sync_records', { request })
}

/** Save a sync record after successful Tempo upload */
export async function saveSyncRecord(request: SaveSyncRecordRequest): Promise<WorklogSyncRecord> {
  return invokeAuth<WorklogSyncRecord>('save_worklog_sync_record', { request })
}
