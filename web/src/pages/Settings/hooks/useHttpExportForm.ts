import { useState, useEffect, useCallback } from 'react'
import { httpExport } from '@/services'
import type { HttpExportConfig, SaveHttpExportConfigRequest, ValidateTemplateResponse, TestConnectionResponse } from '@/types'

interface HttpExportFormState {
  configs: HttpExportConfig[]
  selectedId: string | null
  loading: boolean

  // Form fields
  name: string
  url: string
  method: string
  authType: string
  authToken: string
  authHeaderName: string
  customHeaders: string
  payloadTemplate: string
  llmPrompt: string
  batchMode: boolean
  batchWrapperKey: string
  timeoutSeconds: number

  // UI state
  showToken: boolean
  saving: boolean
  testing: boolean
  validating: boolean
  deleting: boolean
  validateResult: ValidateTemplateResponse | null
  testResult: TestConnectionResponse | null
  message: { type: 'success' | 'error'; text: string } | null
}

const DEFAULT_TEMPLATE = `{
  "date": "{{date}}",
  "content": "{{title}} ({{hours}}h) - {{description}}"
}`

const DEFAULT_LLM_PROMPT = `請將以下工作內容精簡為一句中文摘要（30字以內）：
專案：{{title}}
描述：{{description}}
時數：{{hours}}小時`

const SEED_CONFIG: SaveHttpExportConfigRequest = {
  name: '內部工作日誌',
  url: 'http://172.18.20.190:8001/api/worklog',
  method: 'POST',
  auth_type: 'bearer',
  auth_token: 'tAZnGkyqMCkMvxTQr3_umkQkZNwWBUnIQGgEkuFdoWo',
  payload_template: DEFAULT_TEMPLATE,
  llm_prompt: DEFAULT_LLM_PROMPT,
  batch_mode: false,
  timeout_seconds: 30,
}

export function useHttpExportForm() {
  const [state, setState] = useState<HttpExportFormState>({
    configs: [],
    selectedId: null,
    loading: true,
    name: '',
    url: '',
    method: 'POST',
    authType: 'none',
    authToken: '',
    authHeaderName: '',
    customHeaders: '',
    payloadTemplate: DEFAULT_TEMPLATE,
    llmPrompt: '',
    batchMode: false,
    batchWrapperKey: 'items',
    timeoutSeconds: 30,
    showToken: false,
    saving: false,
    testing: false,
    validating: false,
    deleting: false,
    validateResult: null,
    testResult: null,
    message: null,
  })

  const set = useCallback(
    (partial: Partial<HttpExportFormState>) =>
      setState((prev) => ({ ...prev, ...partial })),
    []
  )

  // Load configs (auto-seed default on first use)
  const loadConfigs = useCallback(async () => {
    try {
      let configs = await httpExport.listConfigs()
      if (configs.length === 0) {
        // Seed a default config for the internal worklog API
        await httpExport.saveConfig(SEED_CONFIG)
        configs = await httpExport.listConfigs()
      } else {
        // If seed config exists but was saved without token (e.g., due to HMR timing),
        // re-save with the token so the COALESCE upsert fills it in
        const seedMatch = configs.find((c) => c.name === SEED_CONFIG.name && c.url === SEED_CONFIG.url)
        if (seedMatch && SEED_CONFIG.auth_token) {
          await httpExport.saveConfig({ ...SEED_CONFIG, id: seedMatch.id })
          configs = await httpExport.listConfigs()
        }
      }
      set({ configs, loading: false })
    } catch {
      set({ loading: false })
    }
  }, [set])

  useEffect(() => {
    loadConfigs()
  }, [loadConfigs])

  // Select a config to edit
  const selectConfig = useCallback(
    (id: string | null) => {
      if (!id) {
        // New config
        set({
          selectedId: null,
          name: '',
          url: '',
          method: 'POST',
          authType: 'none',
          authToken: '',
          authHeaderName: '',
          customHeaders: '',
          payloadTemplate: DEFAULT_TEMPLATE,
          llmPrompt: '',
          batchMode: false,
          batchWrapperKey: 'items',
          timeoutSeconds: 30,
          validateResult: null,
          testResult: null,
          message: null,
        })
        return
      }

      const config = state.configs.find((c) => c.id === id)
      if (!config) return

      set({
        selectedId: id,
        name: config.name,
        url: config.url,
        method: config.method,
        authType: config.auth_type,
        authToken: '', // Never prefilled
        authHeaderName: config.auth_header_name ?? '',
        customHeaders: config.custom_headers ?? '',
        payloadTemplate: config.payload_template,
        llmPrompt: config.llm_prompt ?? '',
        batchMode: config.batch_mode,
        batchWrapperKey: config.batch_wrapper_key,
        timeoutSeconds: config.timeout_seconds,
        validateResult: null,
        testResult: null,
        message: null,
      })
    },
    [state.configs, set]
  )

  // Save config
  const saveConfig = useCallback(async () => {
    set({ saving: true, message: null })
    try {
      const request: SaveHttpExportConfigRequest = {
        id: state.selectedId ?? undefined,
        name: state.name,
        url: state.url,
        method: state.method,
        auth_type: state.authType,
        auth_token: state.authToken || undefined,
        auth_header_name: state.authHeaderName || undefined,
        custom_headers: state.customHeaders || undefined,
        payload_template: state.payloadTemplate,
        llm_prompt: state.llmPrompt || undefined,
        batch_mode: state.batchMode,
        batch_wrapper_key: state.batchWrapperKey,
        timeout_seconds: state.timeoutSeconds,
      }
      await httpExport.saveConfig(request)
      set({ saving: false, message: { type: 'success', text: 'Config saved' } })
      await loadConfigs()
    } catch (e) {
      set({ saving: false, message: { type: 'error', text: String(e) } })
    }
  }, [state, set, loadConfigs])

  // Delete config
  const deleteConfig = useCallback(async () => {
    if (!state.selectedId) return
    set({ deleting: true, message: null })
    try {
      await httpExport.deleteConfig(state.selectedId)
      set({ deleting: false, selectedId: null, message: null })
      selectConfig(null)
      await loadConfigs()
    } catch (e) {
      set({ deleting: false, message: { type: 'error', text: String(e) } })
    }
  }, [state.selectedId, set, selectConfig, loadConfigs])

  // Validate template
  const validateTemplate = useCallback(async () => {
    set({ validating: true, validateResult: null })
    try {
      const result = await httpExport.validateTemplate(state.payloadTemplate)
      set({ validating: false, validateResult: result })
    } catch (e) {
      set({
        validating: false,
        validateResult: { valid: false, fields_used: [], error: String(e) },
      })
    }
  }, [state.payloadTemplate, set])

  // Test connection
  const testConnectionFn = useCallback(async () => {
    if (!state.selectedId) {
      set({ message: { type: 'error', text: 'Please save the config first' } })
      return
    }
    set({ testing: true, testResult: null })
    try {
      const result = await httpExport.testConnection(state.selectedId)
      set({ testing: false, testResult: result })
    } catch (e) {
      set({
        testing: false,
        testResult: { success: false, message: String(e) },
      })
    }
  }, [state.selectedId, set])

  return {
    ...state,
    set,
    loadConfigs,
    selectConfig,
    saveConfig,
    deleteConfig,
    validateTemplate,
    testConnection: testConnectionFn,
  }
}
