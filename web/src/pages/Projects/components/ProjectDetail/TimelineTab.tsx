import { Construction } from 'lucide-react'

interface TimelineTabProps {
  projectName: string
}

export function TimelineTab({ projectName: _projectName }: TimelineTabProps) {
  return (
    <div className="h-full flex flex-col items-center justify-center text-center">
      <Construction className="w-12 h-12 text-muted-foreground mb-4" />
      <h3 className="text-lg font-medium mb-2">時間軸功能開發中</h3>
      <p className="text-sm text-muted-foreground max-w-md">
        此功能將在後續版本中推出，敬請期待。
      </p>
    </div>
  )
}
