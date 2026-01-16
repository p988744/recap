import { useEffect, useState, useCallback } from 'react'
import { reports } from '@/services'
import type { AnalyzeResponse, LegacyPersonalReport, PEReport, TempoReport, TempoReportPeriod } from '@/types'

// =============================================================================
// Types
// =============================================================================

export type ReportPeriod = 'week' | 'last-week' | '7days' | '30days'
export type ReportTab = 'work' | 'pe' | 'tempo'

// =============================================================================
// Main Hook: useReports
// =============================================================================

export function useReports(isAuthenticated: boolean, token: string | null) {
  const [data, setData] = useState<AnalyzeResponse | null>(null)
  const [personalReport, setPersonalReport] = useState<LegacyPersonalReport | null>(null)
  const [peReport, setPEReport] = useState<PEReport | null>(null)
  const [loading, setLoading] = useState(true)
  const [period, setPeriod] = useState<ReportPeriod>('week')
  const [activeTab, setActiveTab] = useState<ReportTab>('work')
  const [peYear, setPEYear] = useState(new Date().getFullYear())
  const [peHalf, setPEHalf] = useState<1 | 2>(new Date().getMonth() < 6 ? 1 : 2)
  const [tempoReport, setTempoReport] = useState<TempoReport | null>(null)
  const [tempoPeriod, setTempoPeriod] = useState<TempoReportPeriod>('weekly')
  const [tempoLoading, setTempoLoading] = useState(false)

  const fetchReport = useCallback(async (p: ReportPeriod) => {
    setLoading(true)
    try {
      let result: AnalyzeResponse
      switch (p) {
        case 'week':
          result = await reports.analyzeWeek()
          break
        case 'last-week':
          result = await reports.analyzeLastWeek()
          break
        case '7days':
          result = await reports.analyzeDays(7)
          break
        case '30days':
          result = await reports.analyzeDays(30)
          break
        default:
          result = await reports.analyzeWeek()
      }
      setData(result)

      // Also fetch personal report for the same date range
      if (result.start_date && result.end_date) {
        try {
          const personal = await reports.getLegacyPersonalReport(result.start_date, result.end_date)
          setPersonalReport(personal)
        } catch (err) {
          console.error('Failed to fetch personal report:', err)
        }
      }
    } catch (err) {
      console.error('Failed to fetch report:', err)
    } finally {
      setLoading(false)
    }
  }, [])

  const fetchPEReport = useCallback(async () => {
    setLoading(true)
    try {
      const result = await reports.getPEReport(peYear, peHalf)
      setPEReport(result)
    } catch (err) {
      console.error('Failed to fetch PE report:', err)
    } finally {
      setLoading(false)
    }
  }, [peYear, peHalf])

  const fetchTempoReport = useCallback(async (p: TempoReportPeriod) => {
    setTempoLoading(true)
    try {
      const result = await reports.generateTempoReport({ period: p })
      setTempoReport(result)
    } catch (err) {
      console.error('Failed to fetch tempo report:', err)
    } finally {
      setTempoLoading(false)
    }
  }, [])

  useEffect(() => {
    if (!isAuthenticated || !token) {
      return
    }
    if (activeTab === 'work') {
      fetchReport(period)
    } else if (activeTab === 'pe') {
      fetchPEReport()
    } else if (activeTab === 'tempo') {
      fetchTempoReport(tempoPeriod)
    }
  }, [period, activeTab, peYear, peHalf, tempoPeriod, isAuthenticated, token, fetchReport, fetchPEReport, fetchTempoReport])

  const refresh = useCallback(() => {
    if (activeTab === 'work') fetchReport(period)
    else if (activeTab === 'pe') fetchPEReport()
    else if (activeTab === 'tempo') fetchTempoReport(tempoPeriod)
  }, [activeTab, period, tempoPeriod, fetchReport, fetchPEReport, fetchTempoReport])

  return {
    // Work report state
    data,
    personalReport,
    loading,
    period,
    setPeriod,
    // Tab state
    activeTab,
    setActiveTab,
    // PE report state
    peReport,
    peYear,
    setPEYear,
    peHalf,
    setPEHalf,
    // Tempo report state
    tempoReport,
    tempoPeriod,
    setTempoPeriod,
    tempoLoading,
    // Actions
    refresh,
    fetchReport,
    fetchPEReport,
    fetchTempoReport,
  }
}

// =============================================================================
// Report Generation Utils
// =============================================================================

export function generateWorkReport(data: AnalyzeResponse | null): string {
  if (!data) return ''

  const formatHours = (minutes: number) => (minutes / 60).toFixed(1)

  const lines = [
    `工作報告：${data.start_date} ~ ${data.end_date}`,
    '',
    `總工時：${formatHours(data.total_minutes)} 小時`,
    `工作天數：${data.dates_covered.length} 天`,
    `專案數：${data.projects.length}`,
    '',
    '## 專案明細',
    '',
  ]

  data.projects.forEach((project, index) => {
    lines.push(`### ${index + 1}. ${project.project_name}`)
    lines.push(`- 總時數：${formatHours(project.total_minutes)} 小時`)
    if (project.jira_id) {
      lines.push(`- Jira Issue：${project.jira_id}`)
    }
    lines.push('')

    project.daily_entries.forEach((entry) => {
      lines.push(`  - ${entry.date}：${entry.hours.toFixed(1)}h`)
      if (entry.description) {
        lines.push(`    ${entry.description}`)
      }
    })
    lines.push('')
  })

  return lines.join('\n')
}

export function generateTempoReportText(tempoReport: TempoReport): string {
  const lines = [
    `Tempo 工作報告：${tempoReport.period}`,
    `期間：${tempoReport.start_date} ~ ${tempoReport.end_date}`,
    '',
    `總工時：${tempoReport.total_hours.toFixed(1)} 小時`,
    `工作項目：${tempoReport.total_items} 筆`,
    '',
    '## 專案明細',
    '',
  ]
  tempoReport.projects.forEach((project) => {
    lines.push(`### ${project.project}`)
    lines.push(`- 工時：${project.hours.toFixed(1)} 小時`)
    lines.push(`- 項目數：${project.item_count}`)
    if (project.summaries.length > 0) {
      lines.push('- 主要工作：')
      project.summaries.forEach(s => lines.push(`  - ${s}`))
    }
    lines.push('')
  })
  return lines.join('\n')
}
