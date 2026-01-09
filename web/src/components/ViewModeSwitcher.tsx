import { List, FolderKanban, Tags, Clock } from 'lucide-react'
import { cn } from '@/lib/utils'

export type ViewMode = 'list' | 'project' | 'task' | 'timeline'

interface ViewModeSwitcherProps {
  value: ViewMode
  onChange: (mode: ViewMode) => void
  className?: string
}

const viewModes: { id: ViewMode; label: string; icon: typeof List }[] = [
  { id: 'list', label: '列表', icon: List },
  { id: 'project', label: '專案', icon: FolderKanban },
  { id: 'task', label: '任務', icon: Tags },
  { id: 'timeline', label: '時間軸', icon: Clock },
]

export function ViewModeSwitcher({ value, onChange, className }: ViewModeSwitcherProps) {
  return (
    <div className={cn('inline-flex items-center gap-1 p-1 bg-muted/50 rounded-lg', className)}>
      {viewModes.map((mode) => {
        const Icon = mode.icon
        const isActive = value === mode.id
        return (
          <button
            key={mode.id}
            onClick={() => onChange(mode.id)}
            className={cn(
              'flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium transition-all',
              isActive
                ? 'bg-background text-foreground shadow-sm'
                : 'text-muted-foreground hover:text-foreground hover:bg-background/50'
            )}
          >
            <Icon className="w-3.5 h-3.5" strokeWidth={1.5} />
            <span>{mode.label}</span>
          </button>
        )
      })}
    </div>
  )
}
