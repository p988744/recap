import { BarChart3 } from 'lucide-react'

export function ReviewPage() {
  return (
    <div className="space-y-12">
      <header className="animate-fade-up opacity-0 delay-1">
        <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
          Review
        </p>
        <h1 className="font-display text-4xl text-foreground tracking-tight">
          回顧
        </h1>
      </header>

      <div className="flex flex-col items-center justify-center py-24 text-center animate-fade-up opacity-0 delay-2">
        <div className="w-16 h-16 rounded-full bg-muted/50 flex items-center justify-center mb-6">
          <BarChart3 className="w-8 h-8 text-muted-foreground" strokeWidth={1.5} />
        </div>
        <h2 className="text-lg font-medium text-foreground mb-2">即將推出</h2>
        <p className="text-sm text-muted-foreground max-w-md">
          工作回顧與報表功能正在開發中，敬請期待。
        </p>
      </div>
    </div>
  )
}
