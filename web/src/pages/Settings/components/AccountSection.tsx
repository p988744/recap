import { User, Cloud } from 'lucide-react'
import { Card } from '@/components/ui/card'
import type { UserResponse, AppStatus } from '@/types'

interface AccountSectionProps {
  user: UserResponse | null
  appStatus: AppStatus | null
}

export function AccountSection({ user, appStatus }: AccountSectionProps) {
  return (
    <section className="animate-fade-up opacity-0 delay-1">
      <h2 className="font-display text-2xl text-foreground mb-6">帳號</h2>

      <Card className="p-6">
        <div className="space-y-6">
          {/* Current account status */}
          <div className="flex items-center gap-4">
            <div className="w-12 h-12 rounded-full bg-foreground/10 flex items-center justify-center">
              <User className="w-6 h-6 text-foreground" strokeWidth={1.5} />
            </div>
            <div className="flex-1">
              <p className="text-sm font-medium text-foreground">{user?.name || '本地使用者'}</p>
              <p className="text-xs text-muted-foreground">{user?.email || '本地模式'}</p>
            </div>
            {appStatus?.local_mode && (
              <span className="px-2 py-1 text-xs bg-amber-100 text-amber-700 rounded">
                本地模式
              </span>
            )}
          </div>

          <div className="pt-4 border-t border-border">
            <p className="text-sm text-foreground mb-2">本地優先模式</p>
            <p className="text-xs text-muted-foreground leading-relaxed mb-4">
              目前 Recap 以本地模式運行，所有資料儲存在您的裝置上。
              未來將支援雲端同步功能，讓您可以在多台裝置間同步工作記錄。
            </p>
          </div>

          {/* Future cloud sync placeholder */}
          <div className="pt-4 border-t border-border">
            <div className="flex items-center gap-3 text-muted-foreground">
              <Cloud className="w-5 h-5" strokeWidth={1.5} />
              <div>
                <p className="text-sm">雲端同步</p>
                <p className="text-xs">即將推出</p>
              </div>
            </div>
          </div>

        </div>
      </Card>
    </section>
  )
}
