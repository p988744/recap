/**
 * Quota tools configuration
 *
 * Defines the available tools for quota tracking with their metadata.
 */

import type { ComponentType } from 'react'
import { ClaudeIcon, OpenAIIcon, GeminiIcon } from '@/components/icons'

export interface QuotaTool {
  id: string
  name: string
  icon: ComponentType<{ className?: string }>
  description: string
  hasQuota: boolean
  hasCost: boolean
  disabled?: boolean
}

export const QUOTA_TOOLS: Record<string, QuotaTool> = {
  claude: {
    id: 'claude',
    name: 'Claude Code',
    icon: ClaudeIcon,
    description: 'Anthropic Claude Code',
    hasQuota: true,
    hasCost: true,
  },
  codex: {
    id: 'codex',
    name: 'Codex',
    icon: OpenAIIcon,
    description: 'OpenAI Codex',
    hasQuota: true,
    hasCost: true,
    disabled: true, // v2.3.0
  },
  antigravity: {
    id: 'antigravity',
    name: 'Antigravity',
    icon: GeminiIcon,
    description: 'Google Antigravity',
    hasQuota: true,
    hasCost: false,
    disabled: true, // v2.3.0
  },
}

export const TOOL_IDS = Object.keys(QUOTA_TOOLS) as (keyof typeof QUOTA_TOOLS)[]

export function getToolById(toolId: string): QuotaTool | undefined {
  return QUOTA_TOOLS[toolId]
}
