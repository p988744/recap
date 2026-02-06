import { useState, useEffect } from 'react'
import { getVersion } from '@tauri-apps/api/app'
import { CheckCircle2, ArrowUpCircle, Loader2, AlertCircle, RotateCcw } from 'lucide-react'
import { Card } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Progress } from '@/components/ui/progress'
import { useUpdateChecker } from '@/hooks/useUpdateChecker'

export function AboutSection() {
  const [version, setVersion] = useState('')
  const update = useUpdateChecker()

  useEffect(() => {
    getVersion().then(setVersion)
  }, [])

  const downloadPercent =
    update.progress?.contentLength && update.progress.contentLength > 0
      ? Math.round((update.progress.downloaded / update.progress.contentLength) * 100)
      : null

  return (
    <section className="animate-fade-up opacity-0 delay-1">
      <h2 className="font-display text-2xl text-foreground mb-6">關於</h2>

      <Card className="p-6">
        <div className="space-y-6">
          <div className="flex items-center gap-4">
            <div className="w-16 h-16 rounded-2xl bg-[#1F1D1A] flex flex-col items-center justify-center p-2">
              <span className="text-[#F9F7F2] text-sm font-display font-medium tracking-tight">Recap</span>
              <div className="w-10 h-0.5 bg-[#B09872] mt-0.5 rounded-full opacity-70" />
            </div>
            <div>
              <h3 className="font-display text-xl text-foreground">Recap</h3>
              <p className="text-sm text-muted-foreground">{version ? `v${version}` : ''}</p>
            </div>
          </div>

          <div className="pt-4 border-t border-border">
            <p className="text-sm text-foreground mb-2">自動回顧你的工作</p>
            <p className="text-xs text-muted-foreground leading-relaxed">
              Recap 自動追蹤您從 Git、Claude Code 等來源的工作記錄，
              協助您回顧與管理每日工作成果。
            </p>
          </div>

          {/* Update checker section */}
          <div className="pt-4 border-t border-border space-y-3">
            {update.status === 'idle' && (
              <Button variant="outline" size="sm" onClick={update.checkForUpdate}>
                檢查更新
              </Button>
            )}

            {update.status === 'checking' && (
              <Button variant="outline" size="sm" disabled>
                <Loader2 className="w-3.5 h-3.5 mr-2 animate-spin" />
                檢查中...
              </Button>
            )}

            {update.status === 'up-to-date' && (
              <div className="space-y-2">
                <div className="flex items-center gap-2 text-sm text-sage">
                  <CheckCircle2 className="w-4 h-4" />
                  <span>已是最新版本</span>
                </div>
                {update.lastCheckedAt && (
                  <p className="text-xs text-muted-foreground">
                    上次檢查：{new Date(update.lastCheckedAt).toLocaleString('zh-TW')}
                  </p>
                )}
                <Button variant="outline" size="sm" onClick={update.checkForUpdate}>
                  檢查更新
                </Button>
              </div>
            )}

            {update.status === 'available' && (
              <div className="space-y-3">
                <div className="flex items-center gap-2 text-sm text-foreground">
                  <ArrowUpCircle className="w-4 h-4 text-sage" />
                  <span>有新版本 v{update.version}</span>
                </div>
                {update.notes && (
                  <p className="text-xs text-muted-foreground leading-relaxed line-clamp-4">
                    {update.notes}
                  </p>
                )}
                <Button size="sm" onClick={update.downloadAndInstall}>
                  下載並安裝
                </Button>
              </div>
            )}

            {update.status === 'downloading' && (
              <div className="space-y-2">
                <div className="flex items-center gap-2 text-sm text-muted-foreground">
                  <Loader2 className="w-4 h-4 animate-spin" />
                  <span>下載中...{downloadPercent !== null ? ` ${downloadPercent}%` : ''}</span>
                </div>
                <Progress value={downloadPercent ?? undefined} className="h-1.5" />
              </div>
            )}

            {update.status === 'ready' && (
              <div className="space-y-2">
                <div className="flex items-center gap-2 text-sm text-sage">
                  <CheckCircle2 className="w-4 h-4" />
                  <span>更新已準備就緒</span>
                </div>
                <Button size="sm" onClick={update.relaunchApp}>
                  <RotateCcw className="w-3.5 h-3.5 mr-2" />
                  重新啟動
                </Button>
              </div>
            )}

            {update.status === 'error' && (
              <div className="space-y-2">
                <div className="flex items-center gap-2 text-sm text-red-500">
                  <AlertCircle className="w-4 h-4" />
                  <span className="line-clamp-2">{update.error}</span>
                </div>
                <Button variant="outline" size="sm" onClick={update.checkForUpdate}>
                  重試
                </Button>
              </div>
            )}
          </div>

          <div className="pt-4 border-t border-border space-y-2">
            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground">API 狀態</span>
              <span className="flex items-center gap-1.5 text-sage">
                <CheckCircle2 className="w-3.5 h-3.5" />
                運行中
              </span>
            </div>
            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground">資料庫</span>
              <span className="text-foreground">SQLite</span>
            </div>
            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground">框架</span>
              <span className="text-foreground">Tauri v2</span>
            </div>
          </div>
        </div>
      </Card>
    </section>
  )
}
