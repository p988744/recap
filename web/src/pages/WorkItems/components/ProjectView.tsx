import { Briefcase, Info } from 'lucide-react'
import { Card, CardContent } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { ProjectSummaryCard } from '@/components/ProjectSummaryCard'
import type { WorkItem } from '@/types'
import type { ProjectGroup } from '../hooks/types'

interface ProjectViewProps {
  projectGroups: ProjectGroup[]
  items: WorkItem[]
  onItemClick: (item: WorkItem) => void
  onProjectDetail?: (projectName: string) => void
}

export function ProjectView({ projectGroups, items, onItemClick, onProjectDetail }: ProjectViewProps) {
  return (
    <>
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-2">
          <Briefcase className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
          <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
            依專案分組
          </p>
          <Badge variant="secondary" className="ml-2">{projectGroups.length} 專案</Badge>
        </div>
      </div>

      <div className="space-y-4">
        {projectGroups.length === 0 ? (
          <Card>
            <CardContent className="p-16">
              <div className="text-center">
                <Briefcase className="w-12 h-12 mx-auto mb-4 text-muted-foreground/30" strokeWidth={1} />
                <h3 className="font-display text-xl text-foreground mb-2">尚無專案資料</h3>
                <p className="text-sm text-muted-foreground">
                  同步工作紀錄後即可檢視專案分組
                </p>
              </div>
            </CardContent>
          </Card>
        ) : (
          projectGroups.map((project) => (
            <ProjectSummaryCard
              key={project.project_name}
              project={project}
              onItemClick={(item) => {
                const workItem = items.find(i => i.id === item.id) ||
                  { id: item.id, title: item.title, description: item.description, hours: item.hours, date: item.date, source: item.source, synced_to_tempo: item.synced_to_tempo } as WorkItem
                onItemClick(workItem)
              }}
              headerAction={onProjectDetail ? (
                <button
                  onClick={(e) => {
                    e.stopPropagation()
                    onProjectDetail(project.project_name)
                  }}
                  className="p-1.5 rounded hover:bg-foreground/10 transition-colors"
                  title="查看專案詳情"
                >
                  <Info className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
                </button>
              ) : undefined}
            />
          ))
        )}
      </div>
    </>
  )
}
