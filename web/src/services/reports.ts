/**
 * Reports service
 */

import { invokeAuth, getAuthToken } from './client'
import type {
  ReportQuery,
  PersonalReport,
  SummaryReport,
  CategoryReport,
  ExportResult,
  TempoReportQuery,
  TempoReport,
  AnalyzeResponse,
  LegacyPersonalReport,
  PEReport,
} from '@/types'

const API_BASE = '/api'

/**
 * HTTP fetch helper for legacy APIs
 */
async function fetchApi<T>(endpoint: string, options?: RequestInit): Promise<T> {
  const token = getAuthToken()
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...(options?.headers as Record<string, string>),
  }

  if (token) {
    headers['Authorization'] = `Bearer ${token}`
  }

  const response = await fetch(`${API_BASE}${endpoint}`, {
    ...options,
    headers,
  })

  if (!response.ok) {
    if (response.status === 401) {
      localStorage.removeItem('recap_auth_token')
      window.location.href = '/login'
      throw new Error('Session expired. Please login again.')
    }

    const error = await response.json().catch(() => ({ detail: 'Unknown error' }))
    throw new Error(error.detail || error.error || 'API request failed')
  }

  return response.json()
}

// ============================================================================
// Analyze functions (HTTP only - no Tauri equivalents)
// ============================================================================

/**
 * Analyze work for current week
 */
export async function analyzeWeek(useGit?: boolean): Promise<AnalyzeResponse> {
  const params = useGit !== undefined ? `?use_git=${useGit}` : ''
  return fetchApi<AnalyzeResponse>(`/analyze/week${params}`)
}

/**
 * Analyze work for last week
 */
export async function analyzeLastWeek(useGit?: boolean): Promise<AnalyzeResponse> {
  const params = useGit !== undefined ? `?use_git=${useGit}` : ''
  return fetchApi<AnalyzeResponse>(`/analyze/last-week${params}`)
}

/**
 * Analyze work for specific number of days
 */
export async function analyzeDays(days: number, useGit?: boolean): Promise<AnalyzeResponse> {
  const params = useGit !== undefined ? `?use_git=${useGit}` : ''
  return fetchApi<AnalyzeResponse>(`/analyze/days/${days}${params}`)
}

/**
 * Analyze work for date range
 */
export async function analyzeRange(startDate: string, endDate: string, useGit?: boolean): Promise<AnalyzeResponse> {
  return fetchApi<AnalyzeResponse>('/analyze', {
    method: 'POST',
    body: JSON.stringify({ start_date: startDate, end_date: endDate, use_git: useGit }),
  })
}

// ============================================================================
// Tauri-based reports
// ============================================================================

/**
 * Get personal report for date range (Tauri)
 */
export async function getPersonalReport(query: ReportQuery): Promise<PersonalReport> {
  return invokeAuth<PersonalReport>('get_personal_report', { query })
}

/**
 * Get summary report
 */
export async function getSummaryReport(query: ReportQuery): Promise<SummaryReport> {
  return invokeAuth<SummaryReport>('get_summary_report', { query })
}

/**
 * Get report grouped by category
 */
export async function getCategoryReport(query: ReportQuery): Promise<CategoryReport> {
  return invokeAuth<CategoryReport>('get_category_report', { query })
}

/**
 * Get report grouped by source
 */
export async function getSourceReport(query: ReportQuery): Promise<CategoryReport> {
  return invokeAuth<CategoryReport>('get_source_report', { query })
}

/**
 * Export work items to Excel file
 */
export async function exportExcel(query: ReportQuery): Promise<ExportResult> {
  return invokeAuth<ExportResult>('export_excel_report', { query })
}

/**
 * Generate smart Tempo report with LLM summaries
 */
export async function generateTempoReport(query: TempoReportQuery): Promise<TempoReport> {
  return invokeAuth<TempoReport>('generate_tempo_report', { query })
}

// ============================================================================
// Legacy HTTP reports (no Tauri equivalents)
// ============================================================================

/**
 * Get legacy personal report (HTTP)
 */
export async function getLegacyPersonalReport(startDate: string, endDate: string): Promise<LegacyPersonalReport> {
  return fetchApi<LegacyPersonalReport>(`/reports/personal?start_date=${startDate}&end_date=${endDate}`)
}

/**
 * Get PE (Performance Evaluation) report (HTTP only)
 */
export async function getPEReport(year: number, half: 1 | 2): Promise<PEReport> {
  return fetchApi<PEReport>(`/reports/pe?year=${year}&half=${half}`)
}

/**
 * Export report as Markdown (HTTP only)
 */
export async function exportMarkdownReport(startDate: string, endDate: string): Promise<string> {
  const token = getAuthToken()
  const response = await fetch(`${API_BASE}/reports/export/markdown?start_date=${startDate}&end_date=${endDate}`, {
    headers: token ? { Authorization: `Bearer ${token}` } : {},
  })
  if (!response.ok) throw new Error('Export failed')
  return response.text()
}
