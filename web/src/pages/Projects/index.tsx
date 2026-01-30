import { useState } from 'react'
import { FolderKanban } from 'lucide-react'
import { ProjectList } from './components/ProjectList'
import { ProjectDetail } from './components/ProjectDetail'

export function ProjectsPage() {
  const [selectedProject, setSelectedProject] = useState<string | null>(null)

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <header className="flex-shrink-0 pb-6 animate-fade-up opacity-0 delay-1">
        <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
          Projects
        </p>
        <h1 className="font-display text-4xl text-foreground tracking-tight">
          專案
        </h1>
      </header>

      {/* Main content - left/right split */}
      <div className="flex-1 flex gap-6 min-h-0 animate-fade-up opacity-0 delay-2">
        {/* Left panel - Project list */}
        <div className="w-64 flex-shrink-0 flex flex-col min-h-0">
          <ProjectList
            selectedProject={selectedProject}
            onSelectProject={setSelectedProject}
          />
        </div>

        {/* Right panel - Project detail */}
        <div className="flex-1 min-w-0">
          {selectedProject ? (
            <ProjectDetail projectName={selectedProject} />
          ) : (
            <div className="h-full flex flex-col items-center justify-center text-center">
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
