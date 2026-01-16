/**
 * API Client utilities
 *
 * Provides invoke wrapper and token management for Tauri commands.
 */

import { invoke } from '@tauri-apps/api/core'

const TOKEN_KEY = 'recap_auth_token'

/**
 * Get auth token from localStorage
 */
export function getAuthToken(): string | null {
  return localStorage.getItem(TOKEN_KEY)
}

/**
 * Set auth token in localStorage
 */
export function setAuthToken(token: string): void {
  localStorage.setItem(TOKEN_KEY, token)
}

/**
 * Remove auth token from localStorage
 */
export function removeAuthToken(): void {
  localStorage.removeItem(TOKEN_KEY)
}

/**
 * Get required auth token, redirects to login if not found
 */
export function getRequiredToken(): string {
  const token = getAuthToken()
  if (!token) {
    window.location.href = '/login'
    throw new Error('No auth token')
  }
  return token
}

/**
 * Invoke a Tauri command with type safety
 */
export async function invokeCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(command, args)
}

/**
 * Invoke an authenticated Tauri command
 * Automatically includes the auth token
 */
export async function invokeAuth<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const token = getRequiredToken()
  return invoke<T>(command, { token, ...args })
}
