import { createContext, useContext, useState, useEffect, ReactNode } from 'react'

// Types
export interface User {
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
  first_user: User | null
  local_mode: boolean
}

interface AuthContextType {
  user: User | null
  token: string | null
  isLoading: boolean
  appStatus: AppStatus | null
  login: (username: string, password: string) => Promise<void>
  register: (username: string, password: string, name: string, email?: string, title?: string) => Promise<void>
  autoLogin: () => Promise<void>
  logout: () => void
  isAuthenticated: boolean
  needsOnboarding: boolean
}

const AuthContext = createContext<AuthContextType | undefined>(undefined)

// Token storage
const TOKEN_KEY = 'recap_auth_token'

function getStoredToken(): string | null {
  return localStorage.getItem(TOKEN_KEY)
}

function setStoredToken(token: string): void {
  localStorage.setItem(TOKEN_KEY, token)
}

function removeStoredToken(): void {
  localStorage.removeItem(TOKEN_KEY)
}

// Auth API calls
async function fetchWithAuth(endpoint: string, options?: RequestInit): Promise<Response> {
  const token = getStoredToken()
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...(options?.headers as Record<string, string>),
  }

  if (token) {
    headers['Authorization'] = `Bearer ${token}`
  }

  return fetch(`/api${endpoint}`, {
    ...options,
    headers,
  })
}

// Provider component
export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null)
  const [token, setToken] = useState<string | null>(getStoredToken())
  const [isLoading, setIsLoading] = useState(true)
  const [appStatus, setAppStatus] = useState<AppStatus | null>(null)

  // Load app status and user on mount
  useEffect(() => {
    async function initialize() {
      try {
        // First, get app status
        const statusResponse = await fetch('/api/auth/status')
        if (statusResponse.ok) {
          const status = await statusResponse.json()
          setAppStatus(status)

          // If no users, we need onboarding
          if (!status.has_users) {
            setIsLoading(false)
            return
          }

          // Check for existing token
          const storedToken = getStoredToken()
          if (storedToken) {
            // Validate existing token
            const response = await fetchWithAuth('/auth/me')
            if (response.ok) {
              const userData = await response.json()
              setUser(userData)
              setToken(storedToken)
            } else {
              // Token is invalid, try auto-login for local mode
              removeStoredToken()
              setToken(null)
              if (status.local_mode && status.has_users) {
                await performAutoLogin()
              }
            }
          } else if (status.local_mode && status.has_users) {
            // No token but local mode with users - auto login
            await performAutoLogin()
          }
        }
      } catch (error) {
        console.error('Failed to initialize:', error)
        removeStoredToken()
        setToken(null)
      } finally {
        setIsLoading(false)
      }
    }

    async function performAutoLogin() {
      try {
        const response = await fetch('/api/auth/auto-login', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
        })
        if (response.ok) {
          const data = await response.json()
          setStoredToken(data.access_token)
          setToken(data.access_token)

          // Fetch user info
          const userResponse = await fetchWithAuth('/auth/me')
          if (userResponse.ok) {
            const userData = await userResponse.json()
            setUser(userData)
          }
        }
      } catch (error) {
        console.error('Auto-login failed:', error)
      }
    }

    initialize()
  }, [])

  const login = async (username: string, password: string) => {
    const response = await fetch('/api/auth/login', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ username, password }),
    })

    if (!response.ok) {
      const error = await response.json()
      throw new Error(error.detail || 'Login failed')
    }

    const data = await response.json()
    setStoredToken(data.access_token)
    setToken(data.access_token)

    // Fetch user info
    const userResponse = await fetchWithAuth('/auth/me')
    if (userResponse.ok) {
      const userData = await userResponse.json()
      setUser(userData)
    }
  }

  const register = async (username: string, password: string, name: string, email?: string, title?: string) => {
    const response = await fetch('/api/auth/register', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ username, password, name, email, title }),
    })

    if (!response.ok) {
      const error = await response.json()
      throw new Error(error.detail || 'Registration failed')
    }

    // Update appStatus to reflect that we now have users
    setAppStatus(prev => prev ? { ...prev, has_users: true, user_count: prev.user_count + 1 } : prev)

    // Auto-login after registration
    await login(username, password)
  }

  const logout = () => {
    removeStoredToken()
    setToken(null)
    setUser(null)
  }

  const autoLogin = async () => {
    try {
      const response = await fetch('/api/auth/auto-login', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
      })
      if (response.ok) {
        const data = await response.json()
        setStoredToken(data.access_token)
        setToken(data.access_token)

        // Fetch user info
        const userResponse = await fetchWithAuth('/auth/me')
        if (userResponse.ok) {
          const userData = await userResponse.json()
          setUser(userData)
        }
      } else {
        throw new Error('Auto-login failed')
      }
    } catch (error) {
      console.error('Auto-login failed:', error)
      throw error
    }
  }

  const value: AuthContextType = {
    user,
    token,
    isLoading,
    appStatus,
    login,
    register,
    autoLogin,
    logout,
    isAuthenticated: !!user && !!token,
    needsOnboarding: appStatus !== null && !appStatus.has_users,
  }

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}

// Hook for using auth
export function useAuth() {
  const context = useContext(AuthContext)
  if (context === undefined) {
    throw new Error('useAuth must be used within an AuthProvider')
  }
  return context
}

// Helper function to get auth headers
export function getAuthHeaders(): Record<string, string> {
  const token = getStoredToken()
  if (!token) return {}
  return { Authorization: `Bearer ${token}` }
}
