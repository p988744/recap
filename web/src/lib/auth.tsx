import { createContext, useContext, useState, useEffect, useCallback, ReactNode } from 'react'
import { auth, config } from '@/services'

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
  onboardingCompleted: boolean
  login: (username: string, password: string) => Promise<void>
  register: (username: string, password: string, name: string, email?: string, title?: string) => Promise<void>
  autoLogin: () => Promise<void>
  logout: () => void
  completeOnboarding: () => Promise<void>
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

// Provider component
export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null)
  const [token, setToken] = useState<string | null>(getStoredToken())
  const [isLoading, setIsLoading] = useState(true)
  const [appStatus, setAppStatus] = useState<AppStatus | null>(null)
  const [onboardingCompleted, setOnboardingCompleted] = useState(false)

  // Load app status and user on mount
  useEffect(() => {
    async function initialize() {
      try {
        // First, get app status via Tauri command
        const status = await auth.getAppStatus()
        setAppStatus(status)

        // If no users exist, we need onboarding first
        if (!status.has_users) {
          setOnboardingCompleted(false)
          setIsLoading(false)
          return
        }

        // Check for existing token
        const storedToken = getStoredToken()
        if (storedToken) {
          // Validate existing token
          try {
            const userData = await auth.getCurrentUser(storedToken)
            setUser(userData)
            setToken(storedToken)
            // Check onboarding status from DB
            await checkOnboardingStatus()
          } catch {
            // Token is invalid, try auto-login for local mode
            removeStoredToken()
            setToken(null)
            if (status.local_mode && status.has_users) {
              await performAutoLogin()
            }
          }
        } else if (status.local_mode && status.has_users) {
          // No token but local mode with existing users - auto login
          await performAutoLogin()
        }
      } catch (error) {
        console.error('Failed to initialize:', error)
        removeStoredToken()
        setToken(null)
      } finally {
        setIsLoading(false)
      }
    }

    async function checkOnboardingStatus() {
      try {
        const status = await config.getOnboardingStatus()
        setOnboardingCompleted(status.completed)
      } catch (error) {
        console.error('Failed to check onboarding status:', error)
        // Default to not completed if check fails
        setOnboardingCompleted(false)
      }
    }

    async function performAutoLogin() {
      try {
        const data = await auth.autoLogin()
        setStoredToken(data.access_token)
        setToken(data.access_token)

        // Fetch user info
        const userData = await auth.getCurrentUser(data.access_token)
        setUser(userData)

        // Check onboarding status from DB
        await checkOnboardingStatus()
      } catch (error) {
        console.error('Auto-login failed:', error)
      }
    }

    initialize()
  }, [])

  const login = async (username: string, password: string) => {
    try {
      const data = await auth.login({ username, password })
      setStoredToken(data.access_token)
      setToken(data.access_token)

      // Fetch user info
      const userData = await auth.getCurrentUser(data.access_token)
      setUser(userData)
    } catch (error) {
      throw new Error(error instanceof Error ? error.message : 'Login failed')
    }
  }

  const register = async (username: string, password: string, name: string, email?: string, title?: string) => {
    try {
      await auth.register({ username, password, name, email, title })

      // Update appStatus to reflect that we now have users
      setAppStatus(prev => prev ? { ...prev, has_users: true, user_count: prev.user_count + 1 } : prev)

      // Auto-login after registration
      await login(username, password)
    } catch (error) {
      throw new Error(error instanceof Error ? error.message : 'Registration failed')
    }
  }

  const logout = () => {
    removeStoredToken()
    setToken(null)
    setUser(null)
  }

  const autoLogin = async () => {
    try {
      const data = await auth.autoLogin()
      setStoredToken(data.access_token)
      setToken(data.access_token)

      // Fetch user info
      const userData = await auth.getCurrentUser(data.access_token)
      setUser(userData)

      // Check onboarding status
      try {
        const status = await config.getOnboardingStatus()
        setOnboardingCompleted(status.completed)
      } catch (err) {
        console.error('Failed to check onboarding status:', err)
      }
    } catch (error) {
      console.error('Auto-login failed:', error)
      throw error
    }
  }

  const completeOnboarding = useCallback(async () => {
    try {
      await config.completeOnboarding()
      setOnboardingCompleted(true)
    } catch (error) {
      console.error('Failed to complete onboarding:', error)
      throw error
    }
  }, [])

  // Needs onboarding if:
  // 1. No users exist (first time), or
  // 2. User exists but hasn't completed onboarding
  const needsOnboarding = appStatus !== null && (
    !appStatus.has_users || (!!user && !onboardingCompleted)
  )

  const value: AuthContextType = {
    user,
    token,
    isLoading,
    appStatus,
    onboardingCompleted,
    login,
    register,
    autoLogin,
    logout,
    completeOnboarding,
    isAuthenticated: !!user && !!token,
    needsOnboarding,
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

// Helper function to get auth headers (still needed for legacy HTTP API calls)
export function getAuthHeaders(): Record<string, string> {
  const token = getStoredToken()
  if (!token) return {}
  return { Authorization: `Bearer ${token}` }
}
