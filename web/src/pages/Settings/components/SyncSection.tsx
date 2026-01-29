import { useMemo } from 'react'
import { Save, Loader2, Clock } from 'lucide-react'
import { Card } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import type { SettingsMessage } from '../hooks/useSettings'

// =============================================================================
// Constants
// =============================================================================

const TIMEZONE_OPTIONS = [
  'Asia/Taipei',
  'Asia/Tokyo',
  'Asia/Shanghai',
  'Asia/Hong_Kong',
  'Asia/Singapore',
  'Asia/Seoul',
  'America/New_York',
  'America/Chicago',
  'America/Denver',
  'America/Los_Angeles',
  'Europe/London',
  'Europe/Berlin',
  'Europe/Paris',
  'Australia/Sydney',
  'Pacific/Auckland',
  'UTC',
]

const WEEK_DAYS = [
  { value: 0, label: '日' },
  { value: 1, label: '一' },
  { value: 2, label: '二' },
  { value: 3, label: '三' },
  { value: 4, label: '四' },
  { value: 5, label: '五' },
  { value: 6, label: '六' },
]

// =============================================================================
// Types
// =============================================================================

interface SyncSectionProps {
  // Sync Config
  enabled: boolean
  setEnabled: (v: boolean) => void
  intervalMinutes: number
  setIntervalMinutes: (v: number) => void
  // Sync UI
  loading: boolean
  saving: boolean
  // Sync Actions
  onSave: (setMessage: (msg: SettingsMessage | null) => void) => Promise<void>
  // Preferences
  dailyHours: number
  setDailyHours: (v: number) => void
  normalizeHours: boolean
  setNormalizeHours: (v: boolean) => void
  timezone: string | null
  setTimezone: (v: string | null) => void
  weekStartDay: number
  setWeekStartDay: (v: number) => void
  savingPreferences: boolean
  onSavePreferences: (setMessage: (msg: SettingsMessage | null) => void) => Promise<void>
  // Shared
  setMessage: (msg: SettingsMessage | null) => void
}

// =============================================================================
// Components
// =============================================================================

function Toggle({
  checked,
  onChange,
  label,
  description,
  disabled,
}: {
  checked: boolean
  onChange: (v: boolean) => void
  label: string
  description?: string
  disabled?: boolean
}) {
  return (
    <label className={`flex items-center gap-3 ${disabled ? 'opacity-50' : 'cursor-pointer'}`}>
      <div className="relative">
        <input
          type="checkbox"
          checked={checked}
          onChange={(e) => onChange(e.target.checked)}
          disabled={disabled}
          className="sr-only peer"
        />
        <div className="w-10 h-5 bg-foreground/15 peer-checked:bg-foreground transition-colors" />
        <div className="absolute top-0.5 left-0.5 w-4 h-4 bg-white transition-transform peer-checked:translate-x-5" />
      </div>
      <div>
        <span className="text-sm text-foreground">{label}</span>
        {description && <p className="text-xs text-muted-foreground">{description}</p>}
      </div>
    </label>
  )
}

// =============================================================================
// Main Component
// =============================================================================

