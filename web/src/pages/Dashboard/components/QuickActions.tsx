import { Link } from 'react-router-dom'
import {
  Upload,
  FileText,
  TrendingUp,
  Loader2,
  Check,
  AlertCircle,
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Separator } from '@/components/ui/separator'
import { cn } from '@/lib/utils'
import type { SyncState } from '../hooks/useDashboard'

interface QuickActionsProps {
  syncStatus: SyncState
  syncMessage: string
  onSyncToTempo: () => void
}

export function QuickActions({ syncStatus, syncMessage, onSyncToTempo }: QuickActionsProps) {
  return (
    <section className="pt-8 animate-fade-up opacity-0 delay-6">
      <Separator className="mb-8" />
      <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-4">
        快捷操作
      </p>
      <div className="flex items-center gap-3">
        <Button
          variant="outline"
          onClick={onSyncToTempo}
          disabled={syncStatus === 'syncing'}
          className={cn(
            syncStatus === 'success' && 'border-sage text-sage',
            syncStatus === 'error' && 'border-destructive text-destructive'
          )}
        >
          {syncStatus === 'syncing' ? (
            <Loader2 className="w-4 h-4 mr-2 animate-spin" strokeWidth={1.5} />
          ) : syncStatus === 'success' ? (
            <Check className="w-4 h-4 mr-2" strokeWidth={1.5} />
          ) : syncStatus === 'error' ? (
            <AlertCircle className="w-4 h-4 mr-2" strokeWidth={1.5} />
          ) : (
            <Upload className="w-4 h-4 mr-2" strokeWidth={1.5} />
          )}
          {syncStatus === 'syncing' ? '同步中...' : syncMessage || '同步到 Tempo'}
        </Button>
        <Link to="/reports">
          <Button variant="ghost">
            <FileText className="w-4 h-4 mr-2" strokeWidth={1.5} />
            生成週報
          </Button>
        </Link>
        <Link to="/reports?tab=pe">
          <Button variant="ghost">
            <TrendingUp className="w-4 h-4 mr-2" strokeWidth={1.5} />
            績效考核
          </Button>
        </Link>
      </div>
    </section>
  )
}
