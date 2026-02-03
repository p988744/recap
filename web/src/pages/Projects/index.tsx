import { useState, useEffect, useCallback } from 'react'
import { useSearchParams } from 'react-router-dom'
import { FolderKanban } from 'lucide-react'
import { ProjectList } from './components/ProjectList'
import { ProjectDetail } from './components/ProjectDetail'
import { useProjects } from './hooks/useProjects'

// Re-export detail pages
export { TimelinePeriodDetailPage } from './TimelinePeriodDetailPage'

export function ProjectsPage() {
  const [searchParams, setSearchParams] = useSearchParams()
  const [showHidden, setShowHidden] = useState(false)
  const { projects, isLoading } = useProjects({ showHidden })

  // Get selected project from URL or default to first project
  const selectedProject = searchParams.get('project')

  // Update URL when selecting a project
  const setSelectedProject = useCallback((projectName: string | null) => {
    if (projectName) {
      setSearchParams({ project: projectName })
    } else {
      setSearchParams({})
    }
  }, [setSearchParams])

  // Auto-select first project when loaded (only if no project in URL)
  useEffect(() => {
    if (!isLoading && projects.length > 0 && !selectedProject) {
      setSelectedProject(projects[0].project_name)
    }
  }, [isLoading, projects, selectedProject, setSelectedProject])

  return (
    <div className="h-[calc(100vh-5rem)] flex flex-col">
      {/* Header */}
      <header className="flex-shrink-0 pb-6 animate-fade-up opacity-0 delay-1">
        <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
          Projects
        </p>
        <h1 className="font-display text-4xl text-foreground tracking-tight">
          專案
        </h1>
      </header>

      {/* Main content - left/right split with independent scrolling */}
      <div className="flex-1 flex gap-6 min-h-0 animate-fade-up opacity-0 delay-2">
        {/* Left panel - Project list (independent scroll) */}
        <div className="w-64 flex-shrink-0">
          <ProjectList
            projects={projects}
            isLoading={isLoading}
            selectedProject={selectedProject}
            onSelectProject={setSelectedProject}
            showHidden={showHidden}
            onShowHiddenChange={setShowHidden}
          />
        </div>

        {/* Right panel - Project detail (independent scroll) */}
        <div className="flex-1 min-w-0 overflow-y-auto">
          {selectedProject ? (
            <ProjectDetail projectName={selectedProject} />
          ) : (
            <div className="h-full flex flex-col items-center justify-center text-center bg-card rounded-lg border">
              <div className="w-16 h-16 rounded-full bg-muted/50 flex items-center justify-center mb-6">
                <FolderKanban className="w-8 h-8 text-muted-foreground" strokeWidth={1.5} />
              </div>
              <h2 className="text-lg font-medium text-foreground mb-2">選擇專案</h2>
              <p className="text-sm text-muted-foreground max-w-md">
                從左側列表選擇一個專案，查看詳細資訊和時間軸。
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
