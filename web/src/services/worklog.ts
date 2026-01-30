/**
 * Worklog service - queries for worklog overview, hourly breakdown, and compaction
 */

import { invokeAuth } from './client'
import type { WorklogOverviewResponse, HourlyBreakdownItem } from '@/types/worklog'

/** Response from compaction operations */
export interface CompactionResult {
  hourly_compacted: number
  daily_compacted: number
  weekly_compacted: number
  monthly_compacted: number
  errors: string[]
  /** Latest date that was compacted (YYYY-MM-DD format) */
  latest_compacted_date: string | null
}

/** Response from force recompaction */
export interface ForceRecompactResult {
  summaries_deleted: number
  hourly_compacted: number
  daily_compacted: number
  weekly_compacted: number
  monthly_compacted: number
  errors: string[]
  /** Latest date that was compacted (YYYY-MM-DD format) */
  latest_compacted_date: string | null
}

/** Options for force recompaction */
export interface ForceRecompactOptions {
  /** Only recompact summaries from this date (YYYY-MM-DD) */
  from_date?: string
  /** Only recompact summaries up to this date (YYYY-MM-DD) */
  to_date?: string
  /** Only recompact these scales (hourly, daily, weekly, monthly) */
  scales?: string[]
}

/**
 * Get worklog overview for a date range (daily summaries grouped by date + project)
 */
export async function getOverview(startDate: string, endDate: string): Promise<WorklogOverviewResponse> {
  return invokeAuth<WorklogOverviewResponse>('get_worklog_overview', {
    start_date: startDate,
    end_date: endDate,
  })
}

/**
 * Get hourly breakdown for a specific day and project
 */
export async function getHourlyBreakdown(date: string, projectPath: string): Promise<HourlyBreakdownItem[]> {
  return invokeAuth<HourlyBreakdownItem[]>('get_hourly_breakdown', {
    date,
    project_path: projectPath,
  })
}

/**
 * Trigger a compaction cycle to generate summaries from raw snapshots
 */
export async function triggerCompaction(): Promise<CompactionResult> {
  return invokeAuth<CompactionResult>('trigger_compaction', {})
}

/**
 * Force recalculate all work summaries from snapshot_raw_data.
 *
 * This operation:
 * 1. Deletes existing work_summaries entries (preserving original work_items and snapshot_raw_data)
 * 2. Re-runs the compaction cycle to regenerate all summaries
 *
 * Use this when you've made changes to the compaction logic and want to
 * retroactively apply them to historical data.
 */
export async function forceRecompact(options: ForceRecompactOptions = {}): Promise<ForceRecompactResult> {
  return invokeAuth<ForceRecompactResult>('force_recompact', {
    from_date: options.from_date,
    to_date: options.to_date,
    scales: options.scales,
  })
}
