/**
 * Tauri API Client
 *
 * This module provides direct communication with the Rust backend via Tauri commands.
 * It replaces HTTP API calls with invoke() for better performance and security.
 */

import { invoke } from '@tauri-apps/api/core'

// Types (matching Rust structs)

export interface UserResponse {
  id: string
  email: string
  name: string
  username?: string
  employee_id?: string
  department_id?: string
  title?: string
  gitlab_url?: string
  jira_email?: string
  is_active: boolean
  is_admin: boolean
  created_at: string
}

export interface AppStatus {
  has_users: boolean
  user_count: number
  first_user: UserResponse | null
  local_mode: boolean
}

export interface TokenResponse {
  access_token: string
  token_type: string
  expires_in: number
}

export interface RegisterRequest {
  username: string
  password: string
  name: string
  email?: string
  title?: string
}

export interface LoginRequest {
  username: string
  password: string
}

// Auth Commands

/**
 * Get app status (has_users, local_mode, etc.)
 */
export async function getAppStatus(): Promise<AppStatus> {
  return invoke<AppStatus>('get_app_status')
}

/**
 * Register a new user
 */
export async function registerUser(request: RegisterRequest): Promise<UserResponse> {
  return invoke<UserResponse>('register_user', { request })
}

/**
 * Login with username and password
 */
export async function login(request: LoginRequest): Promise<TokenResponse> {
  return invoke<TokenResponse>('login', { request })
}

/**
 * Auto-login for local mode (uses first user)
 */
export async function autoLogin(): Promise<TokenResponse> {
  return invoke<TokenResponse>('auto_login')
}

/**
 * Get current user by token
 */
export async function getCurrentUser(token: string): Promise<UserResponse> {
  return invoke<UserResponse>('get_current_user', { token })
}

// Config Types

export interface ConfigResponse {
  jira_url: string | null
  auth_type: string
  jira_configured: boolean
  tempo_configured: boolean
  llm_provider: string
  llm_model: string
  llm_base_url: string | null
  llm_configured: boolean
  daily_work_hours: number
  normalize_hours: boolean
  gitlab_url: string | null
  gitlab_configured: boolean
  use_git_mode: boolean
  git_repos: string[]
  outlook_enabled: boolean
}

export interface UpdateConfigRequest {
  daily_work_hours?: number
  normalize_hours?: boolean
}

export interface UpdateLlmConfigRequest {
  provider: string
  model: string
  api_key?: string
  base_url?: string
}

export interface UpdateJiraConfigRequest {
  jira_url?: string
  jira_pat?: string
  jira_email?: string
  jira_api_token?: string
  auth_type?: string
  tempo_api_token?: string
}

export interface MessageResponse {
  message: string
}

// Config Commands

/**
 * Get current user configuration
 */
export async function getConfig(token: string): Promise<ConfigResponse> {
  return invoke<ConfigResponse>('get_config', { token })
}

/**
 * Update general config settings
 */
export async function updateConfig(token: string, request: UpdateConfigRequest): Promise<MessageResponse> {
  return invoke<MessageResponse>('update_config', { token, request })
}

/**
 * Update LLM configuration
 */
export async function updateLlmConfig(token: string, request: UpdateLlmConfigRequest): Promise<MessageResponse> {
  return invoke<MessageResponse>('update_llm_config', { token, request })
}

/**
 * Update Jira configuration
 */
export async function updateJiraConfig(token: string, request: UpdateJiraConfigRequest): Promise<MessageResponse> {
  return invoke<MessageResponse>('update_jira_config', { token, request })
}

// Re-export for convenience
export const tauriApi = {
  // Auth
  getAppStatus,
  registerUser,
  login,
  autoLogin,
  getCurrentUser,
  // Config
  getConfig,
  updateConfig,
  updateLlmConfig,
  updateJiraConfig,
}
