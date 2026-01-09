import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuth } from '@/lib/auth'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Card } from '@/components/ui/card'
import { Loader2, ArrowRight, User, Sparkles } from 'lucide-react'

export function OnboardingPage() {
  const navigate = useNavigate()
  const { register } = useAuth()
  const [step, setStep] = useState(1)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')

  // Form data
  const [name, setName] = useState('')
  const [email, setEmail] = useState('')
  const [title, setTitle] = useState('')

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!name.trim()) {
      setError('請輸入您的名稱')
      return
    }

    setLoading(true)
    setError('')

    try {
      // Use a default email if not provided (local mode)
      const userEmail = email.trim() || `${name.toLowerCase().replace(/\s+/g, '.')}@local`
      // Use a simple password for local mode (not security critical)
      const password = 'local-mode-password'

      await register(userEmail, password, name.trim(), title.trim() || undefined)
      navigate('/')
    } catch (err) {
      setError(err instanceof Error ? err.message : '建立帳號失敗')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="min-h-screen bg-background flex items-center justify-center p-6">
      <div className="w-full max-w-lg animate-fade-up opacity-0 delay-1">
        {/* Logo */}
        <div className="text-center mb-8">
          <div className="inline-flex flex-col items-center justify-center w-20 h-20 rounded-2xl bg-[#1F1D1A] mb-4 p-3">
            <span className="text-[#F9F7F2] text-lg font-display font-medium tracking-tight">Recap</span>
            <div className="w-12 h-0.5 bg-[#B09872] mt-1 rounded-full opacity-70" />
          </div>
          <h1 className="font-display text-3xl text-foreground tracking-tight">
            歡迎使用 Recap
          </h1>
          <p className="text-muted-foreground mt-2">
            自動追蹤您的工作，讓您專注於創造價值
          </p>
        </div>

        {step === 1 && (
          <Card className="p-8 animate-fade-up opacity-0 delay-2">
            <div className="text-center mb-8">
              <Sparkles className="w-12 h-12 mx-auto text-warm mb-4" strokeWidth={1.5} />
              <h2 className="text-xl font-medium text-foreground mb-2">
                開始之前
              </h2>
              <p className="text-sm text-muted-foreground">
                Recap 會自動從 GitLab、Claude Code 等來源收集您的工作記錄，
                並協助您生成報告、同步到 Jira Tempo。
              </p>
            </div>

            <div className="space-y-4 mb-8">
              <div className="flex items-start gap-3 p-3 border border-border rounded-lg">
                <div className="w-8 h-8 rounded-full bg-warm/10 flex items-center justify-center shrink-0 mt-0.5">
                  <span className="text-warm font-medium">1</span>
                </div>
                <div>
                  <p className="font-medium text-foreground">連接資料來源</p>
                  <p className="text-sm text-muted-foreground">GitLab、Git 倉庫、Claude Code</p>
                </div>
              </div>
              <div className="flex items-start gap-3 p-3 border border-border rounded-lg">
                <div className="w-8 h-8 rounded-full bg-warm/10 flex items-center justify-center shrink-0 mt-0.5">
                  <span className="text-warm font-medium">2</span>
                </div>
                <div>
                  <p className="font-medium text-foreground">自動追蹤工作</p>
                  <p className="text-sm text-muted-foreground">Commits、MR、開發 Session 自動記錄</p>
                </div>
              </div>
              <div className="flex items-start gap-3 p-3 border border-border rounded-lg">
                <div className="w-8 h-8 rounded-full bg-warm/10 flex items-center justify-center shrink-0 mt-0.5">
                  <span className="text-warm font-medium">3</span>
                </div>
                <div>
                  <p className="font-medium text-foreground">生成報告</p>
                  <p className="text-sm text-muted-foreground">週報、月報、績效考核表一鍵產生</p>
                </div>
              </div>
            </div>

            <Button
              className="w-full"
              onClick={() => setStep(2)}
            >
              開始設定
              <ArrowRight className="w-4 h-4 ml-2" strokeWidth={1.5} />
            </Button>
          </Card>
        )}

        {step === 2 && (
          <Card className="p-8 animate-fade-up opacity-0 delay-2">
            <div className="text-center mb-6">
              <div className="w-12 h-12 mx-auto rounded-full bg-warm/10 flex items-center justify-center mb-4">
                <User className="w-6 h-6 text-warm" strokeWidth={1.5} />
              </div>
              <h2 className="text-xl font-medium text-foreground mb-2">
                建立您的帳號
              </h2>
              <p className="text-sm text-muted-foreground">
                填寫基本資訊以開始使用
              </p>
            </div>

            <form onSubmit={handleSubmit} className="space-y-4">
              <div>
                <Label htmlFor="name" className="mb-2 block">
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
                <Label htmlFor="email" className="mb-2 block">
                  Email <span className="text-muted-foreground text-xs">(選填)</span>
                </Label>
                <Input
                  id="email"
                  type="email"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  placeholder="your@email.com"
                />
                <p className="text-xs text-muted-foreground mt-1">
                  本地模式下可以不填
                </p>
              </div>

              <div>
                <Label htmlFor="title" className="mb-2 block">
                  職稱 <span className="text-muted-foreground text-xs">(選填)</span>
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
                <div className="p-3 bg-destructive/10 text-destructive text-sm border-l-2 border-destructive">
                  {error}
                </div>
              )}

              <div className="flex gap-3 pt-2">
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => setStep(1)}
                  className="flex-1"
                >
                  返回
                </Button>
                <Button
                  type="submit"
                  disabled={loading || !name.trim()}
                  className="flex-1"
                >
                  {loading ? (
                    <>
                      <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                      建立中...
                    </>
                  ) : (
                    <>
                      完成設定
                      <ArrowRight className="w-4 h-4 ml-2" strokeWidth={1.5} />
                    </>
                  )}
                </Button>
              </div>
            </form>
          </Card>
        )}

        <p className="text-center text-xs text-muted-foreground mt-6">
          Recap - 自動回顧你的工作
        </p>
      </div>
    </div>
  )
}
