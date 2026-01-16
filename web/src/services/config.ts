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
