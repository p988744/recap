import { useState, useEffect } from 'react'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import {
  Sparkles,
  CalendarDays,
  FolderKanban,
  Settings,
  ChevronRight,
  ChevronLeft,
  Check,
} from 'lucide-react'
import { cn } from '@/lib/utils'

interface OnboardingProps {
  open: boolean
  onComplete: () => void
}

interface Step {
  id: string
  title: string
  subtitle: string
  content: React.ReactNode
}

export function Onboarding({ open, onComplete }: OnboardingProps) {
  const [currentStep, setCurrentStep] = useState(0)

  const steps: Step[] = [
    {
      id: 'welcome',
      title: '快速導覽',
      subtitle: '了解 Recap 的主要功能',
      content: (
        <div className="space-y-6 text-center">
          <div className="w-16 h-16 mx-auto rounded-2xl bg-primary/10 flex items-center justify-center">
            <Sparkles className="w-8 h-8 text-primary" />
          </div>
          <div className="space-y-2">
            <h3 className="text-lg font-bold text-foreground">
              歡迎使用 Recap
            </h3>
            <p className="text-sm text-muted-foreground max-w-sm mx-auto">
              Recap 會自動收集您的開發工作，整理成結構化的時間軸，讓您輕鬆追蹤進度。
            </p>
          </div>
        </div>
      )
    },
    {
      id: 'navigation',
      title: '主要頁面',
      subtitle: '認識導覽結構',
      content: (
        <div className="space-y-4">
          <div className="flex items-start gap-4 p-4 rounded-lg border border-border">
            <div className="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center shrink-0">
              <CalendarDays className="w-5 h-5 text-primary" />
            </div>
            <div>
              <h4 className="font-medium text-foreground">本週工作</h4>
              <p className="text-sm text-muted-foreground mt-1">
                查看本週的工作時間軸，按日期瀏覽每天的工作記錄
              </p>
            </div>
          </div>

          <div className="flex items-start gap-4 p-4 rounded-lg border border-border">
            <div className="w-10 h-10 rounded-lg bg-emerald-500/10 flex items-center justify-center shrink-0">
              <FolderKanban className="w-5 h-5 text-emerald-600" />
            </div>
            <div>
              <h4 className="font-medium text-foreground">專案</h4>
              <p className="text-sm text-muted-foreground mt-1">
                按專案查看工作摘要，包含週報、月報等時間軸視圖
              </p>
            </div>
          </div>

          <div className="flex items-start gap-4 p-4 rounded-lg border border-border">
            <div className="w-10 h-10 rounded-lg bg-slate-500/10 flex items-center justify-center shrink-0">
              <Settings className="w-5 h-5 text-slate-600" />
            </div>
            <div>
              <h4 className="font-medium text-foreground">設定</h4>
              <p className="text-sm text-muted-foreground mt-1">
                管理資料來源、AI 模型、同步設定等
              </p>
            </div>
          </div>
        </div>
      )
    },
    {
      id: 'start',
      title: '開始使用',
      subtitle: '準備就緒',
      content: (
        <div className="space-y-6 text-center">
          <div className="w-16 h-16 mx-auto rounded-full bg-green-100 flex items-center justify-center">
            <Check className="w-8 h-8 text-green-600" />
          </div>
          <div className="space-y-2">
            <h3 className="text-lg font-bold text-foreground">
              準備完成！
            </h3>
            <p className="text-sm text-muted-foreground max-w-sm mx-auto">
              Recap 會在背景自動同步您的工作記錄。您可以隨時點擊側邊欄的 ? 按鈕重新查看這個導覽。
            </p>
          </div>
        </div>
      )
    }
  ]

  const progress = ((currentStep + 1) / steps.length) * 100
  const currentStepData = steps[currentStep]
  const isLastStep = currentStep === steps.length - 1

  const handleNext = () => {
    if (isLastStep) {
      onComplete()
    } else {
      setCurrentStep(prev => prev + 1)
    }
  }

  const handlePrev = () => {
    setCurrentStep(prev => Math.max(0, prev - 1))
  }

  const handleSkip = () => {
    onComplete()
  }

  return (
    <Dialog open={open} onOpenChange={() => {}}>
      <DialogContent className="sm:max-w-[440px] p-0 gap-0 overflow-hidden" hideCloseButton>
        {/* Progress bar */}
        <div className="h-1 bg-muted">
          <div
            className="h-full bg-primary transition-all duration-300 ease-out"
            style={{ width: `${progress}%` }}
          />
        </div>

        {/* Step indicators */}
        <div className="flex justify-center gap-1.5 pt-5 pb-2">
          {steps.map((step, index) => (
            <button
              key={step.id}
              onClick={() => setCurrentStep(index)}
              className={cn(
                "h-1.5 rounded-full transition-all duration-200",
                index === currentStep
                  ? "w-6 bg-primary"
                  : index < currentStep
                    ? "w-1.5 bg-primary/50"
                    : "w-1.5 bg-muted-foreground/20"
              )}
            />
          ))}
        </div>

        {/* Content */}
        <div className="px-6 py-4">
          <div className="text-center mb-4">
            <p className="text-[10px] text-muted-foreground uppercase tracking-wider mb-1">
              {currentStep + 1} / {steps.length}
            </p>
            <h2 className="text-lg font-bold">{currentStepData.title}</h2>
            <p className="text-xs text-muted-foreground">{currentStepData.subtitle}</p>
          </div>

          <div className="min-h-[260px] flex items-center">
            <div className="w-full">
              {currentStepData.content}
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between px-6 py-3 border-t bg-muted/30">
          <Button
            variant="ghost"
            size="sm"
            onClick={handleSkip}
            className="text-muted-foreground hover:text-foreground text-xs"
          >
            跳過
          </Button>

          <div className="flex items-center gap-2">
            {currentStep > 0 && (
              <Button variant="outline" size="sm" onClick={handlePrev}>
                <ChevronLeft className="w-3 h-3 mr-1" />
                上一步
              </Button>
            )}
            <Button size="sm" onClick={handleNext} className="min-w-[80px]">
              {isLastStep ? (
                <>
                  開始
                  <Check className="w-3 h-3 ml-1" />
                </>
              ) : (
                <>
                  下一步
                  <ChevronRight className="w-3 h-3 ml-1" />
                </>
              )}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}

// Hook to manage onboarding state
export function useOnboarding() {
  const [showOnboarding, setShowOnboarding] = useState(false)
  const [hasCompletedOnboarding, setHasCompletedOnboarding] = useState(true)

  useEffect(() => {
    const completed = localStorage.getItem('recap_onboarding_completed')
    if (!completed) {
      setHasCompletedOnboarding(false)
      setShowOnboarding(true)
    }
  }, [])

  const completeOnboarding = () => {
    localStorage.setItem('recap_onboarding_completed', 'true')
    setHasCompletedOnboarding(true)
    setShowOnboarding(false)
  }

  const resetOnboarding = () => {
    localStorage.removeItem('recap_onboarding_completed')
    setHasCompletedOnboarding(false)
    setShowOnboarding(true)
  }

  return {
    showOnboarding,
    hasCompletedOnboarding,
    completeOnboarding,
    resetOnboarding,
    openOnboarding: () => setShowOnboarding(true)
  }
}
