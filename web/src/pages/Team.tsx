import {
  Users,
  UserPlus,
} from 'lucide-react'
import { Card } from '@/components/ui/card'
import { Button } from '@/components/ui/button'

export function TeamPage() {
  return (
    <div className="space-y-12">
      {/* Header */}
      <header className="flex items-start justify-between animate-fade-up opacity-0 delay-1">
        <div>
          <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
            管理
          </p>
          <h1 className="font-display text-4xl text-foreground tracking-tight">團隊管理</h1>
        </div>
        <Button variant="outline" disabled>
          <UserPlus className="w-4 h-4" strokeWidth={1.5} />
          新增團隊
        </Button>
      </header>

      {/* Coming Soon State */}
      <section className="animate-fade-up opacity-0 delay-2">
        <Card className="p-16">
          <div className="text-center">
            <Users className="w-12 h-12 mx-auto mb-4 text-charcoal/20" strokeWidth={1} />
            <h3 className="font-display text-xl text-foreground mb-2">團隊功能即將推出</h3>
            <p className="text-sm text-muted-foreground">
              團隊管理功能正在開發中，敬請期待
            </p>
          </div>
        </Card>
      </section>
    </div>
  )
}
