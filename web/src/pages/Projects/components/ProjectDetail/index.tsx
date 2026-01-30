import { useState } from 'react'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Card } from '@/components/ui/card'
import { InfoTab } from './InfoTab'
import { TimelineTab } from './TimelineTab'
import { SettingsTab } from './SettingsTab'
import { useProjectDetail } from '../../hooks/useProjectDetail'

interface ProjectDetailProps {
  projectName: string
}

export function ProjectDetail({ projectName }: ProjectDetailProps) {
  const [activeTab, setActiveTab] = useState('info')
  const { detail, isLoading, error, refetch } = useProjectDetail(projectName)

  if (isLoading) {
    return (
      <Card className="h-full flex items-center justify-center">
        <div className="w-6 h-6 border border-border border-t-foreground/60 rounded-full animate-spin" />
      </Card>
    )
  }

  if (error || !detail) {
    return (
      <Card className="h-full flex items-center justify-center">
        <span className="text-destructive">{error || '無法載入專案'}</span>
      </Card>
    )
  }

  const displayName = detail.display_name || detail.project_name

  return (
    <Card className="h-full flex flex-col">
      {/* Header */}
      <div className="p-4 border-b">
        <h2 className="text-xl font-semibold">{displayName}</h2>
        {detail.project_path && (
          <p className="text-xs text-muted-foreground mt-1 font-mono truncate">
            {detail.project_path}
          </p>
        )}
      </div>

      {/* Tabs */}
      <Tabs value={activeTab} onValueChange={setActiveTab} className="flex-1 flex flex-col min-h-0">
        <TabsList className="mx-4 mt-2 w-fit">
          <TabsTrigger value="info">專案資訊</TabsTrigger>
          <TabsTrigger value="timeline">時間軸</TabsTrigger>
          <TabsTrigger value="settings">設定</TabsTrigger>
        </TabsList>

        <div className="flex-1 min-h-0 overflow-hidden">
          <TabsContent value="info" className="h-full m-0 p-4 overflow-auto">
            <InfoTab projectName={projectName} detail={detail} onUpdate={refetch} />
          </TabsContent>
          <TabsContent value="timeline" className="h-full m-0 p-4 overflow-auto">
            <TimelineTab projectName={projectName} />
          </TabsContent>
          <TabsContent value="settings" className="h-full m-0 p-4 overflow-auto">
            <SettingsTab projectName={projectName} detail={detail} onUpdate={refetch} />
          </TabsContent>
        </div>
      </Tabs>
    </Card>
  )
}
