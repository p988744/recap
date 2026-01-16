import { describe, it, expect, beforeEach } from 'vitest'
import {
  mockInvoke,
  mockCommandValue,
  mockCommandError,
  resetTauriMock,
} from '@/test/mocks/tauri'
import * as config from './config'

// Mock fixtures
const mockConfigResponse = {
  daily_work_hours: 8,
  normalize_hours: true,
  jira_url: 'https://jira.example.com',
  auth_type: 'pat',
  llm_provider: 'openai',
  llm_model: 'gpt-4o-mini',
  llm_base_url: '',
  gitlab_url: 'https://gitlab.example.com',
}

const mockMessageResponse = {
  message: 'Configuration updated successfully',
}

describe('config service', () => {
  beforeEach(() => {
    resetTauriMock()
    localStorage.setItem('recap_auth_token', 'test-token')
  })

  describe('getConfig', () => {
    it('should return current configuration', async () => {
      mockCommandValue('get_config', mockConfigResponse)

      const result = await config.getConfig()

      expect(result).toEqual(mockConfigResponse)
      expect(mockInvoke).toHaveBeenCalledWith('get_config', { token: 'test-token' })
    })

    it('should throw on error', async () => {
      mockCommandError('get_config', 'Failed to load config')

      await expect(config.getConfig()).rejects.toThrow('Failed to load config')
    })
  })

  describe('updateConfig', () => {
    it('should update general config settings', async () => {
      mockCommandValue('update_config', mockMessageResponse)

      const request = {
        daily_work_hours: 7,
        normalize_hours: false,
      }
      const result = await config.updateConfig(request)

      expect(result.message).toBe('Configuration updated successfully')
      expect(mockInvoke).toHaveBeenCalledWith('update_config', {
        token: 'test-token',
        request,
      })
    })

    it('should throw on invalid settings', async () => {
      mockCommandError('update_config', 'Invalid daily hours')

      const request = { daily_work_hours: -1 }

      await expect(config.updateConfig(request)).rejects.toThrow('Invalid daily hours')
    })
  })

  describe('updateLlmConfig', () => {
    it('should update LLM configuration', async () => {
      mockCommandValue('update_llm_config', mockMessageResponse)

      const request = {
        provider: 'openai',
        model: 'gpt-4o',
        api_key: 'sk-xxx',
      }
      const result = await config.updateLlmConfig(request)

      expect(result.message).toBe('Configuration updated successfully')
      expect(mockInvoke).toHaveBeenCalledWith('update_llm_config', {
        token: 'test-token',
        request,
      })
    })

    it('should throw on invalid API key', async () => {
      mockCommandError('update_llm_config', 'Invalid API key format')

      const request = { api_key: 'invalid' }

      await expect(config.updateLlmConfig(request)).rejects.toThrow('Invalid API key format')
    })
  })

  describe('updateJiraConfig', () => {
    it('should update Jira configuration', async () => {
      mockCommandValue('update_jira_config', mockMessageResponse)

      const request = {
        url: 'https://jira.example.com',
        email: 'user@example.com',
        token: 'jira-token',
        auth_type: 'basic' as const,
      }
      const result = await config.updateJiraConfig(request)

      expect(result.message).toBe('Configuration updated successfully')
      expect(mockInvoke).toHaveBeenCalledWith('update_jira_config', {
        token: 'test-token',
        request,
      })
    })

    it('should update Jira config with PAT auth', async () => {
      mockCommandValue('update_jira_config', mockMessageResponse)

      const request = {
        url: 'https://jira.example.com',
        token: 'pat-token',
        auth_type: 'pat' as const,
      }
      const result = await config.updateJiraConfig(request)

      expect(result.message).toBe('Configuration updated successfully')
    })

    it('should throw on invalid Jira URL', async () => {
      mockCommandError('update_jira_config', 'Invalid Jira URL')

      const request = { url: 'not-a-url' }

      await expect(config.updateJiraConfig(request)).rejects.toThrow('Invalid Jira URL')
    })
  })
})
