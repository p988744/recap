import { Folder, Clock, EyeOff } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Badge } from '@/components/ui/badge'
import type { ProjectInfo } from '@/types'

interface ProjectCardProps {
  project: ProjectInfo
  isSelected: boolean
  onClick: () => void
}

const SOURCE_COLORS: Record<string, string> = {
  claude_code: 'bg-orange-500/10 text-orange-600 border-orange-500/20',
  antigravity: 'bg-purple-500/10 text-purple-600 border-purple-500/20',
  git: 'bg-green-500/10 text-green-600 border-green-500/20',
  gitlab: 'bg-blue-500/10 text-blue-600 border-blue-500/20',
  manual: 'bg-gray-500/10 text-gray-600 border-gray-500/20',
}

const SOURCE_LABELS: Record<string, string> = {
  claude_code: 'Claude',
  antigravity: 'Gemini',
  git: 'Git',
  gitlab: 'GitLab',
  manual: '手動',
}

export function ProjectCard({ project, isSelected, onClick }: ProjectCardProps) {
  const displayName = project.display_name || project.project_name

  return (
    <button
      onClick={onClick}
      className={cn(
        'w-full text-left p-3 rounded-md transition-colors',
        'hover:bg-accent',
        isSelected && 'bg-accent',
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
            {project.sources.map((source) => (
              <Badge
                key={source}
                variant="outline"
                className={cn('text-[10px] px-1.5 py-0', SOURCE_COLORS[source])}
              >
                {SOURCE_LABELS[source] || source}
              </Badge>
            ))}
          </div>
        </div>
      </div>
    </button>
  )
}
