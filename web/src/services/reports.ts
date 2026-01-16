/**
 * Reports service
 */

import { invokeAuth } from './client'
import type {
  ReportQuery,
  PersonalReport,
  SummaryReport,
  CategoryReport,
  ExportResult,
  TempoReportQuery,
  TempoReport,
} from '@/types'

/**
 * Get personal report for date range
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
