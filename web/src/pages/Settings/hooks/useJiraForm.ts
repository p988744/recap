import { useEffect, useState } from 'react'
import { config as configService, tempo } from '@/services'
import type { ConfigResponse } from '@/types'
import type { SettingsMessage } from './types'

export function useJiraForm(config: ConfigResponse | null) {
  const [jiraUrl, setJiraUrl] = useState('')
  const [jiraAuthType, setJiraAuthType] = useState<'pat' | 'basic'>('pat')
  const [jiraToken, setJiraToken] = useState('')
  const [jiraEmail, setJiraEmail] = useState('')
  const [tempoToken, setTempoToken] = useState('')
  const [showToken, setShowToken] = useState(false)
  const [saving, setSaving] = useState(false)
  const [testing, setTesting] = useState(false)

  useEffect(() => {
    if (config) {
      setJiraUrl(config.jira_url || '')
      setJiraAuthType(config.auth_type === 'basic' ? 'basic' : 'pat')
    }
  }, [config])

  const handleSave = async (
    setMessage: (msg: SettingsMessage | null) => void,
    refreshConfig: () => Promise<ConfigResponse>
  ) => {
    setSaving(true)
    setMessage(null)
    try {
      const payload: {
        jira_url?: string
        jira_pat?: string
        jira_email?: string
        jira_api_token?: string
        auth_type?: string
        tempo_api_token?: string
      } = {
        jira_url: jiraUrl,
        auth_type: jiraAuthType,
      }

      if (jiraToken) {
        if (jiraAuthType === 'pat') {
          payload.jira_pat = jiraToken
        } else {
          payload.jira_api_token = jiraToken
        }
      }

      if (jiraAuthType === 'basic' && jiraEmail) {
        payload.jira_email = jiraEmail
      }

      if (tempoToken) {
        payload.tempo_api_token = tempoToken
      }

      await configService.updateJiraConfig(payload)
      await refreshConfig()
      setJiraToken('')
      setTempoToken('')
      setMessage({ type: 'success', text: 'Jira 設定已儲存' })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '儲存失敗' })
    } finally {
      setSaving(false)
    }
  }

  const handleTest = async (setMessage: (msg: SettingsMessage | null) => void) => {
    setTesting(true)
    setMessage(null)
    try {
      const result = await tempo.testConnection()
      setMessage({ type: 'success', text: result.message })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '連線失敗' })
    } finally {
      setTesting(false)
    }
  }

  return {
    jiraUrl,
    setJiraUrl,
    jiraAuthType,
    setJiraAuthType,
    jiraToken,
    setJiraToken,
    jiraEmail,
    setJiraEmail,
    tempoToken,
    setTempoToken,
    showToken,
    setShowToken,
    saving,
    testing,
    handleSave,
    handleTest,
  }
}
