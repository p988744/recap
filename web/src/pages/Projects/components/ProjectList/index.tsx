import { useState } from 'react'
import { Search } from 'lucide-react'
import { Input } from '@/components/ui/input'
import { Checkbox } from '@/components/ui/checkbox'
import { ScrollArea } from '@/components/ui/scroll-area'
import { useProjects } from '../../hooks/useProjects'
import { ProjectCard } from './ProjectCard'

interface ProjectListProps {
  selectedProject: string | null
  onSelectProject: (projectName: string | null) => void
}

export function ProjectList({ selectedProject, onSelectProject }: ProjectListProps) {
  const [search, setSearch] = useState('')
  const [showHidden, setShowHidden] = useState(false)
  const { projects, isLoading } = useProjects({ showHidden })

  const filteredProjects = projects.filter(p =>
    p.project_name.toLowerCase().includes(search.toLowerCase()) ||
    (p.display_name?.toLowerCase().includes(search.toLowerCase()))
  )

  return (
    <div className="h-full flex flex-col bg-card rounded-lg border">
      {/* Search */}
      <div className="p-3 border-b">
        <div className="relative">
          <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="搜尋專案..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="pl-8 h-9"
          />
        </div>
      </div>

      {/* Project list */}
      <ScrollArea className="flex-1">
        <div className="p-2 space-y-1">
          {isLoading ? (
            <div className="p-4 flex justify-center">
              <div className="w-5 h-5 border border-border border-t-foreground/60 rounded-full animate-spin" />
            </div>
          ) : filteredProjects.length === 0 ? (
            <div className="p-4 text-center text-sm text-muted-foreground">
              {search ? '找不到符合的專案' : '尚無專案'}
            </div>
          ) : (
            filteredProjects.map((project) => (
              <ProjectCard
                key={project.project_name}
                project={project}
                isSelected={selectedProject === project.project_name}
                onClick={() => onSelectProject(project.project_name)}
              />
            ))
          )}
        </div>
      </ScrollArea>

      {/* Footer */}
      <div className="p-3 border-t">
        <div className="flex items-center gap-2">
          <Checkbox
            id="show-hidden"
            checked={showHidden}
            onCheckedChange={(checked) => setShowHidden(checked === true)}
          />
          <label
            htmlFor="show-hidden"
            className="text-xs text-muted-foreground cursor-pointer"
          >
            顯示隱藏專案
          </label>
        </div>
      </div>
    </div>
  )
}
