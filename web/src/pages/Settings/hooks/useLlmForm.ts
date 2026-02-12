import { useCallback, useEffect, useState } from 'react'
import { config as configService } from '@/services'
import type { ConfigResponse, LlmPreset } from '@/types'
import type { SettingsMessage } from './types'

export function useLlmForm(config: ConfigResponse | null) {
  const [llmProvider, setLlmProvider] = useState('openai')
  const [llmModel, setLlmModel] = useState('gpt-5-nano')
  const [llmApiKey, setLlmApiKey] = useState('')
  const [llmBaseUrl, setLlmBaseUrl] = useState('')
  const [showLlmKey, setShowLlmKey] = useState(false)
  const [saving, setSaving] = useState(false)
  const [presets, setPresets] = useState<LlmPreset[]>([])

  useEffect(() => {
    if (config) {
      setLlmProvider(config.llm_provider || 'openai')
      setLlmModel(config.llm_model || 'gpt-5-nano')
      setLlmBaseUrl(config.llm_base_url || '')
    }
  }, [config])

  const loadPresets = useCallback(async () => {
    try {
      const list = await configService.listLlmPresets()
      setPresets(list)
    } catch {
      // Ignore — presets are optional
    }
  }, [])

  useEffect(() => {
    if (config) loadPresets()
  }, [config, loadPresets])

  const handleProviderChange = (providerId: string) => {
    if (providerId === llmProvider) return
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
      await loadPresets()
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '儲存失敗' })
    } finally {
      setSaving(false)
    }
  }

  const handleSavePreset = async (
    name: string,
    setMessage: (msg: SettingsMessage | null) => void,
  ) => {
    try {
      await configService.saveLlmPreset(name)
      await loadPresets()
      setMessage({ type: 'success', text: `已儲存預設「${name}」` })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '儲存預設失敗' })
    }
  }

  const handleDeletePreset = async (presetId: string) => {
    try {
      await configService.deleteLlmPreset(presetId)
      await loadPresets()
    } catch {
      // Ignore
    }
  }

  const handleApplyPreset = async (
    presetId: string,
    setMessage: (msg: SettingsMessage | null) => void,
    refreshConfig: () => Promise<ConfigResponse>,
  ) => {
    try {
      const updated = await configService.applyLlmPreset(presetId)
      // Refresh config state via parent
      await refreshConfig()
      // Update local form state from the returned config
      setLlmProvider(updated.llm_provider || 'openai')
      setLlmModel(updated.llm_model || 'gpt-5-nano')
      setLlmBaseUrl(updated.llm_base_url || '')
      setLlmApiKey('')
      await loadPresets()
      setMessage({ type: 'success', text: '已套用預設設定' })
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : '套用預設失敗' })
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
    presets,
    handleSavePreset,
    handleDeletePreset,
    handleApplyPreset,
  }
}
