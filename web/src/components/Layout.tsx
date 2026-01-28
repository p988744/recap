import { NavLink, Outlet } from 'react-router-dom'
import {
  LayoutDashboard,
  Briefcase,
  Settings,
  User,
  HelpCircle,
  RefreshCw,
  CheckCircle2,
} from 'lucide-react'
import { cn } from '@/lib/utils'
import { useAuth } from '@/lib/auth'
import { Button } from '@/components/ui/button'
import { Onboarding, useOnboarding } from '@/components/Onboarding'
import { useAppSync, SyncProvider } from '@/hooks/useAppSync'

const navItems = [
  { to: '/', icon: LayoutDashboard, label: '儀表板' },
  { to: '/work-items', icon: Briefcase, label: '工作日誌' },
]

export function Layout() {
  const { user, token, isAuthenticated } = useAuth()
  const { showOnboarding, completeOnboarding, openOnboarding } = useOnboarding()

  // App-level background sync: starts service, listens for tray events, runs initial sync
  const syncValue = useAppSync(isAuthenticated, token)

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
            </div>
          )}

          {/* Sync status & help */}
          <div className="mt-2 px-3 py-2 border-t border-border">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                {syncValue.dataSyncState === 'syncing' || syncValue.summaryState === 'syncing' || syncValue.backendStatus?.is_syncing ? (
                  <>
                    <RefreshCw className="w-3 h-3 text-muted-foreground animate-spin" strokeWidth={1.5} />
                    <span className="text-[10px] text-muted-foreground">同步中...</span>
                  </>
                ) : (
                  <>
                    <CheckCircle2 className="w-3 h-3 text-sage" strokeWidth={1.5} />
                    <span className="text-[10px] text-muted-foreground">
                      {syncValue.backendStatus?.last_sync_at
                        ? `上次同步 ${new Date(syncValue.backendStatus.last_sync_at).toLocaleTimeString('zh-TW', { hour: '2-digit', minute: '2-digit' })}`
                        : '尚未同步'}
                    </span>
                  </>
                )}
              </div>
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
        </div>
      </aside>

      {/* Main content */}
      <main className="flex-1 ml-56">
        <SyncProvider value={syncValue}>
          <div className="px-12 py-10 max-w-5xl">
            <Outlet />
          </div>
        </SyncProvider>
      </main>

      {/* Global sync notification toast */}
      {syncValue.syncInfo && (
        <div className="fixed bottom-4 right-4 p-3 bg-sage text-white text-sm rounded-lg animate-fade-up shadow-md z-50">
          <div className="flex items-center gap-2">
            <CheckCircle2 className="w-4 h-4" strokeWidth={1.5} />
            {syncValue.syncInfo}
          </div>
        </div>
      )}

      {/* Onboarding tutorial */}
      <Onboarding open={showOnboarding} onComplete={completeOnboarding} />
    </div>
  )
}
