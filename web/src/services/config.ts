/**
 * Configuration service
 */

import { invokeAuth } from './client'
import type {
  ConfigResponse,
  UpdateConfigRequest,
  UpdateLlmConfigRequest,
  UpdateJiraConfigRequest,
  MessageResponse,
  LlmPreset,
} from '@/types'

/**
 * Get current user configuration
 */
export async function getConfig(): Promise<ConfigResponse> {
  return invokeAuth<ConfigResponse>('get_config')
}

/**
 * Update general config settings
 */
export async function updateConfig(request: UpdateConfigRequest): Promise<MessageResponse> {
  return invokeAuth<MessageResponse>('update_config', { request })
}

/**
 * Update LLM configuration
 */
export async function updateLlmConfig(request: UpdateLlmConfigRequest): Promise<MessageResponse> {
  return invokeAuth<MessageResponse>('update_llm_config', { request })
}

/**
 * Update Jira configuration
 */
export async function updateJiraConfig(request: UpdateJiraConfigRequest): Promise<MessageResponse> {
  return invokeAuth<MessageResponse>('update_jira_config', { request })
}

/**
 * LLM test result
 */
export interface LlmTestResult {
  success: boolean
  message: string
  latency_ms: number
  prompt_tokens: number | null
  completion_tokens: number | null
  model_response: string | null
}

/**
 * Test LLM connection with current form values
 */
export async function testLlmConnection(request?: {
  provider?: string
  model?: string
  api_key?: string
  base_url?: string
}): Promise<LlmTestResult> {
  return invokeAuth<LlmTestResult>('test_llm_connection', { request })
}

/**
 * Onboarding status response
 */
export interface OnboardingStatusResponse {
  completed: boolean
}

/**
 * Get onboarding status
 */
export async function getOnboardingStatus(): Promise<OnboardingStatusResponse> {
  return invokeAuth<OnboardingStatusResponse>('get_onboarding_status')
}

/**
 * Mark onboarding as completed
 */
export async function completeOnboarding(): Promise<MessageResponse> {
  return invokeAuth<MessageResponse>('complete_onboarding')
}

/**
 * List all LLM presets for the current user
 */
export async function listLlmPresets(): Promise<LlmPreset[]> {
  return invokeAuth<LlmPreset[]>('list_llm_presets')
}

/**
 * Save current LLM config as a named preset
 */
export async function saveLlmPreset(name: string): Promise<LlmPreset> {
  return invokeAuth<LlmPreset>('save_llm_preset', { name })
}

/**
 * Delete an LLM preset
 */
export async function deleteLlmPreset(presetId: string): Promise<void> {
  return invokeAuth<void>('delete_llm_preset', { presetId })
}

/**
 * Apply an LLM preset — copies config to active settings
 */
export async function applyLlmPreset(presetId: string): Promise<ConfigResponse> {
  return invokeAuth<ConfigResponse>('apply_llm_preset', { presetId })
}
