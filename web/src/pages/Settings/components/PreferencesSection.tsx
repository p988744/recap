import { Save, Loader2 } from 'lucide-react'
import { Card } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import type { SettingsMessage } from '../hooks/useSettings'

interface PreferencesSectionProps {
  dailyHours: number
  setDailyHours: (v: number) => void
  normalizeHours: boolean
  setNormalizeHours: (v: boolean) => void
  savingPreferences: boolean
  onSavePreferences: (setMessage: (msg: SettingsMessage | null) => void) => Promise<void>
  setMessage: (msg: SettingsMessage | null) => void
}

export function PreferencesSection({
  dailyHours,
  setDailyHours,
  normalizeHours,
  setNormalizeHours,
  savingPreferences,
  onSavePreferences,
  setMessage,
}: PreferencesSectionProps) {
  return (
    <section className="animate-fade-up opacity-0 delay-1">
      <h2 className="font-display text-2xl text-foreground mb-6">偏好設定</h2>

      <Card className="p-6">
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

          <div>
            <label className="flex items-center gap-3 cursor-pointer">
              <div className="relative">
                <input
                  type="checkbox"
                  checked={normalizeHours}
                  onChange={(e) => setNormalizeHours(e.target.checked)}
                  className="sr-only peer"
                />
                <div className="w-10 h-5 bg-foreground/15 peer-checked:bg-foreground transition-colors" />
                <div className="absolute top-0.5 left-0.5 w-4 h-4 bg-white transition-transform peer-checked:translate-x-5" />
              </div>
              <div>
                <span className="text-sm text-foreground">自動正規化工時</span>
                <p className="text-xs text-muted-foreground">將每日工時調整為標準工時</p>
              </div>
            </label>
          </div>

          <div className="pt-4 border-t border-border">
            <Button onClick={() => onSavePreferences(setMessage)} disabled={savingPreferences}>
              {savingPreferences ? <Loader2 className="w-4 h-4 animate-spin" /> : <Save className="w-4 h-4" />}
              {savingPreferences ? '儲存中...' : '儲存'}
            </Button>
          </div>
        </div>
      </Card>
    </section>
  )
}
