/**
 * LLM Usage service
 *
 * Provides functions for querying LLM token usage statistics.
 */

import { invokeAuth } from './client'
import type { LlmUsageStats, DailyUsage, ModelUsage, LlmUsageLog } from '@/types'

export async function getUsageStats(startDate: string, endDate: string): Promise<LlmUsageStats> {
  return invokeAuth<LlmUsageStats>('get_llm_usage_stats', {
    start_date: startDate,
    end_date: endDate,
  })
}

export async function getUsageDaily(startDate: string, endDate: string): Promise<DailyUsage[]> {
  return invokeAuth<DailyUsage[]>('get_llm_usage_daily', {
    start_date: startDate,
    end_date: endDate,
  })
}

export async function getUsageByModel(startDate: string, endDate: string): Promise<ModelUsage[]> {
  return invokeAuth<ModelUsage[]>('get_llm_usage_by_model', {
    start_date: startDate,
    end_date: endDate,
  })
}

export async function getUsageLogs(
  startDate: string,
  endDate: string,
  limit?: number,
  offset?: number,
): Promise<LlmUsageLog[]> {
  return invokeAuth<LlmUsageLog[]>('get_llm_usage_logs', {
    start_date: startDate,
    end_date: endDate,
    limit: limit ?? 50,
    offset: offset ?? 0,
  })
}
