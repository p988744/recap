import { useEffect, useState } from 'react'
import { auth } from '@/services'
import { useAuth } from '@/lib/auth'
import type { SettingsMessage } from './types'

export function useProfileForm(user: ReturnType<typeof useAuth>['user']) {
  const [profileName, setProfileName] = useState('')
  const [profileEmail, setProfileEmail] = useState('')
  const [profileTitle, setProfileTitle] = useState('')
  const [profileEmployeeId, setProfileEmployeeId] = useState('')
  const [profileDepartment, setProfileDepartment] = useState('')
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    if (user) {
      setProfileName(user.name || '')
      setProfileEmail(user.email || '')
      setProfileTitle(user.title || '')
      setProfileEmployeeId(user.employee_id || '')
      setProfileDepartment(user.department_id || '')
    }
  }, [user])

  const handleSave = async (setMessage: (msg: SettingsMessage | null) => void) => {
    setSaving(true)
    setMessage(null)
    try {
      await auth.updateProfile({
        name: profileName,
        email: profileEmail || undefined,
        title: profileTitle,
        employee_id: profileEmployeeId || undefined,
        department_id: profileDepartment || undefined,
      })
      setMessage({ type: 'success', text: '個人資料已更新' })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '更新失敗' })
    } finally {
      setSaving(false)
    }
  }

  return {
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
    handleSave,
  }
}
