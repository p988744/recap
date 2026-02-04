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
 * Test LLM connection
 */
export async function testLlmConnection(): Promise<LlmTestResult> {
  return invokeAuth<LlmTestResult>('test_llm_connection')
}

/**
 * Detected LLM API key from environment variable
 */
export interface DetectedLlmApiKey {
  provider: string
  env_var: string
  masked_key: string
}

/**
 * Response for detected LLM API keys
 */
export interface DetectedLlmApiKeysResponse {
  keys: DetectedLlmApiKey[]
}

/**
 * Detect LLM API keys from environment variables
 * No auth required - just checks local environment
 */
export async function detectLlmApiKeys(): Promise<DetectedLlmApiKeysResponse> {
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke<DetectedLlmApiKeysResponse>('detect_llm_api_keys')
}

/**
 * Get the actual API key from an environment variable
 */
export async function getEnvApiKey(envVar: string): Promise<string | null> {
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke<string | null>('get_env_api_key', { envVar })
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
