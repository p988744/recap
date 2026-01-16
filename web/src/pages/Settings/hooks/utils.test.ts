import { describe, it, expect } from 'vitest'
import { formatFileSize, formatTimestamp } from './utils'

describe('formatFileSize', () => {
  it('should format bytes correctly', () => {
    expect(formatFileSize(500)).toBe('500 B')
    expect(formatFileSize(0)).toBe('0 B')
  })

  it('should format kilobytes correctly', () => {
    expect(formatFileSize(1024)).toBe('1.0 KB')
    expect(formatFileSize(2048)).toBe('2.0 KB')
    expect(formatFileSize(1536)).toBe('1.5 KB')
  })

  it('should format megabytes correctly', () => {
    expect(formatFileSize(1024 * 1024)).toBe('1.0 MB')
    expect(formatFileSize(2.5 * 1024 * 1024)).toBe('2.5 MB')
  })
})

describe('formatTimestamp', () => {
  it('should return dash for undefined input', () => {
    expect(formatTimestamp(undefined)).toBe('-')
  })

  it('should return dash for empty string', () => {
    expect(formatTimestamp('')).toBe('-')
  })

  it('should format valid timestamp', () => {
    const result = formatTimestamp('2024-01-15T10:30:00Z')
    // The format includes month, day, hour, minute in zh-TW locale
    expect(result).toMatch(/\d/)
  })
})
