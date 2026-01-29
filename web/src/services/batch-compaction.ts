/**
 * Batch Compaction Service
 *
 * API for batch mode compaction using OpenAI Batch API.
 * Provides 50% cost savings with 24-hour turnaround.
 */

import { invokeAuth } from './client'

// =============================================================================
// Types
// =============================================================================

export interface BatchJobStatus {
  job_id: string
  status: 'pending' | 'submitted' | 'in_progress' | 'completed' | 'failed' | 'cancelled' | 'expired'
  total_requests: number
  completed_requests: number
  failed_requests: number
  created_at: string
  submitted_at: string | null
  completed_at: string | null
  openai_batch_id: string | null
  error_message: string | null
}

export interface BatchAvailability {
  available: boolean
  reason: string | null
}

export interface PendingHourlyItem {
  project_path: string
  hour_bucket: string
  snapshot_count: number
}

export interface PendingHourlyResponse {
  count: number
  items: PendingHourlyItem[]
}

export interface BatchSubmitResponse {
  success: boolean
  job_id: string | null
  total_requests: number
  message: string
}

export interface BatchProcessResponse {
  success: boolean
  summaries_saved: number
  daily_compacted: number
  monthly_compacted: number
  errors: string[]
  message: string
}

// =============================================================================
// API Functions
// =============================================================================

/**
 * Check if batch API is available (requires OpenAI provider with API key)
 */
export async function checkBatchAvailability(): Promise<BatchAvailability> {
  return invokeAuth<BatchAvailability>('check_batch_availability')
}

/**
 * Get pending hourly compactions that can be batched
 */
export async function getPendingHourlyCompactions(): Promise<PendingHourlyResponse> {
  return invokeAuth<PendingHourlyResponse>('get_pending_hourly_compactions')
}

/**
 * Get current batch job status (if any)
 */
export async function getBatchJobStatus(): Promise<BatchJobStatus | null> {
  return invokeAuth<BatchJobStatus | null>('get_batch_job_status')
}

/**
 * Submit pending hourly compactions as a batch job
 */
export async function submitBatchCompaction(): Promise<BatchSubmitResponse> {
  return invokeAuth<BatchSubmitResponse>('submit_batch_compaction')
}

/**
 * Refresh batch job status from OpenAI
 */
export async function refreshBatchStatus(jobId: string): Promise<BatchJobStatus> {
  return invokeAuth<BatchJobStatus>('refresh_batch_status', { jobId })
}

/**
 * Process completed batch job and run remaining compaction
 */
export async function processCompletedBatch(jobId: string): Promise<BatchProcessResponse> {
  return invokeAuth<BatchProcessResponse>('process_completed_batch_job', { jobId })
}

/**
 * Helper: Check if a batch job is in a terminal state
 */
export function isTerminalStatus(status: BatchJobStatus['status']): boolean {
  return ['completed', 'failed', 'cancelled', 'expired'].includes(status)
}

/**
 * Helper: Get human-readable status label
 */
export function getStatusLabel(status: BatchJobStatus['status']): string {
  const labels: Record<BatchJobStatus['status'], string> = {
    pending: '準備中',
    submitted: '已提交',
    in_progress: '處理中',
    completed: '已完成',
    failed: '失敗',
    cancelled: '已取消',
    expired: '已過期',
  }
  return labels[status] || status
}
