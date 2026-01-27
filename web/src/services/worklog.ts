/**
 * Worklog service - queries for worklog overview and hourly breakdown
 */

import { invokeAuth } from './client'
import type { WorklogOverviewResponse, HourlyBreakdownItem } from '@/types/worklog'

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
