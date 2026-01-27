/**
 * LLM Usage types
 */

export interface LlmUsageStats {
  total_calls: number
  success_calls: number
  error_calls: number
  total_prompt_tokens: number
  total_completion_tokens: number
  total_tokens: number
  total_cost: number
  avg_duration_ms: number
  avg_tokens_per_call: number
}

export interface DailyUsage {
  date: string
  calls: number
  prompt_tokens: number
  completion_tokens: number
  total_tokens: number
  cost: number
}

export interface ModelUsage {
  provider: string
  model: string
  calls: number
  total_tokens: number
  cost: number
}

export interface LlmUsageLog {
  id: string
  provider: string
  model: string
  prompt_tokens: number | null
  completion_tokens: number | null
  total_tokens: number | null
  estimated_cost: number | null
  purpose: string
  duration_ms: number | null
  status: string
  error_message: string | null
  created_at: string
}
