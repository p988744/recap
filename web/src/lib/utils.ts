import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

// Time-based greeting
export function getGreeting(): string {
  const hour = new Date().getHours()
  if (hour < 12) return '早安'
  if (hour < 18) return '午安'
  return '晚安'
}

// Format hours with appropriate precision
export function formatHours(hours: number): string {
  if (hours === 0) return '0h'
  if (hours < 1) return `${Math.round(hours * 60)}m`
  return `${hours.toFixed(1)}h`
}

// Format date in locale format
export function formatDate(date: Date | string): string {
  const d = typeof date === 'string' ? new Date(date) : date
  return d.toLocaleDateString('zh-TW', { month: 'short', day: 'numeric' })
}

// Format date with full detail
export function formatDateFull(date: Date | string): string {
  const d = typeof date === 'string' ? new Date(date) : date
  return d.toLocaleDateString('zh-TW', {
    year: 'numeric',
    month: 'long',
    day: 'numeric'
  })
}

// Get week progress (0-100%)
export function getWeekProgress(): number {
  const now = new Date()
  const day = now.getDay()
  const hour = now.getHours()
  // Week starts on Monday (1), ends on Friday (5)
  if (day === 0 || day === 6) return 100
  const workDays = day - 1 // Days completed (Mon=0, Tue=1, etc.)
  const dayProgress = Math.min(hour / 8, 1) // Assume 8-hour workday
  return Math.round(((workDays + dayProgress) / 5) * 100)
}
