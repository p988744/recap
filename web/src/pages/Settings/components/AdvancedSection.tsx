import { useState } from 'react'
import {
  AlertTriangle, RefreshCw, Trash2, RotateCcw, Loader2, Save,
} from 'lucide-react'
import { Card } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Progress } from '@/components/ui/progress'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { dangerZone } from '@/services'
import type { RecompactProgress } from '@/services/danger-zone'
import { useBackgroundTask, phaseLabels } from '@/hooks/useBackgroundTask'
import type { SettingsMessage } from '../hooks/useSettings'

const DEFAULT_SUMMARY_PROMPT = `你是工作報告助手。請根據以下工作資料，產生專業的工作摘要（{length_hint}）。
{context_section}
本時段的工作資料：
{data}

安全規則（最高優先，務必遵守）：
- 絕對不要在摘要中出現任何機密資訊，包括：IP 位址、密碼、API Key、Token、Secret、帳號密碼組合、伺服器位址、內部 URL、資料庫連線字串
- 如果原始資料包含這些機密資訊，請完全省略或用泛稱替代（如「更新伺服器認證設定」而非列出實際 IP 或密碼）

撰寫風格：
- 以「成果導向」撰寫，描述完成了什麼、推進了什麼、解決了什麼問題
- 避免「流水帳」式的步驟列舉（不要寫「使用 grep 搜尋」、「透過 bash 登入」這類操作細節）
- 每個要點應能讓主管或同事理解你的工作貢獻和價值
- 若有 git commit，以 commit 訊息作為成果總結的依據
- 程式碼中的檔名、函式名、變數名請用 \`backtick\` 包裹

請用繁體中文回答，格式如下：
1. 第一行是一句話的總結摘要，點出核心成果或貢獻（不要加前綴）
2. 空一行後，用條列式列出關鍵成果，每個要點以「- 」開頭

重要：請直接輸出完整的工作摘要內容，不要只回覆「OK」或「好的」。`

interface AdvancedSectionProps {
  // LLM params
  summaryMaxChars: number
  setSummaryMaxChars: (v: number) => void
  summaryReasoningEffort: string
  setSummaryReasoningEffort: (v: string) => void
  summaryPrompt: string
  setSummaryPrompt: (v: string) => void
  savingSync: boolean
  onSaveSync: (setMessage: (msg: SettingsMessage | null) => void) => Promise<void>
  // Shared
  setMessage: (msg: { type: 'success' | 'error'; text: string } | null) => void
}

type OperationType = 'recompact' | 'clear_synced' | 'factory_reset' | null

interface OperationConfig {
  title: string
  description: string
  warning: string
  confirmText: string
  confirmPlaceholder: string
  buttonText: string
  buttonClass: string
}

const operationConfigs: Record<Exclude<OperationType, null>, OperationConfig> = {
  recompact: {
    title: '重新計算所有摘要',
    description: '此操作將刪除所有已生成的工作摘要（hourly、daily、weekly、monthly），並從原始快照資料重新計算。原始工作紀錄和快照資料不會受到影響。',
    warning: '這可能需要較長時間，取決於您的資料量。在處理期間請勿關閉應用程式。',
    confirmText: 'RECOMPACT',
    confirmPlaceholder: '輸入 RECOMPACT 確認',
    buttonText: '開始重新計算',
    buttonClass: 'bg-amber-600 hover:bg-amber-700',
  },
  clear_synced: {
    title: '清除所有同步資料',
    description: '此操作將刪除所有從資料來源同步的工作紀錄（Claude Code、Antigravity、Git 等），以及所有快照和摘要資料。',
    warning: '手動建立的工作紀錄將保留，但所有自動同步的資料都會永久刪除且無法復原。',
    confirmText: 'DELETE_SYNCED_DATA',
    confirmPlaceholder: '輸入 DELETE_SYNCED_DATA 確認',
    buttonText: '清除同步資料',
    buttonClass: 'bg-orange-600 hover:bg-orange-700',
  },
  factory_reset: {
    title: '重置所有資料與設定',
    description: '此操作將完全重置您的帳號，包括：所有工作紀錄（含手動建立）、所有快照和摘要、所有報表、所有專案設定、所有整合設定（Jira、GitLab、LLM 等）。',
    warning: '這是不可逆的操作！所有資料都會永久刪除，設定會恢復為預設值。請確保您已備份重要資料。',
    confirmText: 'FACTORY_RESET',
    confirmPlaceholder: '輸入 FACTORY_RESET 確認',
    buttonText: '完全重置',
    buttonClass: 'bg-red-600 hover:bg-red-700',
  },
}

