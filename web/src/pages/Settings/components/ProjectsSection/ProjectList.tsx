import { Eye, EyeOff, ChevronRight, Trash2 } from 'lucide-react'
import { Card } from '@/components/ui/card'
import type { ProjectInfo } from '@/types'

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
            {/* Project name */}
            <div className="flex-1 min-w-0">
              <span className={`text-sm ${project.hidden ? 'line-through text-muted-foreground' : 'text-foreground'}`}>
                {project.display_name || project.project_name}
              </span>
              {isManual && (
                <span className="ml-2 text-[10px] text-muted-foreground/60 bg-foreground/5 px-1.5 py-0.5 rounded">
                  手動新增
                </span>
              )}
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
