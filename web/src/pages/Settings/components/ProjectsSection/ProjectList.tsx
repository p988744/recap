import { Eye, EyeOff, ChevronRight, Trash2, GitBranch } from 'lucide-react'
import { Card } from '@/components/ui/card'
import type { ProjectInfo } from '@/types'
import { ClaudeIcon, GeminiIcon } from '@/components/icons'

const SOURCE_CONFIG: Record<string, { icon: React.ReactNode; label: string; badgeBgClass: string }> = {
  claude_code: {
    icon: <ClaudeIcon className="w-2.5 h-2.5" />,
    label: 'Claude Code',
    badgeBgClass: 'bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400',
  },
  antigravity: {
    icon: <GeminiIcon className="w-2.5 h-2.5" />,
    label: 'Antigravity',
    badgeBgClass: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',
  },
  git: {
    icon: <GitBranch className="w-2.5 h-2.5" />,
    label: 'Git',
    badgeBgClass: 'bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-400',
  },
  gitlab: {
    icon: <GitBranch className="w-2.5 h-2.5" />,
    label: 'GitLab',
    badgeBgClass: 'bg-orange-100 text-orange-700 dark:bg-orange-900/30 dark:text-orange-400',
  },
  manual: {
    icon: null,
    label: '手動',
    badgeBgClass: 'bg-foreground/5 text-muted-foreground/60',
  },
}

function SourceBadge({ source }: { source: string }) {
  const config = SOURCE_CONFIG[source]
  if (!config) return null
  return (
    <span className={`inline-flex items-center gap-0.5 text-[10px] px-1.5 py-0.5 rounded ${config.badgeBgClass}`}>
      {config.icon}
      {config.label}
    </span>
  )
}

interface ProjectListProps {
  projects: ProjectInfo[]
  onSelect: (projectName: string) => void
  onToggleVisibility: (projectName: string, hidden: boolean) => void
  onRemove: (projectName: string) => void
}

export function ProjectList({ projects, onSelect, onToggleVisibility, onRemove }: ProjectListProps) {
  if (projects.length === 0) {
    return (
      <Card className="p-8 text-center">
        <p className="text-sm text-muted-foreground">
          尚未發現任何專案。同步工作記錄後，專案會自動顯示在這裡，或點擊「新增專案」手動新增。
        </p>
      </Card>
    )
  }

  return (
    <div className="space-y-1">
      {projects.map((project) => {
        const isManual = project.source === 'manual' && project.work_item_count === 0
        return (
          <div
            key={project.project_name}
            className={`group flex items-center gap-3 px-3 py-2.5 rounded-md transition-colors cursor-pointer
              ${project.hidden
                ? 'opacity-50 hover:opacity-70'
                : 'hover:bg-foreground/5'
              }`}
            onClick={() => onSelect(project.project_name)}
          >
            {/* Project name + source badges */}
            <div className="flex-1 min-w-0 flex items-center gap-2">
              <span className={`text-sm truncate ${project.hidden ? 'line-through text-muted-foreground' : 'text-foreground'}`}>
                {project.display_name || project.project_name}
              </span>
              <div className="flex items-center gap-1">
                {(project.sources || [project.source]).map((source) => (
                  <SourceBadge key={source} source={source} />
                ))}
              </div>
            </div>

            {/* Remove button (only for manual projects) */}
            {isManual && (
              <button
                onClick={(e) => {
                  e.stopPropagation()
                  onRemove(project.project_name)
                }}
                className="p-1 rounded hover:bg-red-100 dark:hover:bg-red-900/20 transition-colors opacity-0 group-hover:opacity-100"
                title="移除專案"
              >
                <Trash2 className="w-3.5 h-3.5 text-red-400" />
              </button>
            )}

            {/* Toggle visibility */}
            <button
              onClick={(e) => {
                e.stopPropagation()
                onToggleVisibility(project.project_name, !project.hidden)
              }}
              className="p-1 rounded hover:bg-foreground/10 transition-colors opacity-0 group-hover:opacity-100"
              title={project.hidden ? '顯示專案' : '隱藏專案'}
            >
              {project.hidden ? (
                <EyeOff className="w-3.5 h-3.5 text-muted-foreground" />
              ) : (
                <Eye className="w-3.5 h-3.5 text-muted-foreground" />
              )}
            </button>

            {/* Chevron */}
            <ChevronRight className="w-3.5 h-3.5 text-muted-foreground/50 shrink-0" />
          </div>
        )
      })}
    </div>
  )
}
