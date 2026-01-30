import { useState, useEffect } from 'react'
import { getVersion } from '@tauri-apps/api/app'
import { CheckCircle2 } from 'lucide-react'
import { Card } from '@/components/ui/card'

export function AboutSection() {
  const [version, setVersion] = useState('')

  useEffect(() => {
    getVersion().then(setVersion)
  }, [])

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
