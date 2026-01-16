import { describe, it, expect, beforeEach, vi } from 'vitest'
import {
  mockInvoke,
  mockCommandValue,
  resetTauriMock,
} from '@/test/mocks/tauri'
import {
  mockPersonalReport,
  mockTempoReport,
  mockAnalyzeResponse,
} from '@/test/fixtures'
import * as reports from './reports'

// Mock fetch for HTTP-based functions
const mockFetch = vi.fn()
global.fetch = mockFetch

describe('reports service', () => {
  beforeEach(() => {
    resetTauriMock()
    mockFetch.mockReset()
    localStorage.clear()
    localStorage.setItem('recap_auth_token', 'test-token')
  })

  // Tauri-based functions
  describe('getPersonalReport (Tauri)', () => {
    it('should get personal report for date range', async () => {
      mockCommandValue('get_personal_report', mockPersonalReport)

      const query = {
        start_date: '2024-01-01',
        end_date: '2024-01-15',
      }
      const result = await reports.getPersonalReport(query)

      expect(result).toEqual(mockPersonalReport)
      expect(mockInvoke).toHaveBeenCalledWith('get_personal_report', {
        token: 'test-token',
        query,
      })
    })
  })

  describe('generateTempoReport (Tauri)', () => {
    it('should generate tempo report with period', async () => {
      mockCommandValue('generate_tempo_report', mockTempoReport)

      const query = { period: 'weekly' as const }
      const result = await reports.generateTempoReport(query)

      expect(result).toEqual(mockTempoReport)
      expect(result.total_hours).toBe(40.0)
      expect(result.projects).toHaveLength(2)
    })

    it('should generate tempo report with specific date', async () => {
      mockCommandValue('generate_tempo_report', mockTempoReport)

      const query = { period: 'daily' as const, date: '2024-01-15' }
      await reports.generateTempoReport(query)

      expect(mockInvoke).toHaveBeenCalledWith('generate_tempo_report', {
        token: 'test-token',
        query,
      })
    })
  })

  // HTTP-based functions
  describe('analyzeWeek (HTTP)', () => {
    it('should analyze current week', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => mockAnalyzeResponse,
      })

      const result = await reports.analyzeWeek()

      expect(result).toEqual(mockAnalyzeResponse)
      expect(mockFetch).toHaveBeenCalledWith(
        '/api/analyze/week',
        expect.objectContaining({
          headers: expect.objectContaining({
            Authorization: 'Bearer test-token',
          }),
        })
      )
    })

    it('should analyze with useGit parameter', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => mockAnalyzeResponse,
      })

      await reports.analyzeWeek(true)

      expect(mockFetch).toHaveBeenCalledWith(
        '/api/analyze/week?use_git=true',
        expect.any(Object)
      )
    })
  })

  describe('analyzeLastWeek (HTTP)', () => {
    it('should analyze last week', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => mockAnalyzeResponse,
      })

      const result = await reports.analyzeLastWeek()

      expect(result).toEqual(mockAnalyzeResponse)
      expect(mockFetch).toHaveBeenCalledWith(
        '/api/analyze/last-week',
        expect.any(Object)
      )
    })
  })

  describe('analyzeDays (HTTP)', () => {
    it('should analyze specific number of days', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => mockAnalyzeResponse,
      })

      const result = await reports.analyzeDays(7)

      expect(result).toEqual(mockAnalyzeResponse)
      expect(mockFetch).toHaveBeenCalledWith(
        '/api/analyze/days/7',
        expect.any(Object)
      )
    })

    it('should handle 30 days analysis', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => mockAnalyzeResponse,
      })

      await reports.analyzeDays(30)

      expect(mockFetch).toHaveBeenCalledWith(
        '/api/analyze/days/30',
        expect.any(Object)
      )
    })
  })

  describe('getPEReport (HTTP)', () => {
    it('should get PE report for year and half', async () => {
      const mockPEReport = {
        user_name: 'Test User',
        evaluation_period: '2024 H1',
        total_hours: 500,
        work_results: [],
        skills: [],
        goal_progress: [],
        jira_issues_count: 50,
        commits_count: 200,
        merge_requests_count: 30,
      }
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => mockPEReport,
      })

      const result = await reports.getPEReport(2024, 1)

      expect(result).toEqual(mockPEReport)
      expect(mockFetch).toHaveBeenCalledWith(
        '/api/reports/pe?year=2024&half=1',
        expect.any(Object)
      )
    })
  })

  describe('exportMarkdownReport (HTTP)', () => {
    it('should export markdown report', async () => {
      const markdownContent = '# Work Report\n\n## Summary\n...'
      mockFetch.mockResolvedValueOnce({
        ok: true,
        text: async () => markdownContent,
      })

      const result = await reports.exportMarkdownReport('2024-01-01', '2024-01-15')

      expect(result).toBe(markdownContent)
      expect(mockFetch).toHaveBeenCalledWith(
        '/api/reports/export/markdown?start_date=2024-01-01&end_date=2024-01-15',
        expect.objectContaining({
          headers: { Authorization: 'Bearer test-token' },
        })
      )
    })

    it('should throw on export failure', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 500,
      })

      await expect(
        reports.exportMarkdownReport('2024-01-01', '2024-01-15')
      ).rejects.toThrow('Export failed')
    })
  })

  describe('getLegacyPersonalReport (HTTP)', () => {
    it('should get legacy personal report', async () => {
      const legacyReport = {
        user_name: 'Test User',
        user_email: 'test@example.com',
        start_date: '2024-01-01',
        end_date: '2024-01-15',
        total_hours: 45.5,
        work_items: [],
        daily_breakdown: [],
        category_breakdown: { development: 30, review: 15.5 },
        jira_issues: {},
        source_breakdown: {},
      }
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => legacyReport,
      })

      const result = await reports.getLegacyPersonalReport('2024-01-01', '2024-01-15')

      expect(result).toEqual(legacyReport)
      expect(mockFetch).toHaveBeenCalledWith(
        '/api/reports/personal?start_date=2024-01-01&end_date=2024-01-15',
        expect.any(Object)
      )
    })
  })

  // Error handling
  describe('error handling', () => {
    it('should handle 401 unauthorized', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 401,
      })

      await expect(reports.analyzeWeek()).rejects.toThrow('Session expired')
      expect(localStorage.getItem('recap_auth_token')).toBeNull()
      expect(window.location.href).toBe('/login')
    })

    it('should handle API errors with detail', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 400,
        json: async () => ({ detail: 'Invalid date range' }),
      })

      await expect(reports.analyzeWeek()).rejects.toThrow('Invalid date range')
    })
  })
})
