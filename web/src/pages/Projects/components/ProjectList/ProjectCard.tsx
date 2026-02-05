import { Folder, Clock, EyeOff, GitBranch } from 'lucide-react'
import { cn } from '@/lib/utils'
import type { ProjectInfo } from '@/types'
import { ClaudeIcon, GeminiIcon } from '@/components/icons'

interface ProjectCardProps {
  project: ProjectInfo
  isSelected: boolean
  onClick: () => void
}

const SOURCE_CONFIG: Record<string, { icon: React.ReactNode; label: string; className: string }> = {
  claude_code: {
    icon: <ClaudeIcon className="w-2.5 h-2.5" />,
    label: 'Claude',
    className: 'bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400',
  },
  antigravity: {
    icon: <GeminiIcon className="w-2.5 h-2.5" />,
    label: 'Antigravity',
    className: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',
  },
  git: {
    icon: <GitBranch className="w-2.5 h-2.5" />,
    label: 'Git',
    className: 'bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-400',
  },
  gitlab: {
    icon: <GitBranch className="w-2.5 h-2.5" />,
    label: 'GitLab',
    className: 'bg-orange-100 text-orange-700 dark:bg-orange-900/30 dark:text-orange-400',
  },
  manual: {
    icon: null,
    label: '手動',
    className: 'bg-foreground/5 text-muted-foreground/60',
  },
}

export function ProjectCard({ project, isSelected, onClick }: ProjectCardProps) {
  const displayName = project.display_name || project.project_name

  return (
    <button
      onClick={onClick}
      className={cn(
        'w-full text-left p-3 rounded-md transition-colors',
        'hover:bg-muted/50',
        isSelected && 'bg-muted',
        project.hidden && 'opacity-60'
      )}
    >
      <div className="flex items-start gap-2">
        <Folder className="w-4 h-4 mt-0.5 text-muted-foreground flex-shrink-0" />
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-1.5">
            <span className="font-medium text-sm truncate">{displayName}</span>
            {project.hidden && (
              <EyeOff className="w-3 h-3 text-muted-foreground flex-shrink-0" />
            )}
          </div>

          <div className="flex items-center gap-2 mt-1 text-xs text-muted-foreground">
            <span className="flex items-center gap-1">
              <Clock className="w-3 h-3" />
              {project.total_hours.toFixed(1)}h
            </span>
            <span>·</span>
            <span>{project.work_item_count} 項目</span>
          </div>

          <div className="flex flex-wrap gap-1 mt-2">
            {project.sources
              .filter((source) => source !== 'aggregated')
              .map((source) => {
                const config = SOURCE_CONFIG[source]
                if (!config) return null
                return (
                  <span
                    key={source}
                    className={cn(
                      'inline-flex items-center gap-0.5 text-[10px] px-1.5 py-0.5 rounded',
                      config.className
                    )}
                  >
                    {config.icon}
                    {config.label}
                  </span>
                )
              })}
          </div>
        </div>
      </div>
    </button>
  )
}
