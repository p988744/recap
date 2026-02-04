import { Calendar, Filter, Sparkles, Bot } from 'lucide-react'
import { Card, CardContent } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { Checkbox } from '@/components/ui/checkbox'
import { Label } from '@/components/ui/label'
import { WorkGanttChart } from '@/components/WorkGanttChart'
import type { TimelineSession } from '../hooks/types'

// Source configuration
const TIMELINE_SOURCES = [
  { id: 'claude_code', label: 'Claude Code', icon: Bot },
  { id: 'antigravity', label: 'Antigravity', icon: Sparkles },
] as const

interface TimelineViewProps {
  sessions: TimelineSession[]
  date: string
  onDateChange: (date: string) => void
  sources: string[]
  onSourcesChange: (sources: string[]) => void
}

export function TimelineView({ sessions, date, onDateChange, sources, onSourcesChange }: TimelineViewProps) {
  const handleSourceToggle = (sourceId: string, checked: boolean) => {
    if (checked) {
      onSourcesChange([...sources, sourceId])
    } else {
      // Prevent unchecking all sources
      if (sources.length > 1) {
        onSourcesChange(sources.filter(s => s !== sourceId))
      }
    }
  }

  const activeSourcesLabel = sources.length === TIMELINE_SOURCES.length
    ? '全部來源'
    : TIMELINE_SOURCES.filter(s => sources.includes(s.id)).map(s => s.label).join(', ')

  return (
    <>
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-2">
          <Calendar className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
          <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
            時間軸檢視
          </p>
        </div>

        {/* Source Filter */}
        <Popover>
          <PopoverTrigger asChild>
            <Button variant="outline" size="sm" className="h-8 gap-2">
              <Filter className="h-3.5 w-3.5" />
              <span className="text-xs">{activeSourcesLabel}</span>
            </Button>
          </PopoverTrigger>
          <PopoverContent className="w-48 p-3" align="end">
            <div className="space-y-3">
              <p className="text-xs font-medium text-muted-foreground">資料來源</p>
              <div className="space-y-2">
                {TIMELINE_SOURCES.map(source => {
                  const Icon = source.icon
                  const isChecked = sources.includes(source.id)
                  const isDisabled = isChecked && sources.length === 1

                  return (
                    <div key={source.id} className="flex items-center space-x-2">
                      <Checkbox
                        id={`source-${source.id}`}
                        checked={isChecked}
                        disabled={isDisabled}
                        onCheckedChange={(checked) => handleSourceToggle(source.id, checked === true)}
                      />
                      <Label
                        htmlFor={`source-${source.id}`}
                        className="flex items-center gap-2 text-sm font-normal cursor-pointer"
                      >
                        <Icon className="h-3.5 w-3.5 text-muted-foreground" />
                        {source.label}
                      </Label>
                    </div>
                  )
                })}
              </div>
            </div>
          </PopoverContent>
        </Popover>
      </div>

      <Card>
        <CardContent className="p-6">
          <WorkGanttChart
            sessions={sessions}
            date={date}
            onDateChange={onDateChange}
          />
        </CardContent>
      </Card>
    </>
  )
}