export function AdvancedSection({
  summaryMaxChars,
  setSummaryMaxChars,
  summaryReasoningEffort,
  setSummaryReasoningEffort,
  summaryPrompt,
  setSummaryPrompt,
  savingSync,
  onSaveSync,
  setMessage,
}: AdvancedSectionProps) {
  const [activeOperation, setActiveOperation] = useState<OperationType>(null)
  const [confirmInput, setConfirmInput] = useState('')
  const [loading, setLoading] = useState(false)
  const [progress, setProgress] = useState<RecompactProgress | null>(null)

  // Background task context (shared with Layout sidebar)
  const { task, startTask, updateProgress, completeTask, setTaskError } = useBackgroundTask()

  const handleOpenDialog = (operation: OperationType) => {
    // If recompact is running in background, show the dialog with current progress
    if (operation === 'recompact' && task.isRunning && task.taskType === 'recompact') {
      setActiveOperation('recompact')
      setProgress(task.progress)
      setLoading(true)
      return
    }
    setActiveOperation(operation)
    setConfirmInput('')
    setProgress(null)
  }

  const handleCloseDialog = () => {
    // For recompact, allow running in background (just close the dialog)
    if (activeOperation === 'recompact' && loading) {
      setActiveOperation(null)
      return
    }
    if (!loading) {
      setActiveOperation(null)
      setConfirmInput('')
      setProgress(null)
    }
  }

  const handleExecute = async () => {
    if (!activeOperation) return

    const config = operationConfigs[activeOperation]
    if (confirmInput !== config.confirmText) {
      setMessage({ type: 'error', text: '確認文字不正確' })
      return
    }

    setLoading(true)
    setProgress(null)

    // For recompact, start background task tracking
    if (activeOperation === 'recompact') {
      startTask('recompact')
    }

    try {
      switch (activeOperation) {
        case 'recompact': {
          const result = await dangerZone.forceRecompactWithProgress(
            confirmInput,
            (p) => {
              setProgress(p)
              updateProgress(p)
            }
          )
          if (result.success) {
            setMessage({ type: 'success', text: result.message })
          } else {
            setMessage({ type: 'error', text: result.message })
          }
          completeTask()
          break
        }
        case 'clear_synced': {
          const result = await dangerZone.clearSyncedData(confirmInput)
          if (result.success) {
            setMessage({ type: 'success', text: result.message })
          } else {
            setMessage({ type: 'error', text: result.message })
          }
          break
        }
        case 'factory_reset': {
          const result = await dangerZone.factoryReset(confirmInput)
          if (result.success) {
            setMessage({ type: 'success', text: result.message })
            // Optionally reload the page to reflect reset state
            setTimeout(() => window.location.reload(), 1500)
          } else {
            setMessage({ type: 'error', text: result.message })
          }
          break
        }
      }
      setActiveOperation(null)
      setConfirmInput('')
      setProgress(null)
    } catch (error) {
      setMessage({ type: 'error', text: `操作失敗：${error}` })
      if (activeOperation === 'recompact') {
        setTaskError(`${error}`)
      }
    } finally {
      setLoading(false)
    }
  }

  const config = activeOperation ? operationConfigs[activeOperation] : null
  const isConfirmValid = config ? confirmInput === config.confirmText : false

  // Calculate progress percentage
  const progressPercent = progress
    ? progress.total > 0
      ? Math.round((progress.current / progress.total) * 100)
      : progress.phase === 'complete' ? 100 : 0
    : 0

  return (
    <section className="animate-fade-up opacity-0 delay-1">
      <h2 className="font-display text-2xl text-foreground mb-6">進階設定</h2>

      {/* LLM Parameters Card */}
      <Card className="p-6 mb-6">
        <h3 className="font-medium mb-4">LLM 參數</h3>

        <div className="space-y-6">
          {/* Summary Max Chars */}
          <div>
            <Label className="mb-2 block">摘要最大字數</Label>
            <p className="text-xs text-muted-foreground mb-2">
              控制 LLM 生成摘要的長度上限
            </p>
            <select
              value={summaryMaxChars}
              onChange={(e) => setSummaryMaxChars(Number(e.target.value))}
              className="px-3 py-2 bg-background border border-border text-sm focus:outline-none focus:ring-1 focus:ring-foreground"
            >
              <option value={500}>500 字</option>
              <option value={1000}>1000 字</option>
              <option value={1500}>1500 字</option>
              <option value={2000}>2000 字（預設）</option>
              <option value={3000}>3000 字</option>
              <option value={5000}>5000 字</option>
            </select>
          </div>

          {/* Reasoning Effort */}
          <div>
            <Label className="mb-2 block">推理強度</Label>
            <p className="text-xs text-muted-foreground mb-2">
              控制 LLM 的推理深度（僅適用於 OpenAI o 系列及 GPT-5 模型）
            </p>
            <select
              value={summaryReasoningEffort}
              onChange={(e) => setSummaryReasoningEffort(e.target.value)}
              className="px-3 py-2 bg-background border border-border text-sm focus:outline-none focus:ring-1 focus:ring-foreground"
            >
              <option value="low">低 — 較快、較省 Token</option>
              <option value="medium">中 — 平衡（預設）</option>
              <option value="high">高 — 較慢、較精確</option>
            </select>
          </div>

          {/* Summary Prompt */}
          <div>
            <div className="flex items-center justify-between mb-2">
              <Label className="block">摘要 Prompt</Label>
              {summaryPrompt && (
                <button
                  onClick={() => setSummaryPrompt('')}
                  className="text-xs text-muted-foreground hover:text-foreground transition-colors flex items-center gap-1"
                >
                  <RotateCcw className="w-3 h-3" />
                  恢復預設
                </button>
              )}
            </div>
            <p className="text-xs text-muted-foreground mb-2">
              自訂 LLM 生成摘要時使用的 Prompt。可用變數：<code className="px-1 py-0.5 bg-muted rounded text-[11px]">{'{data}'}</code>（工作資料）、<code className="px-1 py-0.5 bg-muted rounded text-[11px]">{'{length_hint}'}</code>（字數提示）、<code className="px-1 py-0.5 bg-muted rounded text-[11px]">{'{context_section}'}</code>（前期摘要）。留空則使用預設 Prompt。
            </p>
            <textarea
              value={summaryPrompt}
              onChange={(e) => setSummaryPrompt(e.target.value)}
              placeholder={DEFAULT_SUMMARY_PROMPT}
              rows={10}
              className="w-full px-3 py-2 bg-background border border-border text-sm font-mono focus:outline-none focus:ring-1 focus:ring-foreground resize-y leading-relaxed"
            />
          </div>

          {/* Save Button */}
          <div className="pt-4 border-t border-border">
            <Button onClick={() => onSaveSync(setMessage)} disabled={savingSync}>
              {savingSync ? (
                <Loader2 className="w-4 h-4 animate-spin" />
              ) : (
                <Save className="w-4 h-4" />
              )}
              {savingSync ? '儲存中...' : '儲存 LLM 設定'}
            </Button>
          </div>
        </div>
      </Card>

      {/* Danger Zone Card */}
      <Card className="border-destructive/30 bg-destructive/5">
        <div className="p-6">
          <div className="flex items-center gap-2 mb-4">
            <AlertTriangle className="w-5 h-5 text-destructive" />
            <h3 className="font-medium text-destructive">危險區域</h3>
          </div>
          <p className="text-sm text-muted-foreground mb-6">
            以下操作可能會永久刪除您的資料，請謹慎操作。建議在執行前先匯出重要資料。
          </p>

          <div className="space-y-4">
            {/* Recompact Summaries */}
            <div className="flex items-center justify-between p-4 bg-background border border-border rounded-lg">
              <div className="flex-1 mr-4">
                <div className="flex items-center gap-2 mb-1">
                  <RefreshCw className="w-4 h-4 text-amber-600" />
                  <span className="font-medium text-foreground">重新計算所有摘要</span>
                </div>
                <p className="text-xs text-muted-foreground">
                  刪除所有工作摘要並從快照重新生成，用於更新摘要演算法後的資料回溯
                </p>
              </div>
              <button
                onClick={() => handleOpenDialog('recompact')}
                className="px-4 py-2 text-sm font-medium text-white bg-amber-600 hover:bg-amber-700 rounded-md transition-colors"
              >
                重新計算
              </button>
            </div>

            {/* Clear Synced Data */}
            <div className="flex items-center justify-between p-4 bg-background border border-border rounded-lg">
              <div className="flex-1 mr-4">
                <div className="flex items-center gap-2 mb-1">
                  <Trash2 className="w-4 h-4 text-orange-600" />
                  <span className="font-medium text-foreground">清除所有同步資料</span>
                </div>
                <p className="text-xs text-muted-foreground">
                  刪除所有自動同步的工作紀錄和快照，保留手動建立的紀錄
                </p>
              </div>
              <button
                onClick={() => handleOpenDialog('clear_synced')}
                className="px-4 py-2 text-sm font-medium text-white bg-orange-600 hover:bg-orange-700 rounded-md transition-colors"
              >
                清除資料
              </button>
            </div>

            {/* Factory Reset */}
            <div className="flex items-center justify-between p-4 bg-background border border-destructive/30 rounded-lg">
              <div className="flex-1 mr-4">
                <div className="flex items-center gap-2 mb-1">
                  <RotateCcw className="w-4 h-4 text-destructive" />
                  <span className="font-medium text-foreground">重置所有資料與設定</span>
                </div>
                <p className="text-xs text-muted-foreground">
                  完全重置帳號，刪除所有資料並將設定恢復為預設值
                </p>
              </div>
              <button
                onClick={() => handleOpenDialog('factory_reset')}
                className="px-4 py-2 text-sm font-medium text-white bg-destructive hover:bg-destructive/90 rounded-md transition-colors"
              >
                完全重置
              </button>
            </div>
          </div>
        </div>
      </Card>

      {/* Confirmation Dialog */}
      <Dialog open={activeOperation !== null} onOpenChange={() => handleCloseDialog()}>
        <DialogContent className="sm:max-w-[480px]">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2 text-destructive">
              <AlertTriangle className="w-5 h-5" />
              {config?.title}
            </DialogTitle>
            <DialogDescription className="text-left pt-2">
              {config?.description}
            </DialogDescription>
          </DialogHeader>

          {/* Progress section for recompact */}
          {activeOperation === 'recompact' && loading && progress && (
            <div className="my-4 space-y-3">
              <div className="flex items-center justify-between text-sm">
                <span className="text-muted-foreground">
                  {phaseLabels[progress.phase]}
                </span>
                <span className="font-mono text-foreground">
                  {progress.current}/{progress.total}
                </span>
              </div>
              <Progress value={progressPercent} className="h-2" />
              <p className="text-xs text-muted-foreground truncate">
                {progress.message}
              </p>
            </div>
          )}

          {/* Warning section - hide when showing progress */}
          {!(activeOperation === 'recompact' && loading && progress) && (
            <div className="my-4 p-3 bg-destructive/10 border border-destructive/30 rounded-md">
              <p className="text-sm text-destructive font-medium">
                {config?.warning}
              </p>
            </div>
          )}

          {/* Confirm input - hide when showing progress */}
          {!(activeOperation === 'recompact' && loading && progress) && (
            <div className="space-y-2">
              <p className="text-sm text-muted-foreground">
                請輸入 <code className="px-1.5 py-0.5 bg-muted rounded text-foreground font-mono text-xs">{config?.confirmText}</code> 以確認操作：
              </p>
              <Input
                value={confirmInput}
                onChange={(e) => setConfirmInput(e.target.value)}
                placeholder={config?.confirmPlaceholder}
                className="font-mono"
                disabled={loading}
              />
            </div>
          )}

          <DialogFooter className="gap-2 sm:gap-0">
            <button
              onClick={handleCloseDialog}
              className="px-4 py-2 text-sm font-medium text-muted-foreground hover:text-foreground transition-colors"
            >
              {loading ? '背景執行' : '取消'}
            </button>
            <button
              onClick={handleExecute}
              disabled={!isConfirmValid || loading}
              className={`px-4 py-2 text-sm font-medium text-white rounded-md transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2 ${config?.buttonClass}`}
            >
              {loading && <Loader2 className="w-4 h-4 animate-spin" />}
              {config?.buttonText}
            </button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </section>
  )
}
