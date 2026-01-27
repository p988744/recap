import { NavLink, Outlet, useNavigate } from 'react-router-dom'
import {
  LayoutDashboard,
  Briefcase,
  FileText,
  Users,
  Activity,
  Settings,
  LogOut,
  User,
  HelpCircle,
} from 'lucide-react'
import { cn } from '@/lib/utils'
import { useAuth } from '@/lib/auth'
import { Button } from '@/components/ui/button'
import { Onboarding, useOnboarding } from '@/components/Onboarding'
import { useAppSync } from '@/hooks/useAppSync'

const navItems = [
  { to: '/', icon: LayoutDashboard, label: '儀表板' },
  { to: '/work-items', icon: Briefcase, label: '工作日誌' },
  { to: '/reports', icon: FileText, label: '報告中心' },
  { to: '/team', icon: Users, label: '團隊管理' },
  { to: '/llm-usage', icon: Activity, label: 'LLM 用量' },
]

export function Layout() {
  const { user, token, isAuthenticated, logout } = useAuth()
  const navigate = useNavigate()
  const { showOnboarding, completeOnboarding, openOnboarding } = useOnboarding()

  // App-level background sync: starts service, listens for tray events, runs initial sync
  useAppSync(isAuthenticated, token)

  const handleLogout = () => {
    logout()
    navigate('/login')
  }

  return (
    <div className="min-h-screen bg-background flex">
      {/* Sidebar - Editorial style */}
      <aside className="w-56 bg-white/40 backdrop-blur-sm border-r border-border flex flex-col fixed h-full">
        {/* Logo */}
        <div className="px-6 py-8">
          <h1 className="font-display text-2xl text-foreground tracking-tight">Recap</h1>
          <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mt-1">自動回顧你的工作</p>
        </div>

        {/* Divider */}
        <div className="mx-6 h-px bg-charcoal/6" />

        {/* Navigation */}
        <nav className="flex-1 px-4 py-6">
          <ul className="space-y-1">
            {navItems.map((item) => (
              <li key={item.to}>
                <NavLink
                  to={item.to}
                  className={({ isActive }) =>
                    cn(
                      "flex items-center gap-3 px-3 py-2.5 text-sm transition-all duration-200",
                      isActive
                        ? "text-foreground border-l-2 border-l-charcoal -ml-px font-medium"
                        : "text-muted-foreground hover:text-foreground"
                    )
                  }
                >
                  <item.icon className="w-4 h-4" strokeWidth={1.5} />
                  <span>{item.label}</span>
                </NavLink>
              </li>
            ))}
          </ul>
        </nav>

        {/* Footer */}
        <div className="p-4 border-t border-border">
          <NavLink
            to="/settings"
            className={({ isActive }) =>
              cn(
                "flex items-center gap-3 px-3 py-2.5 text-sm transition-all duration-200",
                isActive
                  ? "text-foreground border-l-2 border-l-charcoal -ml-px font-medium"
                  : "text-muted-foreground hover:text-foreground"
              )
            }
          >
            <Settings className="w-4 h-4" strokeWidth={1.5} />
            <span>設定</span>
          </NavLink>

          {/* User info */}
          {user && (
            <div className="mt-4 px-3 py-3 border-t border-border">
              <div className="flex items-center gap-2 mb-1">
                <User className="w-3.5 h-3.5 text-muted-foreground" strokeWidth={1.5} />
                <span className="text-sm text-foreground truncate">
                  {user.name}
                </span>
              </div>
              <p className="text-[10px] text-muted-foreground truncate ml-5">{user.email}</p>
              <Button
                variant="ghost"
                size="sm"
                onClick={handleLogout}
                className="w-full mt-3 text-muted-foreground hover:text-foreground justify-start px-0"
              >
                <LogOut className="w-3.5 h-3.5 mr-2" strokeWidth={1.5} />
                登出
              </Button>
            </div>
          )}

          <div className="mt-2 px-3 flex items-center justify-between">
            <p className="text-[10px] text-muted-foreground">Recap v2.0.0</p>
            <Button
              variant="ghost"
              size="icon"
              className="h-6 w-6 text-muted-foreground hover:text-foreground"
              onClick={openOnboarding}
              title="使用教學"
            >
              <HelpCircle className="w-3.5 h-3.5" strokeWidth={1.5} />
            </Button>
          </div>
        </div>
      </aside>

      {/* Main content */}
      <main className="flex-1 ml-56">
        <div className="px-12 py-10 max-w-5xl">
          <Outlet />
        </div>
      </main>

      {/* Onboarding tutorial */}
      <Onboarding open={showOnboarding} onComplete={completeOnboarding} />
    </div>
  )
}
