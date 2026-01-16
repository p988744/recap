import { useEffect, useState } from 'react'
import { config as configService } from '@/services'
import type { ConfigResponse } from '@/types'
import type { SettingsMessage } from './types'

export function usePreferencesForm(config: ConfigResponse | null) {
  const [dailyHours, setDailyHours] = useState(8)
  const [normalizeHours, setNormalizeHours] = useState(true)
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    if (config) {
      setDailyHours(config.daily_work_hours)
      setNormalizeHours(config.normalize_hours)
    }
  }, [config])

  const handleSave = async (setMessage: (msg: SettingsMessage | null) => void) => {
    setSaving(true)
    setMessage(null)
    try {
      await configService.updateConfig({
        daily_work_hours: dailyHours,
        normalize_hours: normalizeHours,
      })
      setMessage({ type: 'success', text: '偏好設定已儲存' })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '儲存失敗' })
    } finally {
      setSaving(false)
    }
  }

  return {
    dailyHours,
    setDailyHours,
    normalizeHours,
    setNormalizeHours,
    saving,
    handleSave,
  }
}
