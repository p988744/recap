/**
 * Antigravity (Gemini Code) integration service
 */

import { invokeAuth } from '../client'
import type {
  AntigravityApiStatus,
  AntigravityProject,
  AntigravitySyncProjectsRequest,
  AntigravitySyncResult,
} from '@/types'

/**
 * Check if Antigravity is installed (directory exists)
 */
export async function checkInstalled(): Promise<boolean> {
  return invokeAuth<boolean>('check_antigravity_installed')
}

/**
 * Check Antigravity API status - returns URL and health check result
 */
export async function checkApiStatus(): Promise<AntigravityApiStatus> {
  return invokeAuth<AntigravityApiStatus>('check_antigravity_api_status')
}

/**
 * List all Antigravity sessions from local machine
 */
export async function listProjects(): Promise<AntigravityProject[]> {
  return invokeAuth<AntigravityProject[]>('list_antigravity_sessions')
}

/**
 * Sync selected projects - aggregate sessions by project+date
 */
export async function sync(request: AntigravitySyncProjectsRequest): Promise<AntigravitySyncResult> {
  return invokeAuth<AntigravitySyncResult>('sync_antigravity_projects', { request })
}
