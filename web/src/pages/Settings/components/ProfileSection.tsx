import { Save, Loader2, User, LogOut } from 'lucide-react'
import { Card } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import type { UserResponse } from '@/types'
import type { SettingsMessage } from '../hooks/useSettings'

interface ProfileSectionProps {
  profileName: string
  setProfileName: (v: string) => void
  profileEmail: string
  setProfileEmail: (v: string) => void
  profileTitle: string
  setProfileTitle: (v: string) => void
  profileEmployeeId: string
  setProfileEmployeeId: (v: string) => void
  profileDepartment: string
  setProfileDepartment: (v: string) => void
  saving: boolean
  onSave: (setMessage: (msg: SettingsMessage | null) => void) => Promise<void>
  setMessage: (msg: SettingsMessage | null) => void
  user: UserResponse | null
  onLogout: () => void
}

export function ProfileSection({
  profileName,
  setProfileName,
  profileEmail,
  setProfileEmail,
  profileTitle,
  setProfileTitle,
  profileEmployeeId,
  setProfileEmployeeId,
  profileDepartment,
  setProfileDepartment,
  saving,
  onSave,
  setMessage,
  user,
  onLogout,
}: ProfileSectionProps) {
  return (
    <section className="animate-fade-up opacity-0 delay-1 space-y-8">
      <h2 className="font-display text-2xl text-foreground mb-6">帳號</h2>

      {/* Account Info */}
      <Card className="p-6">
        <div className="space-y-6">
          <div className="flex items-center gap-4">
            <div className="w-12 h-12 rounded-full bg-foreground/10 flex items-center justify-center">
              <User className="w-6 h-6 text-foreground" strokeWidth={1.5} />
            </div>
            <div className="flex-1">
              <p className="text-sm font-medium text-foreground">{user?.name || '本地使用者'}</p>
              <p className="text-xs text-muted-foreground">{user?.email || '本地模式'}</p>
            </div>
          </div>

          <div className="pt-4 border-t border-border">
            <Button variant="outline" onClick={onLogout} className="text-destructive hover:text-destructive">
              <LogOut className="w-4 h-4" strokeWidth={1.5} />
              登出
            </Button>
            <p className="text-xs text-muted-foreground mt-2">
              登出後將清除本地登入狀態，重新啟動 App 會自動登入。
            </p>
          </div>
        </div>
      </Card>

      {/* Profile Form */}
      <Card className="p-6">
        <h3 className="font-medium text-foreground mb-4">個人資料</h3>
        <div className="space-y-6">
          <div>
            <Label htmlFor="profile-name" className="mb-2 block">名稱</Label>
            <Input
              id="profile-name"
              value={profileName}
              onChange={(e) => setProfileName(e.target.value)}
              placeholder="您的名稱"
            />
          </div>

          <div>
            <Label htmlFor="profile-email" className="mb-2 block">
              Email <span className="text-muted-foreground text-xs">(選填)</span>
            </Label>
            <Input
              id="profile-email"
              type="text"
              value={profileEmail}
              onChange={(e) => setProfileEmail(e.target.value)}
              placeholder="your@email.com"
            />
            <p className="text-xs text-muted-foreground mt-1">用於通知和報告寄送</p>
          </div>

          <div>
            <Label htmlFor="profile-title" className="mb-2 block">職稱</Label>
            <Input
              id="profile-title"
              value={profileTitle}
              onChange={(e) => setProfileTitle(e.target.value)}
              placeholder="例如：軟體工程師"
            />
          </div>

          <div>
            <Label htmlFor="profile-employee-id" className="mb-2 block">
              員工編號 <span className="text-muted-foreground text-xs">(選填)</span>
            </Label>
            <Input
              id="profile-employee-id"
              value={profileEmployeeId}
              onChange={(e) => setProfileEmployeeId(e.target.value)}
              placeholder="例如：EMP001"
            />
          </div>

          <div>
            <Label htmlFor="profile-department" className="mb-2 block">
              部門 <span className="text-muted-foreground text-xs">(選填)</span>
            </Label>
            <Input
              id="profile-department"
              value={profileDepartment}
              onChange={(e) => setProfileDepartment(e.target.value)}
              placeholder="例如：研發部"
            />
          </div>

          <div className="pt-4 border-t border-border">
            <Button onClick={() => onSave(setMessage)} disabled={saving}>
              {saving ? (
                <Loader2 className="w-4 h-4 animate-spin" strokeWidth={1.5} />
              ) : (
                <Save className="w-4 h-4" strokeWidth={1.5} />
              )}
              {saving ? '儲存中...' : '儲存'}
            </Button>
          </div>
        </div>
      </Card>
    </section>
  )
}
