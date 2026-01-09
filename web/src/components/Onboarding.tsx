import { useState, useEffect } from 'react'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import {
  Sparkles,
  GitCommit,
  Terminal,
  Brain,
  BarChart3,
  ChevronRight,
  ChevronLeft,
  Check,
  ArrowRight,
  ArrowDown,
  Calendar,
  FolderGit2,
  FileText,
  Clock,
  Layers,
  Upload,
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
  icon: React.ReactNode
  content: React.ReactNode
}

export function Onboarding({ open, onComplete }: OnboardingProps) {
  const [currentStep, setCurrentStep] = useState(0)

  const steps: Step[] = [
    {
      id: 'welcome',
      title: '歡迎使用 Recap',
      subtitle: '自動化工作紀錄與報告',
      icon: <Sparkles className="w-12 h-12 text-primary" />,
      content: (
        <div className="space-y-6 text-center">
          <div className="w-20 h-20 mx-auto rounded-2xl bg-primary/10 flex items-center justify-center">
            <Sparkles className="w-10 h-10 text-primary" />
          </div>
          <div className="space-y-2">
            <h3 className="text-xl font-bold text-foreground">
              省去繁瑣的工作紀錄
            </h3>
            <p className="text-muted-foreground text-sm max-w-md mx-auto">
              Recap 自動收集您的開發工作，整理成結構化的工作項目，
              讓您輕鬆生成報告、填寫工時。
            </p>
          </div>

          {/* Problem -> Solution */}
          <div className="grid grid-cols-2 gap-4 pt-4">
            <div className="p-4 rounded-lg bg-red-50 border border-red-100">
              <p className="text-xs text-red-600 font-medium mb-2">以前...</p>
              <ul className="text-xs text-red-700/80 space-y-1 text-left">
                <li>• 手動回想做了什麼</li>
                <li>• 翻找 commits 和對話</li>
                <li>• 整理成報告耗時</li>
              </ul>
            </div>
            <div className="p-4 rounded-lg bg-green-50 border border-green-100">
              <p className="text-xs text-green-600 font-medium mb-2">現在</p>
              <ul className="text-xs text-green-700/80 space-y-1 text-left">
                <li>• 自動收集工作紀錄</li>
                <li>• 按日期整理工作項目</li>
                <li>• 一鍵生成報告</li>
              </ul>
            </div>
          </div>
        </div>
      )
    },
    {
      id: 'data-flow',
      title: '資料如何整合',
      subtitle: '了解工作項目的來源',
      icon: <Layers className="w-12 h-12 text-primary" />,
      content: (
        <div className="space-y-4">
          {/* Visual flow diagram */}
          <div className="relative">
            {/* Sources */}
            <div className="grid grid-cols-2 gap-3">
              <div className="p-3 rounded-lg border-2 border-purple-200 bg-purple-50">
                <div className="flex items-center gap-2 mb-2">
                  <Terminal className="w-4 h-4 text-purple-600" />
                  <span className="text-sm font-medium text-purple-700">Claude Code</span>
                </div>
                <p className="text-xs text-purple-600">
                  開發對話、使用的工具、修改的檔案
                </p>
              </div>
              <div className="p-3 rounded-lg border-2 border-emerald-200 bg-emerald-50">
                <div className="flex items-center gap-2 mb-2">
                  <GitCommit className="w-4 h-4 text-emerald-600" />
                  <span className="text-sm font-medium text-emerald-700">Git Commits</span>
                </div>
                <p className="text-xs text-emerald-600">
                  提交記錄、變更內容、提交訊息
                </p>
              </div>
            </div>

            {/* Arrow down */}
            <div className="flex justify-center py-3">
              <div className="flex flex-col items-center">
                <ArrowDown className="w-5 h-5 text-muted-foreground" />
                <span className="text-[10px] text-muted-foreground mt-1">自動整合</span>
              </div>
            </div>

            {/* Work Item */}
            <div className="p-4 rounded-lg border-2 border-primary bg-primary/5">
              <div className="flex items-center gap-2 mb-3">
                <Calendar className="w-5 h-5 text-primary" />
                <span className="font-medium text-primary">工作項目（每日）</span>
              </div>
              <div className="grid grid-cols-3 gap-2 text-xs">
                <div className="p-2 rounded bg-white/80 text-center">
                  <p className="font-medium">專案</p>
                  <p className="text-muted-foreground">worklog-helper</p>
                </div>
                <div className="p-2 rounded bg-white/80 text-center">
                  <p className="font-medium">日期</p>
                  <p className="text-muted-foreground">2026-01-09</p>
                </div>
                <div className="p-2 rounded bg-white/80 text-center">
                  <p className="font-medium">時數</p>
                  <p className="text-muted-foreground">3.5h</p>
                </div>
              </div>
            </div>
          </div>

          <div className="p-3 rounded-lg border border-amber-200 bg-amber-50">
            <p className="text-xs text-amber-700">
              <strong>關鍵概念：</strong>同一專案、同一天的所有 sessions 和 commits
              會自動合併成一個「工作項目」，方便管理和報告。
            </p>
          </div>
        </div>
      )
    },
    {
      id: 'workflow',
      title: '工作流程',
      subtitle: '從收集到報告的完整流程',
      icon: <ArrowRight className="w-12 h-12 text-primary" />,
      content: (
        <div className="space-y-3">
          {/* Step 1 */}
          <div className="flex gap-3">
            <div className="flex flex-col items-center">
              <div className="w-8 h-8 rounded-full bg-primary text-white flex items-center justify-center text-sm font-bold">
                1
              </div>
              <div className="w-0.5 h-full bg-primary/20 mt-2" />
            </div>
            <div className="flex-1 pb-4">
              <h4 className="font-medium text-sm">選擇專案同步</h4>
              <p className="text-xs text-muted-foreground mt-1">
                前往「設定」→「整合服務」→「Claude Code」，勾選要追蹤的專案
              </p>
              <div className="mt-2 p-2 rounded bg-muted/50 text-xs">
                <code className="text-purple-600">設定 → 整合服務 → Claude Code</code>
              </div>
            </div>
          </div>

          {/* Step 2 */}
          <div className="flex gap-3">
            <div className="flex flex-col items-center">
              <div className="w-8 h-8 rounded-full bg-primary text-white flex items-center justify-center text-sm font-bold">
                2
              </div>
              <div className="w-0.5 h-full bg-primary/20 mt-2" />
            </div>
            <div className="flex-1 pb-4">
              <h4 className="font-medium text-sm">點擊「匯入為工作項目」</h4>
              <p className="text-xs text-muted-foreground mt-1">
                系統會自動：讀取 sessions → 抓取 git commits → 按日期整合
              </p>
              <div className="mt-2 flex items-center gap-2 text-xs text-muted-foreground">
                <Terminal className="w-3 h-3" />
                <span>+</span>
                <GitCommit className="w-3 h-3" />
                <ArrowRight className="w-3 h-3" />
                <Calendar className="w-3 h-3" />
                <span>工作項目</span>
              </div>
            </div>
          </div>

          {/* Step 3 */}
          <div className="flex gap-3">
            <div className="flex flex-col items-center">
              <div className="w-8 h-8 rounded-full bg-primary text-white flex items-center justify-center text-sm font-bold">
                3
              </div>
              <div className="w-0.5 h-full bg-primary/20 mt-2" />
            </div>
            <div className="flex-1 pb-4">
              <h4 className="font-medium text-sm">查看與管理工作項目</h4>
              <p className="text-xs text-muted-foreground mt-1">
                在「工作項目」頁面查看、編輯、對應 Jira Issue
              </p>
            </div>
          </div>

          {/* Step 4 */}
          <div className="flex gap-3">
            <div className="flex flex-col items-center">
              <div className="w-8 h-8 rounded-full bg-primary text-white flex items-center justify-center text-sm font-bold">
                4
              </div>
            </div>
            <div className="flex-1">
              <h4 className="font-medium text-sm">生成報告 / 同步 Tempo</h4>
              <p className="text-xs text-muted-foreground mt-1">
                選擇日期範圍，一鍵產出日報、週報，或同步到 Jira Tempo
              </p>
            </div>
          </div>
        </div>
      )
    },
    {
      id: 'work-items',
      title: '工作項目管理',
      subtitle: '編輯、分類、對應 Jira',
      icon: <FolderGit2 className="w-12 h-12 text-primary" />,
      content: (
        <div className="space-y-4">
          {/* Sample work item card */}
          <div className="p-4 rounded-lg border border-border bg-card">
            <div className="flex items-start justify-between mb-3">
              <div>
                <h4 className="font-medium text-sm">[worklog-helper] 3 commits: Add onboarding...</h4>
                <p className="text-xs text-muted-foreground">2026-01-09</p>
              </div>
              <span className="text-xs bg-primary/10 text-primary px-2 py-0.5 rounded">3.5h</span>
            </div>
            <div className="space-y-2 text-xs">
              <div className="flex items-center gap-2">
                <GitCommit className="w-3 h-3 text-emerald-600" />
                <span className="text-muted-foreground">abc1234 - Add onboarding tutorial</span>
              </div>
              <div className="flex items-center gap-2">
                <GitCommit className="w-3 h-3 text-emerald-600" />
                <span className="text-muted-foreground">def5678 - Fix email field editing</span>
              </div>
            </div>
          </div>

          {/* Actions explanation */}
          <div className="grid grid-cols-2 gap-3">
            <div className="p-3 rounded-lg bg-muted/50">
              <div className="flex items-center gap-2 mb-2">
                <Clock className="w-4 h-4 text-primary" />
                <span className="text-xs font-medium">調整時數</span>
              </div>
              <p className="text-[11px] text-muted-foreground">
                修正自動計算的工時
              </p>
            </div>
            <div className="p-3 rounded-lg bg-muted/50">
              <div className="flex items-center gap-2 mb-2">
                <Upload className="w-4 h-4 text-blue-600" />
                <span className="text-xs font-medium">對應 Jira</span>
              </div>
              <p className="text-[11px] text-muted-foreground">
                連結到 Jira Issue
              </p>
            </div>
            <div className="p-3 rounded-lg bg-muted/50">
              <div className="flex items-center gap-2 mb-2">
                <Brain className="w-4 h-4 text-amber-600" />
                <span className="text-xs font-medium">AI 摘要</span>
              </div>
              <p className="text-[11px] text-muted-foreground">
                生成工作內容摘要
              </p>
            </div>
            <div className="p-3 rounded-lg bg-muted/50">
              <div className="flex items-center gap-2 mb-2">
                <FileText className="w-4 h-4 text-purple-600" />
                <span className="text-xs font-medium">分類標籤</span>
              </div>
              <p className="text-[11px] text-muted-foreground">
                加入標籤便於篩選
              </p>
            </div>
          </div>

          <div className="p-3 rounded-lg border border-primary/20 bg-primary/5">
            <p className="text-xs text-primary">
              <Sparkles className="w-3 h-3 inline mr-1" />
              提示：工作項目可重複同步，新的 sessions 和 commits 會自動合併
            </p>
          </div>
        </div>
      )
    },
    {
      id: 'reports',
      title: '報告與工時同步',
      subtitle: '產出報告、填寫 Tempo',
      icon: <BarChart3 className="w-12 h-12 text-primary" />,
      content: (
        <div className="space-y-4">
          {/* Report types */}
          <div className="grid grid-cols-3 gap-2">
            <div className="p-3 rounded-lg border border-border text-center">
              <FileText className="w-5 h-5 mx-auto mb-1 text-primary" />
              <p className="text-xs font-medium">日報</p>
            </div>
            <div className="p-3 rounded-lg border border-border text-center">
              <FileText className="w-5 h-5 mx-auto mb-1 text-primary" />
              <p className="text-xs font-medium">週報</p>
            </div>
            <div className="p-3 rounded-lg border border-border text-center">
              <FileText className="w-5 h-5 mx-auto mb-1 text-primary" />
              <p className="text-xs font-medium">月報</p>
            </div>
          </div>

          {/* Tempo sync */}
          <div className="p-4 rounded-lg border-2 border-blue-200 bg-blue-50">
            <div className="flex items-center gap-2 mb-2">
              <Upload className="w-4 h-4 text-blue-600" />
              <span className="text-sm font-medium text-blue-700">Jira Tempo 同步</span>
            </div>
            <p className="text-xs text-blue-600 mb-3">
              將工作項目同步到 Jira Tempo 工時系統
            </p>
            <ol className="text-xs text-blue-700/80 space-y-1">
              <li>1. 設定 Jira URL 和 Token（設定頁面）</li>
              <li>2. 工作項目對應 Jira Issue</li>
              <li>3. 點擊「同步到 Tempo」</li>
            </ol>
          </div>

          {/* Quick setup checklist */}
          <div className="p-4 rounded-lg bg-muted/50">
            <p className="text-xs font-medium mb-2">快速設定清單：</p>
            <div className="space-y-2">
              <label className="flex items-center gap-2 text-xs cursor-pointer">
                <input type="checkbox" className="w-3 h-3" />
                <span>同步 Claude Code 專案</span>
              </label>
              <label className="flex items-center gap-2 text-xs cursor-pointer">
                <input type="checkbox" className="w-3 h-3" />
                <span>設定 LLM（選用，用於 AI 摘要）</span>
              </label>
              <label className="flex items-center gap-2 text-xs cursor-pointer">
                <input type="checkbox" className="w-3 h-3" />
                <span>設定 Jira/Tempo（選用，用於工時同步）</span>
              </label>
            </div>
          </div>
        </div>
      )
    },
    {
      id: 'start',
      title: '開始使用',
      subtitle: '準備就緒！',
      icon: <Check className="w-12 h-12 text-primary" />,
      content: (
        <div className="space-y-6">
          <div className="text-center">
            <div className="w-16 h-16 mx-auto rounded-full bg-green-100 flex items-center justify-center mb-4">
              <Check className="w-8 h-8 text-green-600" />
            </div>
            <h3 className="text-lg font-bold text-foreground mb-2">
              教學完成！
            </h3>
            <p className="text-sm text-muted-foreground">
              您已了解 Recap 的基本使用方式
            </p>
          </div>

          {/* Quick actions */}
          <div className="space-y-2">
            <p className="text-xs font-medium text-muted-foreground">建議的下一步：</p>
            <div className="p-3 rounded-lg border border-primary/30 bg-primary/5 flex items-center gap-3">
              <div className="w-8 h-8 rounded-full bg-primary/20 flex items-center justify-center flex-shrink-0">
                <span className="text-sm font-bold text-primary">1</span>
              </div>
              <div>
                <p className="text-sm font-medium">同步 Claude Code 專案</p>
                <p className="text-xs text-muted-foreground">設定 → 整合服務 → Claude Code</p>
              </div>
            </div>
            <div className="p-3 rounded-lg border border-border flex items-center gap-3">
              <div className="w-8 h-8 rounded-full bg-muted flex items-center justify-center flex-shrink-0">
                <span className="text-sm font-bold text-muted-foreground">2</span>
              </div>
              <div>
                <p className="text-sm font-medium">查看儀表板</p>
                <p className="text-xs text-muted-foreground">查看本週工時和活動概覽</p>
              </div>
            </div>
          </div>

          <div className="p-3 rounded-lg bg-muted/50 text-center">
            <p className="text-xs text-muted-foreground">
              隨時點擊側邊欄的 <span className="inline-flex items-center"><span className="w-4 h-4 rounded border inline-flex items-center justify-center text-[10px]">?</span></span> 重新查看教學
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
      <DialogContent className="sm:max-w-[540px] p-0 gap-0 overflow-hidden" hideCloseButton>
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

          <div className="min-h-[340px]">
            {currentStepData.content}
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
