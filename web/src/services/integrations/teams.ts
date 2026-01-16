/**
 * Teams service (HTTP only - no Tauri equivalents)
 */

import { getAuthToken } from '../client'
import type { Team } from '@/types'

const API_BASE = '/api'

/**
 * HTTP fetch helper
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

export interface TeamsResponse {
  teams: Team[]
  total: number
}

/**
 * Get all teams
 */
export async function getTeams(): Promise<TeamsResponse> {
  return fetchApi<TeamsResponse>('/config/teams')
}
