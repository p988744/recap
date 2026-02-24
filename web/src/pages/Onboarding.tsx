import { useState, useEffect, useCallback } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuth } from '@/lib/auth'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Card } from '@/components/ui/card'
import {
  Loader2,
  ArrowRight,
  ArrowLeft,
  User,
  Sparkles,
  FolderOpen,
  Check,
  CheckCircle2,
  XCircle,
  Eye,
  EyeOff,
} from 'lucide-react'
import { projects as projectsService, config as configService, auth as authService } from '@/services'
import { ClaudeIcon } from '@/pages/Settings/components/ProjectsSection/icons/ClaudeIcon'

const LLM_PROVIDERS = [
  { id: 'openai', label: 'OpenAI', desc: 'GPT-5 系列', defaultModel: 'gpt-5-nano' },
  { id: 'anthropic', label: 'Anthropic', desc: 'Claude 系列', defaultModel: 'claude-sonnet-4-20250514' },
  { id: 'ollama', label: 'Ollama', desc: '本地部署', defaultModel: 'llama3.2' },
  { id: 'openai-compatible', label: '相容 API', desc: '自架 OpenAI 相容服務', defaultModel: '' },
]

export function OnboardingPage() {
  const navigate = useNavigate()
  const { user, appStatus, register, onboardingCompleted, completeOnboarding } = useAuth()
  const [step, setStep] = useState(1)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')

  // Step 2: Profile
  const [name, setName] = useState('')
  const [email, setEmail] = useState('')
  const [title, setTitle] = useState('')

  // If user already exists, skip profile step and go to step 3 (only on initial load)
  const [initialSkipDone, setInitialSkipDone] = useState(false)
  useEffect(() => {
    if (!initialSkipDone && user && appStatus?.has_users) {
      // Pre-fill with existing user data
      setName(user.name || '')
      setEmail(user.email || '')
      setTitle(user.title || '')
      // Skip to step 3 (data sources)
      setStep(3)
      setInitialSkipDone(true)
    }
  }, [user, appStatus, initialSkipDone])

  // Step 3: Data Sources
  const [claudePath, setClaudePath] = useState('')
  const [claudePathLoading, setClaudePathLoading] = useState(true)

  // Step 4: LLM
  const [llmProvider, setLlmProvider] = useState('openai')
  const [llmModel, setLlmModel] = useState('gpt-5-nano')
  const [llmApiKey, setLlmApiKey] = useState('')
  const [llmBaseUrl, setLlmBaseUrl] = useState('')
  const [showLlmKey, setShowLlmKey] = useState(false)

  // If onboarding already completed, redirect to home
  useEffect(() => {
    if (onboardingCompleted) {
      navigate('/', { replace: true })
    }
  }, [onboardingCompleted, navigate])

  // Fetch Claude path
  const fetchClaudePath = useCallback(async () => {
    try {
      setClaudePathLoading(true)
      const data = await projectsService.getClaudeSessionPath()
      setClaudePath(data.path)
    } catch (err) {
      console.error('Failed to fetch Claude session path:', err)
    } finally {
      setClaudePathLoading(false)
    }
  }, [])

  // Load data sources info when reaching step 3
  useEffect(() => {
    if (step === 3) {
      fetchClaudePath()
    }
  }, [step, fetchClaudePath])

  const handleProviderChange = (providerId: string) => {
    setLlmProvider(providerId)
    const provider = LLM_PROVIDERS.find(p => p.id === providerId)
    if (provider) {
      setLlmModel(provider.defaultModel)
    }
    if (providerId === 'ollama') {
      setLlmBaseUrl('http://localhost:11434')
    } else if (providerId !== 'openai-compatible') {
      setLlmBaseUrl('')
    }
  }

  const handleProfileSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!name.trim()) {
      setError('請輸入您的名稱')
      return
    }

    setLoading(true)
    setError('')

    try {
      if (user) {
        // User exists, update profile
        await authService.updateProfile({
          name: name.trim(),
          email: email.trim() || undefined,
          title: title.trim() || undefined,
        })
      } else {
        // New user, register
        const userEmail = email.trim() || `${name.toLowerCase().replace(/\s+/g, '.')}@local`
        const password = 'local-mode-password'
        await register(userEmail, password, name.trim(), title.trim() || undefined)
      }
      setStep(3)
    } catch (err) {
      setError(err instanceof Error ? err.message : user ? '更新資料失敗' : '建立帳號失敗')
    } finally {
      setLoading(false)
    }
  }

  const handleLlmSubmit = async () => {
    // Validate LLM settings
    if (llmProvider !== 'ollama' && !llmApiKey.trim()) {
      setError('請輸入 API Key')
      return
    }
    if (!llmModel.trim()) {
      setError('請輸入模型名稱')
      return
    }
    if ((llmProvider === 'ollama' || llmProvider === 'openai-compatible') && !llmBaseUrl.trim()) {
      setError('請輸入 API URL')
      return
    }

    setLoading(true)
    setError('')

    try {
      await configService.updateLlmConfig({
        provider: llmProvider,
        model: llmModel,
        api_key: llmApiKey || undefined,
        base_url: llmBaseUrl || undefined,
      })
      setStep(5)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'LLM 設定失敗')
    } finally {
      setLoading(false)
    }
  }

  const handleComplete = async () => {
    try {
      // Mark onboarding as completed in database
      await completeOnboarding()
      navigate('/', { replace: true })
    } catch (err) {
      console.error('Failed to complete onboarding:', err)
      // Navigate anyway to avoid blocking user
      navigate('/', { replace: true })
    }
  }

  const progress = (step / 5) * 100

  return (
    <div className="min-h-screen bg-background flex items-center justify-center p-6">
      <div className="w-full max-w-lg animate-fade-up opacity-0 delay-1">
        {/* Logo */}
        <div className="text-center mb-6">
          <div className="inline-flex flex-col items-center justify-center w-16 h-16 rounded-2xl bg-[#1F1D1A] mb-3 p-2">
            <span className="text-[#F9F7F2] text-sm font-display font-medium tracking-tight">Recap</span>
            <div className="w-8 h-0.5 bg-[#B09872] mt-0.5 rounded-full opacity-70" />
          </div>
          <h1 className="font-display text-2xl text-foreground tracking-tight">
            初始設定
          </h1>
          <p className="text-sm text-muted-foreground mt-1">
            步驟 {step} / 5
          </p>
        </div>

        {/* Progress bar */}
        <div className="h-1 bg-muted rounded-full mb-6 overflow-hidden">
          <div
            className="h-full bg-foreground transition-all duration-300 ease-out"
            style={{ width: `${progress}%` }}
          />
        </div>

        {/* Step 1: Welcome */}
        {step === 1 && (
          <Card className="p-6 animate-fade-up opacity-0 delay-2">
            <div className="text-center mb-6">
              <Sparkles className="w-10 h-10 mx-auto text-warm mb-3" strokeWidth={1.5} />
              <h2 className="text-lg font-medium text-foreground mb-2">
                歡迎使用 Recap
              </h2>
              <p className="text-sm text-muted-foreground">
                自動追蹤您的開發工作，讓報告生成更輕鬆
              </p>
            </div>

            <div className="space-y-3 mb-6">
              <div className="flex items-start gap-3 p-3 border border-border rounded-lg">
                <div className="w-7 h-7 rounded-full bg-warm/10 flex items-center justify-center shrink-0">
                  <span className="text-warm font-medium text-sm">1</span>
                </div>
                <div>
                  <p className="font-medium text-foreground text-sm">自動收集工作記錄</p>
                  <p className="text-xs text-muted-foreground">從 Claude Code、Git 自動擷取</p>
                </div>
              </div>
              <div className="flex items-start gap-3 p-3 border border-border rounded-lg">
                <div className="w-7 h-7 rounded-full bg-warm/10 flex items-center justify-center shrink-0">
                  <span className="text-warm font-medium text-sm">2</span>
                </div>
                <div>
                  <p className="font-medium text-foreground text-sm">AI 智慧摘要</p>
                  <p className="text-xs text-muted-foreground">自動生成工作摘要與時間軸</p>
                </div>
              </div>
              <div className="flex items-start gap-3 p-3 border border-border rounded-lg">
                <div className="w-7 h-7 rounded-full bg-warm/10 flex items-center justify-center shrink-0">
                  <span className="text-warm font-medium text-sm">3</span>
                </div>
                <div>
                  <p className="font-medium text-foreground text-sm">輕鬆產出報告</p>
                  <p className="text-xs text-muted-foreground">週報、月報一鍵生成</p>
                </div>
              </div>
            </div>

            <Button className="w-full" onClick={() => setStep(2)}>
              開始設定
              <ArrowRight className="w-4 h-4 ml-2" strokeWidth={1.5} />
            </Button>
          </Card>
        )}

        {/* Step 2: Profile */}
        {step === 2 && (
          <Card className="p-6 animate-fade-up opacity-0 delay-2">
            <div className="text-center mb-5">
              <div className="w-10 h-10 mx-auto rounded-full bg-warm/10 flex items-center justify-center mb-3">
                <User className="w-5 h-5 text-warm" strokeWidth={1.5} />
              </div>
              <h2 className="text-lg font-medium text-foreground mb-1">
                基本資料
              </h2>
              <p className="text-xs text-muted-foreground">
                用於報告顯示
              </p>
            </div>

            <form onSubmit={handleProfileSubmit} className="space-y-4">
              <div>
                <Label htmlFor="name" className="mb-1.5 block text-xs">
                  名稱 <span className="text-destructive">*</span>
                </Label>
                <Input
                  id="name"
                  type="text"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="您的名稱"
                  autoFocus
                />
              </div>

              <div>
                <Label htmlFor="email" className="mb-1.5 block text-xs">
                  Email <span className="text-muted-foreground">(選填)</span>
                </Label>
                <Input
                  id="email"
                  type="email"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  placeholder="your@email.com"
                />
              </div>

              <div>
                <Label htmlFor="title" className="mb-1.5 block text-xs">
                  職稱 <span className="text-muted-foreground">(選填)</span>
                </Label>
                <Input
                  id="title"
                  type="text"
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  placeholder="例如：軟體工程師"
                />
              </div>

              {error && (
                <div className="p-2.5 bg-destructive/10 text-destructive text-xs border-l-2 border-destructive">
                  {error}
                </div>
              )}

              <div className="flex gap-3 pt-2">
                <Button type="button" variant="outline" onClick={() => setStep(1)} className="flex-1">
                  <ArrowLeft className="w-4 h-4 mr-1" strokeWidth={1.5} />
                  返回
                </Button>
                <Button type="submit" disabled={loading || !name.trim()} className="flex-1">
                  {loading ? (
                    <Loader2 className="w-4 h-4 animate-spin" />
                  ) : (
                    <>
                      下一步
                      <ArrowRight className="w-4 h-4 ml-1" strokeWidth={1.5} />
                    </>
                  )}
                </Button>
              </div>
            </form>
          </Card>
        )}

        {/* Step 3: Data Sources */}
        {step === 3 && (
          <Card className="p-6 animate-fade-up opacity-0 delay-2">
            <div className="text-center mb-5">
              <div className="w-10 h-10 mx-auto rounded-full bg-warm/10 flex items-center justify-center mb-3">
                <FolderOpen className="w-5 h-5 text-warm" strokeWidth={1.5} />
              </div>
              <h2 className="text-lg font-medium text-foreground mb-1">
                資料來源
              </h2>
              <p className="text-xs text-muted-foreground">
                確認 Recap 可以存取的資料來源
              </p>
            </div>

            <div className="space-y-3 mb-6">
              {/* Claude Code */}
              <div className="p-3 border border-border rounded-lg">
                <div className="flex items-center gap-2 mb-2">
                  <ClaudeIcon className="w-4 h-4" />
                  <span className="text-sm font-medium">Claude Code</span>
                  {claudePathLoading ? (
                    <Loader2 className="w-3.5 h-3.5 animate-spin text-muted-foreground ml-auto" />
                  ) : claudePath ? (
                    <CheckCircle2 className="w-3.5 h-3.5 text-sage ml-auto" />
                  ) : (
                    <XCircle className="w-3.5 h-3.5 text-muted-foreground ml-auto" />
                  )}
                </div>
                <p className="text-xs text-muted-foreground">
                  {claudePathLoading ? '檢查中...' : claudePath || '未偵測到路徑'}
                </p>
              </div>

            </div>

            <p className="text-xs text-muted-foreground mb-4 p-2.5 bg-muted/50 rounded">
              Git 專案目錄可以稍後在「設定 → 專案」中新增
            </p>

            <div className="flex gap-3">
              <Button variant="outline" onClick={() => setStep(2)} className="flex-1">
                <ArrowLeft className="w-4 h-4 mr-1" strokeWidth={1.5} />
                返回
              </Button>
              <Button onClick={() => { setError(''); setStep(4) }} className="flex-1">
                下一步
                <ArrowRight className="w-4 h-4 ml-1" strokeWidth={1.5} />
              </Button>
            </div>
          </Card>
        )}

        {/* Step 4: LLM Setup (Required) */}
        {step === 4 && (
          <Card className="p-6 animate-fade-up opacity-0 delay-2">
            <div className="text-center mb-5">
              <div className="w-10 h-10 mx-auto rounded-full bg-amber-500/10 flex items-center justify-center mb-3">
                <Sparkles className="w-5 h-5 text-amber-600" strokeWidth={1.5} />
              </div>
              <h2 className="text-lg font-medium text-foreground mb-1">
                AI 設定
              </h2>
              <p className="text-xs text-muted-foreground">
                設定 LLM 用於生成工作摘要（必填）
              </p>
            </div>

            <div className="space-y-4">
              {/* Provider Selection */}
              <div>
                <Label className="mb-2 block text-xs">提供者</Label>
                <div className="grid grid-cols-2 gap-2">
                  {LLM_PROVIDERS.map((provider) => (
                    <button
                      key={provider.id}
                      type="button"
                      onClick={() => handleProviderChange(provider.id)}
                      className={`p-2.5 text-left border rounded-lg transition-colors ${
                        llmProvider === provider.id
                          ? 'border-foreground bg-foreground/5'
                          : 'border-border hover:border-foreground/30'
                      }`}
                    >
                      <p className="text-sm font-medium">{provider.label}</p>
                      <p className="text-[10px] text-muted-foreground">{provider.desc}</p>
                    </button>
                  ))}
                </div>
              </div>

              {/* Model */}
              <div>
                <Label htmlFor="llm-model" className="mb-1.5 block text-xs">
                  模型名稱 <span className="text-destructive">*</span>
                </Label>
                <Input
                  id="llm-model"
                  value={llmModel}
                  onChange={(e) => setLlmModel(e.target.value)}
                  placeholder="輸入模型名稱"
                />
              </div>

              {/* API Key (not for Ollama) */}
              {llmProvider !== 'ollama' && (
                <div>
                  <Label htmlFor="llm-api-key" className="mb-1.5 block text-xs">
                    API Key <span className="text-destructive">*</span>
                  </Label>
                  <div className="relative">
                    <Input
                      id="llm-api-key"
                      type={showLlmKey ? 'text' : 'password'}
                      value={llmApiKey}
                      onChange={(e) => setLlmApiKey(e.target.value)}
                      placeholder="輸入 API Key"
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
                  <Label htmlFor="llm-base-url" className="mb-1.5 block text-xs">
                    API URL <span className="text-destructive">*</span>
                  </Label>
                  <Input
                    id="llm-base-url"
                    type="url"
                    value={llmBaseUrl}
                    onChange={(e) => setLlmBaseUrl(e.target.value)}
                    placeholder={llmProvider === 'ollama' ? 'http://localhost:11434' : 'https://your-api.example.com/v1'}
                  />
                </div>
              )}

              {error && (
                <div className="p-2.5 bg-destructive/10 text-destructive text-xs border-l-2 border-destructive">
                  {error}
                </div>
              )}
            </div>

            <div className="flex gap-3 mt-6">
              <Button variant="outline" onClick={() => { setError(''); setStep(3) }} className="flex-1">
                <ArrowLeft className="w-4 h-4 mr-1" strokeWidth={1.5} />
                返回
              </Button>
              <Button onClick={handleLlmSubmit} disabled={loading} className="flex-1">
                {loading ? (
                  <Loader2 className="w-4 h-4 animate-spin" />
                ) : (
                  <>
                    完成設定
                    <ArrowRight className="w-4 h-4 ml-1" strokeWidth={1.5} />
                  </>
                )}
              </Button>
            </div>
          </Card>
        )}

        {/* Step 5: Complete */}
        {step === 5 && (
          <Card className="p-6 animate-fade-up opacity-0 delay-2">
            <div className="text-center mb-6">
              <div className="w-14 h-14 mx-auto rounded-full bg-sage/10 flex items-center justify-center mb-4">
                <Check className="w-7 h-7 text-sage" strokeWidth={2} />
              </div>
              <h2 className="text-lg font-medium text-foreground mb-2">
                設定完成！
              </h2>
              <p className="text-sm text-muted-foreground">
                Recap 已準備好開始追蹤您的工作
              </p>
            </div>

            <div className="space-y-2 mb-6 p-3 bg-muted/50 rounded-lg">
              <div className="flex items-center gap-2 text-sm">
                <CheckCircle2 className="w-4 h-4 text-sage" />
                <span>基本資料已設定</span>
              </div>
              <div className="flex items-center gap-2 text-sm">
                <CheckCircle2 className="w-4 h-4 text-sage" />
                <span>資料來源已確認</span>
              </div>
              <div className="flex items-center gap-2 text-sm">
                <CheckCircle2 className="w-4 h-4 text-sage" />
                <span>AI 模型已設定</span>
              </div>
            </div>

            <Button className="w-full" onClick={handleComplete}>
              開始使用 Recap
              <ArrowRight className="w-4 h-4 ml-2" strokeWidth={1.5} />
            </Button>
          </Card>
        )}

        <p className="text-center text-[10px] text-muted-foreground mt-4">
          Recap - 自動追蹤你的工作
        </p>
      </div>
    </div>
  )
}
