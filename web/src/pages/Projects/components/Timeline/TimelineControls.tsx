import { Calendar, ChevronDown, Filter } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Checkbox } from '@/components/ui/checkbox'
import { Label } from '@/components/ui/label'
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover'
import type { TimeUnit } from '@/types'
import { ClaudeIcon } from '@/pages/Settings/components/ProjectsSection/icons/ClaudeIcon'
import { GeminiIcon } from '@/pages/Settings/components/ProjectsSection/icons/GeminiIcon'

interface TimelineControlsProps {
  timeUnit: TimeUnit
  onTimeUnitChange: (unit: TimeUnit) => void
  sources: string[]
  onSourcesChange: (sources: string[]) => void
  availableSources: string[]
}

const TIME_UNIT_LABELS: Record<TimeUnit, string> = {
  day: 'Day',
  week: 'Week',
  month: 'Month',
  quarter: 'Quarter',
  year: 'Year',
}

const SOURCE_CONFIG: Record<string, { icon: React.ReactNode; label: string }> = {
  claude_code: {
    icon: <ClaudeIcon className="w-4 h-4" />,
    label: 'Claude Code',
  },
  antigravity: {
    icon: <GeminiIcon className="w-4 h-4" />,
    label: 'Antigravity',
  },
}

export function TimelineControls({
  timeUnit,
  onTimeUnitChange,
  sources,
  onSourcesChange,
  availableSources,
}: TimelineControlsProps) {
  const handleSourceToggle = (source: string, checked: boolean) => {
    if (checked) {
      onSourcesChange([...sources, source])
    } else {
      onSourcesChange(sources.filter((s) => s !== source))
    }
  }

  return (
    <div className="flex items-center gap-2">
      {/* Time unit selector */}
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="outline" size="sm" className="h-8 gap-1.5">
            <Calendar className="w-3.5 h-3.5" />
            {TIME_UNIT_LABELS[timeUnit]}
            <ChevronDown className="w-3.5 h-3.5 opacity-60" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start">
          <DropdownMenuRadioGroup
            value={timeUnit}
            onValueChange={(value) => onTimeUnitChange(value as TimeUnit)}
          >
            {Object.entries(TIME_UNIT_LABELS).map(([value, label]) => (
              <DropdownMenuRadioItem key={value} value={value}>
                {label}
              </DropdownMenuRadioItem>
            ))}
          </DropdownMenuRadioGroup>
        </DropdownMenuContent>
      </DropdownMenu>

      {/* Source filter */}
      {availableSources.length > 1 && (
        <Popover>
          <PopoverTrigger asChild>
            <Button variant="outline" size="sm" className="h-8 gap-1.5">
              <Filter className="w-3.5 h-3.5" />
              Source
              {sources.length > 0 && sources.length < availableSources.length && (
                <span className="text-xs bg-primary/10 text-primary px-1.5 py-0.5 rounded-full">
                  {sources.length}
                </span>
              )}
            </Button>
          </PopoverTrigger>
          <PopoverContent className="w-48" align="start">
            <div className="space-y-3">
              <p className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
                Filter by source
              </p>
              {availableSources.map((source) => {
                const config = SOURCE_CONFIG[source]
                return (
                  <div key={source} className="flex items-center gap-2">
                    <Checkbox
                      id={`source-${source}`}
                      checked={sources.length === 0 || sources.includes(source)}
                      onCheckedChange={(checked) =>
                        handleSourceToggle(source, checked as boolean)
                      }
                    />
                    <Label
                      htmlFor={`source-${source}`}
                      className="flex items-center gap-2 text-sm font-normal cursor-pointer"
                    >
                      {config?.icon}
                      {config?.label || source}
                    </Label>
                  </div>
                )
              })}
            </div>
          </PopoverContent>
        </Popover>
      )}
    </div>
  )
}
