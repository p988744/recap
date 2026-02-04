import { useState } from 'react'
import { GitBranch } from 'lucide-react'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Card } from '@/components/ui/card'
import { cn } from '@/lib/utils'
import { InfoTab } from './InfoTab'
import { TimelineTab } from './TimelineTab'
import { useProjectDetail } from '../../hooks/useProjectDetail'
import { ClaudeIcon } from '@/pages/Settings/components/ProjectsSection/icons/ClaudeIcon'
import { GeminiIcon } from '@/pages/Settings/components/ProjectsSection/icons/GeminiIcon'

const SOURCE_CONFIG: Record<string, { icon: React.ReactNode; label: string; className: string }> = {
  claude_code: {
    icon: <ClaudeIcon className="w-3 h-3" />,
    label: 'Claude',
    className: 'bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400',
  },
  antigravity: {
    icon: <GeminiIcon className="w-3 h-3" />,
    label: 'Antigravity',
    className: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',
  },
  git: {
    icon: <GitBranch className="w-3 h-3" />,
    label: 'Git',
    className: 'bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-400',
  },
  gitlab: {
    icon: <GitBranch className="w-3 h-3" />,
    label: 'GitLab',
    className: 'bg-orange-100 text-orange-700 dark:bg-orange-900/30 dark:text-orange-400',
  },
  manual: {
    icon: null,
    label: '手動',
    className: 'bg-foreground/5 text-muted-foreground/60',
  },
}

interface ProjectDetailProps {
  projectName: string
}

export function ProjectDetail({ projectName }: ProjectDetailProps) {
  const [activeTab, setActiveTab] = useState('info')
  const { detail, isLoading, error, refetch } = useProjectDetail(projectName)

  if (isLoading) {
    return (
      <Card className="flex items-center justify-center py-16">
        <div className="w-6 h-6 border border-border border-t-foreground/60 rounded-full animate-spin" />
      </Card>
    )
  }

  if (error || !detail) {
    return (
      <Card className="flex items-center justify-center py-16">
        <span className="text-destructive">{error || '無法載入專案'}</span>
      </Card>
    )
  }

  const displayName = detail.display_name || detail.project_name

  // Get unique sources (excluding 'aggregated')
  const sources = detail.sources
    .map(s => s.source)
    .filter(s => s !== 'aggregated')

  return (
    <Card>
      {/* Header */}
      <div className="p-4 border-b">
        <div className="flex items-start justify-between gap-4">
          <div className="min-w-0">
            <h2 className="text-xl font-semibold">{displayName}</h2>
            {detail.project_path && (
              <p className="text-xs text-muted-foreground mt-1 font-mono truncate">
                {detail.project_path}
              </p>
            )}
          </div>
          {/* Source badges */}
          <div className="flex flex-wrap gap-1.5 flex-shrink-0">
            {sources.map((source) => {
              const config = SOURCE_CONFIG[source]
              if (!config) return null
              return (
                <span
                  key={source}
                  className={cn(
                    'inline-flex items-center gap-1 text-xs px-2 py-1 rounded',
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

      {/* Tabs */}
      <Tabs value={activeTab} onValueChange={setActiveTab}>
        <TabsList className="mx-4 mt-2 w-fit">
          <TabsTrigger value="info">專案資訊</TabsTrigger>
          <TabsTrigger value="timeline">時間軸</TabsTrigger>
        </TabsList>

        <TabsContent value="info" className="m-0 p-4">
          <InfoTab projectName={projectName} detail={detail} onUpdate={refetch} />
        </TabsContent>
        <TabsContent value="timeline" className="m-0 p-4">
          <TimelineTab projectName={projectName} />
        </TabsContent>
      </Tabs>
    </Card>
  )
}
