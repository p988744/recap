import { useEffect, useState } from 'react'
import { config as configService } from '@/services'
import type { ConfigResponse } from '@/types'
import type { SettingsMessage } from './types'

export function useLlmForm(config: ConfigResponse | null) {
  const [llmProvider, setLlmProvider] = useState('openai')
  const [llmModel, setLlmModel] = useState('gpt-4o-mini')
  const [llmApiKey, setLlmApiKey] = useState('')
  const [llmBaseUrl, setLlmBaseUrl] = useState('')
  const [showLlmKey, setShowLlmKey] = useState(false)
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    if (config) {
      setLlmProvider(config.llm_provider || 'openai')
      setLlmModel(config.llm_model || 'gpt-4o-mini')
      setLlmBaseUrl(config.llm_base_url || '')
    }
  }, [config])

  const handleProviderChange = (providerId: string) => {
    setLlmProvider(providerId)
    if (providerId === 'openai') setLlmModel('gpt-4o-mini')
    else if (providerId === 'anthropic') setLlmModel('claude-3-5-sonnet-20241022')
    else if (providerId === 'ollama') setLlmModel('llama3.2')
    else setLlmModel('')
    if (providerId === 'ollama') setLlmBaseUrl('http://localhost:11434')
    else if (providerId !== 'openai-compatible') setLlmBaseUrl('')
  }

  const handleSave = async (
    setMessage: (msg: SettingsMessage | null) => void,
    refreshConfig: () => Promise<ConfigResponse>
  ) => {
    setSaving(true)
    setMessage(null)
    try {
      await configService.updateLlmConfig({
        provider: llmProvider,
        model: llmModel,
        api_key: llmApiKey || undefined,
        base_url: llmBaseUrl || undefined,
      })
      await refreshConfig()
      setLlmApiKey('')
      setMessage({ type: 'success', text: 'LLM 設定已儲存' })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '儲存失敗' })
    } finally {
      setSaving(false)
    }
  }

  return {
    llmProvider,
    llmModel,
    setLlmModel,
    llmApiKey,
    setLlmApiKey,
    llmBaseUrl,
    setLlmBaseUrl,
    showLlmKey,
    setShowLlmKey,
    saving,
    handleProviderChange,
    handleSave,
  }
}
