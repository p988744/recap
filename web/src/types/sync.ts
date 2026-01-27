/**
 * Sync related types
 */

export interface SyncStatus {
  id: string
  source: string
  source_path?: string
  last_sync_at?: string
  last_item_count: number
  status: string
  error_message?: string
}

export interface AutoSyncRequest {
  project_paths?: string[]
}

export interface SyncResult {
  success: boolean
  source: string
  items_synced: number
  projects_scanned: number
  items_created: number
  message?: string
}

export interface AutoSyncResponse {
  success: boolean
  results: SyncResult[]
  total_items: number
  projects_scanned: number
  items_created: number
}

export interface AvailableProject {
  path: string
  name: string
  source: string
}
