import { useCallback, useEffect, useState } from 'react'
import { config as configService } from '@/services'
import type { ConfigResponse } from '@/types'
import type { SettingsMessage } from './types'
import type { DetectedLlmApiKey } from '@/services/config'

export function useLlmForm(config: ConfigResponse | null) {
  const [llmProvider, setLlmProvider] = useState('openai')
  const [llmModel, setLlmModel] = useState('gpt-5-nano')
  const [llmApiKey, setLlmApiKey] = useState('')
  const [llmBaseUrl, setLlmBaseUrl] = useState('')
  const [showLlmKey, setShowLlmKey] = useState(false)
  const [saving, setSaving] = useState(false)

  // Detected API keys from environment
  const [detectedKeys, setDetectedKeys] = useState<DetectedLlmApiKey[]>([])
  const [detectingKeys, setDetectingKeys] = useState(false)

  useEffect(() => {
    if (config) {
      setLlmProvider(config.llm_provider || 'openai')
      setLlmModel(config.llm_model || 'gpt-5-nano')
      setLlmBaseUrl(config.llm_base_url || '')
    }
  }, [config])

  // Detect API keys from environment on mount
  useEffect(() => {
    const detectKeys = async () => {
      setDetectingKeys(true)
      try {
        const result = await configService.detectLlmApiKeys()
        setDetectedKeys(result.keys)
      } catch (err) {
        console.error('Failed to detect LLM API keys:', err)
      } finally {
        setDetectingKeys(false)
      }
    }
    detectKeys()
  }, [])

  // Use a detected API key
  const useDetectedKey = useCallback(async (
    key: DetectedLlmApiKey,
    setMessage: (msg: SettingsMessage | null) => void
  ) => {
    try {
      const actualKey = await configService.getEnvApiKey(key.env_var)
      if (actualKey) {
        setLlmApiKey(actualKey)
        // Auto-select provider based on detected key
        if (key.provider === 'openai') {
          setLlmProvider('openai')
          setLlmModel('gpt-5-nano')
        } else if (key.provider === 'anthropic') {
          setLlmProvider('anthropic')
          setLlmModel('claude-3-5-sonnet-20241022')
        } else if (key.provider === 'google') {
          setLlmProvider('openai-compatible')
          setLlmModel('gemini-2.0-flash')
          setLlmBaseUrl('https://generativelanguage.googleapis.com/v1beta/openai')
        }
        setMessage({ type: 'success', text: `已載入 ${key.env_var} 的 API Key` })
      }
    } catch (err) {
      setMessage({ type: 'error', text: '無法讀取環境變數' })
    }
  }, [])

  const handleProviderChange = (providerId: string) => {
    setLlmProvider(providerId)
    if (providerId === 'openai') setLlmModel('gpt-5-nano')
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
    // Detected API keys
    detectedKeys,
    detectingKeys,
    useDetectedKey,
  }
}
