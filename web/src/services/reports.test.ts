import { describe, it, expect, beforeEach } from 'vitest'
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

describe('reports service', () => {
  beforeEach(() => {
    resetTauriMock()
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

  // Analyze functions (now Tauri IPC)
  describe('analyzeWorkItems (Tauri)', () => {
    it('should analyze work items for a date range', async () => {
      mockCommandValue('analyze_work_items', mockAnalyzeResponse)

      const result = await reports.analyzeWorkItems('2024-01-01', '2024-01-07')

      expect(result).toEqual(mockAnalyzeResponse)
      expect(mockInvoke).toHaveBeenCalledWith('analyze_work_items', {
        token: 'test-token',
        query: { start_date: '2024-01-01', end_date: '2024-01-07' },
      })
    })

    it('should return analyze response with projects', async () => {
      mockCommandValue('analyze_work_items', mockAnalyzeResponse)

      const result = await reports.analyzeWorkItems('2024-01-01', '2024-01-07')

      expect(result.projects).toBeDefined()
      expect(result.total_minutes).toBeDefined()
      expect(result.total_hours).toBeDefined()
      expect(result.dates_covered).toBeDefined()
      expect(result.mode).toBeDefined()
    })
  })
})
