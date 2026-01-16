import {
  Save,
  Loader2,
  Sparkles,
  CheckCircle2,
  XCircle,
  Eye,
  EyeOff,
} from 'lucide-react'
import { Card } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import type { ConfigResponse } from '@/types'
import type { SettingsMessage } from '../hooks/useSettings'

interface AiSectionProps {
  config: ConfigResponse | null
  llmProvider: string
  llmModel: string
  setLlmModel: (v: string) => void
  llmApiKey: string
  setLlmApiKey: (v: string) => void
  llmBaseUrl: string
  setLlmBaseUrl: (v: string) => void
  showLlmKey: boolean
  setShowLlmKey: (v: boolean) => void
  savingLlm: boolean
  onProviderChange: (providerId: string) => void
  onSaveLlm: (
    setMessage: (msg: SettingsMessage | null) => void,
    refreshConfig: () => Promise<ConfigResponse>
  ) => Promise<void>
  setMessage: (msg: SettingsMessage | null) => void
  refreshConfig: () => Promise<ConfigResponse>
}

const LLM_PROVIDERS = [
  { id: 'openai', label: 'OpenAI', desc: 'GPT-4o, GPT-4 等' },
  { id: 'anthropic', label: 'Anthropic', desc: 'Claude 系列' },
  { id: 'ollama', label: 'Ollama', desc: '本地部署' },
  { id: 'openai-compatible', label: '相容 API', desc: '自架 OpenAI 相容服務' },
]

export function AiSection({
  config,
  llmProvider,
  llmModel,
  setLlmModel,
  llmApiKey,
  setLlmApiKey,
  llmBaseUrl,
  setLlmBaseUrl,
  showLlmKey,
  setShowLlmKey,
  savingLlm,
  onProviderChange,
  onSaveLlm,
  setMessage,
  refreshConfig,
}: AiSectionProps) {
  return (
    <section className="animate-fade-up opacity-0 delay-1">
      <h2 className="font-display text-2xl text-foreground mb-6">AI 助手</h2>

      <Card className="p-6">
        <div className="flex items-center gap-3 mb-6">
          <div className="w-10 h-10 rounded-lg bg-amber-500/10 flex items-center justify-center">
            <Sparkles className="w-5 h-5 text-amber-600" strokeWidth={1.5} />
          </div>
          <div className="flex-1">
            <h3 className="font-medium text-foreground">LLM 設定</h3>
            <p className="text-xs text-muted-foreground">設定 AI 模型用於分析和建議</p>
          </div>
          {config?.llm_configured ? (
            <span className="flex items-center gap-1.5 text-xs text-sage">
              <CheckCircle2 className="w-3.5 h-3.5" strokeWidth={1.5} />
              已設定
            </span>
          ) : (
            <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
              <XCircle className="w-3.5 h-3.5" strokeWidth={1.5} />
              未設定
            </span>
          )}
        </div>

        <div className="space-y-4">
          {/* Provider Selection */}
          <div>
            <Label className="mb-2 block text-xs">提供者</Label>
            <div className="grid grid-cols-2 gap-2">
              {LLM_PROVIDERS.map((provider) => (
                <button
                  key={provider.id}
                  onClick={() => onProviderChange(provider.id)}
                  className={`p-3 text-left border rounded-lg transition-colors ${
                    llmProvider === provider.id
                      ? 'border-foreground bg-foreground/5'
                      : 'border-border hover:border-foreground/30'
                  }`}
                >
                  <p className="text-sm font-medium">{provider.label}</p>
                  <p className="text-xs text-muted-foreground">{provider.desc}</p>
                </button>
              ))}
            </div>
          </div>

          {/* Model */}
          <div>
            <Label htmlFor="llm-model" className="mb-2 block text-xs">模型名稱</Label>
            <Input
              id="llm-model"
              value={llmModel}
              onChange={(e) => setLlmModel(e.target.value)}
              placeholder={
                llmProvider === 'openai' ? 'gpt-4o-mini' :
                llmProvider === 'anthropic' ? 'claude-3-5-sonnet-20241022' :
                llmProvider === 'ollama' ? 'llama3.2' : '輸入模型名稱'
              }
            />
          </div>

          {/* API Key (not needed for Ollama) */}
          {llmProvider !== 'ollama' && (
            <div>
              <Label htmlFor="llm-api-key" className="mb-2 block text-xs">API Key</Label>
              <div className="relative">
                <Input
                  id="llm-api-key"
                  type={showLlmKey ? 'text' : 'password'}
                  value={llmApiKey}
                  onChange={(e) => setLlmApiKey(e.target.value)}
                  placeholder={config?.llm_configured ? '••••••••（已設定）' : '輸入 API Key'}
                  className="pr-10"
                />
                <button
                  type="button"
                  onClick={() => setShowLlmKey(!showLlmKey)}
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                >
                  {showLlmKey ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                </button>
              </div>
            </div>
          )}

          {/* Base URL (for Ollama and OpenAI-compatible) */}
          {(llmProvider === 'ollama' || llmProvider === 'openai-compatible') && (
            <div>
              <Label htmlFor="llm-base-url" className="mb-2 block text-xs">API URL</Label>
              <Input
                id="llm-base-url"
                type="url"
                value={llmBaseUrl}
                onChange={(e) => setLlmBaseUrl(e.target.value)}
                placeholder={llmProvider === 'ollama' ? 'http://localhost:11434' : 'https://your-api.example.com/v1'}
              />
              <p className="text-xs text-muted-foreground mt-1">
                {llmProvider === 'ollama' ? 'Ollama 服務地址' : 'OpenAI 相容的 API 端點'}
              </p>
            </div>
          )}

          <div className="pt-4 border-t border-border">
            <Button onClick={() => onSaveLlm(setMessage, refreshConfig)} disabled={savingLlm}>
              {savingLlm ? <Loader2 className="w-4 h-4 animate-spin" /> : <Save className="w-4 h-4" />}
              儲存 LLM 設定
            </Button>
          </div>
        </div>
      </Card>
    </section>
  )
}
