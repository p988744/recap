import { useState, useCallback } from 'react'
import {
  Save,
  Loader2,
  Sparkles,
  CheckCircle2,
  XCircle,
  Eye,
  EyeOff,
  RefreshCw,
  Zap,
  Key,
} from 'lucide-react'
import type { DetectedLlmApiKey } from '@/services/config'
import { Card } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import type { ConfigResponse } from '@/types'
import type { SettingsMessage } from '../hooks/useSettings'
import { useLlmUsage } from '@/pages/LlmUsage/hooks/useLlmUsage'
import { UsageSummary } from '@/pages/LlmUsage/components/UsageSummary'
import { DailyChart } from '@/pages/LlmUsage/components/DailyChart'
import { UsageLogs } from '@/pages/LlmUsage/components/UsageLogs'
import { config as configService } from '@/services'

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
  // Detected API keys
  detectedKeys: DetectedLlmApiKey[]
  detectingKeys: boolean
  onUseDetectedKey: (key: DetectedLlmApiKey) => Promise<void>
}

const LLM_PROVIDERS = [
  { id: 'openai', label: 'OpenAI', desc: 'GPT-5 系列' },
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
  detectedKeys,
  detectingKeys,
  onUseDetectedKey,
}: AiSectionProps) {
  const [rangeDays, setRangeDays] = useState(30)
  const [testing, setTesting] = useState(false)
  const [testResult, setTestResult] = useState<{ success: boolean; message: string; latency?: number } | null>(null)
  const usageRange = getUsageDateRange(rangeDays)
  const { stats, daily, logs, loading: usageLoading, refresh } = useLlmUsage(usageRange.start, usageRange.end)

  const handleTestConnection = useCallback(async () => {
    setTesting(true)
    setTestResult(null)
    try {
      const result = await configService.testLlmConnection()
      setTestResult({
        success: result.success,
        message: result.message,
        latency: result.latency_ms,
      })
    } catch (err) {
      setTestResult({
        success: false,
        message: err instanceof Error ? err.message : '連線測試失敗',
      })
    } finally {
      setTesting(false)
    }
  }, [])

  return (
    <section className="animate-fade-up opacity-0 delay-1 space-y-8">
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
                llmProvider === 'openai' ? 'gpt-5-nano' :
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

              {/* Detected API Keys from environment */}
              {detectingKeys ? (
                <div className="mt-2 flex items-center gap-2 text-xs text-muted-foreground">
                  <Loader2 className="w-3 h-3 animate-spin" />
                  偵測環境變數中...
                </div>
              ) : detectedKeys.length > 0 && (
                <div className="mt-3 p-3 bg-amber-500/5 border border-amber-500/20 rounded-lg">
                  <div className="flex items-center gap-2 text-xs text-amber-600 mb-2">
                    <Key className="w-3.5 h-3.5" />
                    從環境變數偵測到 API Key
                  </div>
                  <div className="space-y-1.5">
                    {detectedKeys.map((key) => (
                      <button
                        key={key.env_var}
                        onClick={() => onUseDetectedKey(key)}
                        className="w-full flex items-center justify-between p-2 text-xs bg-background border border-border rounded hover:border-foreground/30 transition-colors"
                      >
                        <div className="flex items-center gap-2">
                          <code className="px-1.5 py-0.5 bg-foreground/5 rounded text-[10px]">
                            {key.env_var}
                          </code>
                          <span className="text-muted-foreground">
                            ({key.provider})
                          </span>
                        </div>
                        <span className="font-mono text-muted-foreground">
                          {key.masked_key}
                        </span>
                      </button>
                    ))}
                  </div>
                  <p className="mt-2 text-[10px] text-muted-foreground">
                    點擊使用偵測到的 API Key
                  </p>
                </div>
              )}
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

          <div className="pt-4 border-t border-border space-y-3">
            <div className="flex items-center gap-2">
              <Button onClick={() => onSaveLlm(setMessage, refreshConfig)} disabled={savingLlm}>
                {savingLlm ? <Loader2 className="w-4 h-4 animate-spin" /> : <Save className="w-4 h-4" />}
                儲存 LLM 設定
              </Button>
              <Button
                variant="outline"
                onClick={handleTestConnection}
                disabled={testing || !config?.llm_configured}
              >
                {testing ? <Loader2 className="w-4 h-4 animate-spin" /> : <Zap className="w-4 h-4" />}
                測試連線
              </Button>
            </div>

            {testResult && (
              <div className={`p-3 text-sm rounded-lg flex items-start gap-2 ${
                testResult.success
                  ? 'bg-sage/10 text-sage border border-sage/20'
                  : 'bg-destructive/10 text-destructive border border-destructive/20'
              }`}>
                {testResult.success ? (
                  <CheckCircle2 className="w-4 h-4 mt-0.5 shrink-0" />
                ) : (
                  <XCircle className="w-4 h-4 mt-0.5 shrink-0" />
                )}
                <div>
                  <p>{testResult.message}</p>
                  {testResult.success && testResult.latency && (
                    <p className="text-xs opacity-70 mt-1">延遲: {testResult.latency}ms</p>
                  )}
                </div>
              </div>
            )}
          </div>
        </div>
      </Card>

      {/* LLM Usage */}
      <div className="space-y-6">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="font-medium text-foreground">用量統計</h3>
            <p className="text-xs text-muted-foreground mt-1">
              追蹤 API 呼叫次數、Token 用量與預估費用
            </p>
          </div>
          <div className="flex items-center gap-2">
            {RANGE_OPTIONS.map((opt) => (
              <Button
                key={opt.days}
                variant={rangeDays === opt.days ? 'default' : 'outline'}
                size="sm"
                onClick={() => setRangeDays(opt.days)}
                className="text-xs"
              >
                {opt.label}
              </Button>
            ))}
            <Button
              variant="ghost"
              size="icon"
              onClick={refresh}
              disabled={usageLoading}
              className="h-8 w-8"
            >
              <RefreshCw className={`w-3.5 h-3.5 ${usageLoading ? 'animate-spin' : ''}`} strokeWidth={1.5} />
            </Button>
          </div>
        </div>

        <UsageSummary stats={stats} />

        <DailyChart data={daily} />

        <div className="h-px bg-charcoal/6" />

        <UsageLogs logs={logs} />
      </div>
    </section>
  )
}

const RANGE_OPTIONS = [
  { label: '7 天', days: 7 },
  { label: '30 天', days: 30 },
  { label: '90 天', days: 90 },
]

function getUsageDateRange(days: number): { start: string; end: string } {
  const end = new Date()
  const start = new Date()
  start.setDate(end.getDate() - days + 1)
  return {
    start: start.toISOString().slice(0, 10),
    end: end.toISOString().slice(0, 10),
  }
}