export function SyncSection({
  enabled,
  setEnabled,
  intervalMinutes,
  setIntervalMinutes,
  loading,
  saving,
  onSave,
  dailyHours,
  setDailyHours,
  normalizeHours,
  setNormalizeHours,
  timezone,
  setTimezone,
  weekStartDay,
  setWeekStartDay,
  savingPreferences,
  onSavePreferences,
  setMessage,
}: SyncSectionProps) {
  const systemTimezone = useMemo(() => {
    try {
      return Intl.DateTimeFormat().resolvedOptions().timeZone
    } catch {
      return 'UTC'
    }
  }, [])

  if (loading) {
    return (
      <section className="animate-fade-up opacity-0 delay-1">
        <h2 className="font-display text-2xl text-foreground mb-6">系統設定</h2>
        <div className="flex items-center justify-center h-48">
          <Loader2 className="w-6 h-6 animate-spin text-muted-foreground" />
        </div>
      </section>
    )
  }

  return (
    <section className="animate-fade-up opacity-0 delay-1">
      <h2 className="font-display text-2xl text-foreground mb-6">系統設定</h2>

      {/* Sync Configuration Card */}
      <Card className="p-6 mb-6">
        <h3 className="font-medium mb-4">背景同步</h3>

        <div className="space-y-6">
          {/* Enable Toggle */}
          <Toggle
            checked={enabled}
            onChange={setEnabled}
            label="啟用背景同步"
            description="自動定時同步工作項目"
          />

          {/* Interval Selection */}
          <div className={enabled ? '' : 'opacity-50 pointer-events-none'}>
            <Label className="mb-2 block">同步間隔</Label>
            <div className="flex items-center gap-2">
              <Clock className="w-4 h-4 text-muted-foreground" />
              <select
                value={intervalMinutes}
                onChange={(e) => setIntervalMinutes(Number(e.target.value))}
                className="px-3 py-2 bg-background border border-border text-sm focus:outline-none focus:ring-1 focus:ring-foreground"
                disabled={!enabled}
              >
                <option value={5}>每 5 分鐘</option>
                <option value={15}>每 15 分鐘</option>
                <option value={30}>每 30 分鐘</option>
                <option value={60}>每小時</option>
              </select>
            </div>
          </div>

          {/* Save Button */}
          <div className="pt-4 border-t border-border">
            <Button onClick={() => onSave(setMessage)} disabled={saving}>
              {saving ? (
                <Loader2 className="w-4 h-4 animate-spin" />
              ) : (
                <Save className="w-4 h-4" />
              )}
              {saving ? '儲存中...' : '儲存同步設定'}
            </Button>
          </div>
        </div>
      </Card>

      {/* Preferences Card */}
      <Card className="p-6">
        <h3 className="font-medium mb-4">工時與時區</h3>

        <div className="space-y-6">
          <div>
            <Label htmlFor="daily-hours" className="mb-2 block">每日標準工時</Label>
            <div className="flex items-center gap-3">
              <Input
                id="daily-hours"
                type="number"
                value={dailyHours}
                onChange={(e) => setDailyHours(Number(e.target.value))}
                min={1}
                max={24}
                step={0.5}
                className="w-24"
              />
              <span className="text-sm text-muted-foreground">小時</span>
            </div>
          </div>

          <Toggle
            checked={normalizeHours}
            onChange={setNormalizeHours}
            label="自動正規化工時"
            description="將每日工時調整為標準工時"
          />

          <div>
            <Label htmlFor="timezone" className="mb-2 block">時區</Label>
            <select
              id="timezone"
              value={timezone ?? ''}
              onChange={(e) => setTimezone(e.target.value || null)}
              className="flex h-9 w-full max-w-xs rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
            >
              <option value="">{systemTimezone} (系統預設)</option>
              {TIMEZONE_OPTIONS.map((tz) => (
                <option key={tz} value={tz}>{tz}</option>
              ))}
            </select>
            <p className="text-xs text-muted-foreground mt-1">
              留空則使用系統偵測的時區
            </p>
          </div>

          <div>
            <Label htmlFor="week-start-day" className="mb-2 block">每週起始日</Label>
            <select
              id="week-start-day"
              value={weekStartDay}
              onChange={(e) => setWeekStartDay(Number(e.target.value))}
              className="flex h-9 w-full max-w-xs rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
            >
              {WEEK_DAYS.map((day) => (
                <option key={day.value} value={day.value}>{day.label}</option>
              ))}
            </select>
          </div>

          <div className="pt-4 border-t border-border">
            <Button onClick={() => onSavePreferences(setMessage)} disabled={savingPreferences}>
              {savingPreferences ? <Loader2 className="w-4 h-4 animate-spin" /> : <Save className="w-4 h-4" />}
              {savingPreferences ? '儲存中...' : '儲存偏好設定'}
            </Button>
          </div>
        </div>
      </Card>
    </section>
  )
}
