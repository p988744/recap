/**
 * Authentication service
 */

import { invokeCommand, invokeAuth } from './client'
import type {
  UserResponse,
  AppStatus,
  TokenResponse,
  RegisterRequest,
  LoginRequest,
  UpdateProfileRequest,
} from '@/types'

/**
 * Get app status (has_users, local_mode, etc.)
 */
export async function getAppStatus(): Promise<AppStatus> {
  return invokeCommand<AppStatus>('get_app_status')
}

/**
 * Register a new user
 */
export async function register(request: RegisterRequest): Promise<UserResponse> {
  return invokeCommand<UserResponse>('register_user', { request })
}

/**
 * Login with username and password
 */
export async function login(request: LoginRequest): Promise<TokenResponse> {
  return invokeCommand<TokenResponse>('login', { request })
}

/**
 * Auto-login for local mode (uses first user)
 */
export async function autoLogin(): Promise<TokenResponse> {
  return invokeCommand<TokenResponse>('auto_login')
}

/**
 * Get current user by token
 */
export async function getCurrentUser(): Promise<UserResponse> {
  return invokeAuth<UserResponse>('get_current_user')
}

/**
 * Get user profile
 */
export async function getProfile(): Promise<UserResponse> {
  return invokeAuth<UserResponse>('get_profile')
}

/**
 * Update user profile
 */
export async function updateProfile(request: UpdateProfileRequest): Promise<UserResponse> {
  return invokeAuth<UserResponse>('update_profile', { request })
}
