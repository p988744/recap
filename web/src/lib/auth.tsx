import { createContext, useContext, useState, useEffect, ReactNode } from 'react'
import { auth } from '@/services'

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
        // First, get app status via Tauri command
        const status = await auth.getAppStatus()
        setAppStatus(status)

        // 本地模式：如果沒有用戶，自動建立預設用戶
        if (!status.has_users) {
          await createDefaultLocalUser()
          // 重新取得狀態
          const newStatus = await auth.getAppStatus()
          setAppStatus(newStatus)
        }

        // Check for existing token
        const storedToken = getStoredToken()
        if (storedToken) {
          // Validate existing token
          try {
            const userData = await auth.getCurrentUser(storedToken)
            setUser(userData)
            setToken(storedToken)
          } catch {
            // Token is invalid, try auto-login for local mode
            removeStoredToken()
            setToken(null)
            if (status.local_mode) {
              await performAutoLogin()
            }
          }
        } else if (status.local_mode) {
          // No token but local mode - auto login
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

    // 自動建立本地預設用戶
    async function createDefaultLocalUser() {
      try {
        await auth.register({
          username: 'local',
          password: 'local',
          name: '本地使用者',
          email: 'local@localhost',
        })
        console.log('Created default local user')
      } catch (error) {
        console.error('Failed to create default user:', error)
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

// Helper function to get auth headers (still needed for legacy HTTP API calls)
export function getAuthHeaders(): Record<string, string> {
  const token = getStoredToken()
  if (!token) return {}
  return { Authorization: `Bearer ${token}` }
}
