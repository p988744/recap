/**
 * Authentication related types
 */

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

export interface UpdateProfileRequest {
  name?: string
  email?: string
  title?: string
  employee_id?: string
  department_id?: string
}
