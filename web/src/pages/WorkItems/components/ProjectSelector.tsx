import { useState, useEffect, useCallback } from 'react'
import { Check, ChevronsUpDown, Plus } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover'
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
} from '@/components/ui/command'
import { cn } from '@/lib/utils'
import { projects as projectsService } from '@/services'
import type { ProjectInfo } from '@/types'

interface ProjectSelectorProps {
  value: string
  onChange: (value: string) => void
  placeholder?: string
}

export function ProjectSelector({
  value,
  onChange,
  placeholder = '選擇專案...',
}: ProjectSelectorProps) {
  const [open, setOpen] = useState(false)
  const [projects, setProjects] = useState<ProjectInfo[]>([])
  const [loading, setLoading] = useState(false)
  const [showNewInput, setShowNewInput] = useState(false)
  const [newProjectName, setNewProjectName] = useState('')

  // Fetch projects when popover opens
  const fetchProjects = useCallback(async () => {
    if (projects.length > 0) return // Already loaded
    setLoading(true)
    try {
      const response = await projectsService.listProjects()
      // Filter to only show visible projects
      setProjects(response.filter(p => !p.hidden))
    } catch (err) {
      console.error('Failed to fetch projects:', err)
    } finally {
      setLoading(false)
    }
  }, [projects.length])

  useEffect(() => {
    if (open) {
      fetchProjects()
    }
  }, [open, fetchProjects])

  const handleSelect = (projectName: string) => {
    onChange(projectName === value ? '' : projectName)
    setOpen(false)
  }

  const handleAddNew = () => {
    if (newProjectName.trim()) {
      onChange(newProjectName.trim())
      setNewProjectName('')
      setShowNewInput(false)
      setOpen(false)
    }
  }

  const displayValue = value || placeholder

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          role="combobox"
          aria-expanded={open}
          className="w-full justify-between font-normal"
        >
          <span className={cn(!value && 'text-muted-foreground')}>
            {displayValue}
          </span>
          <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-[300px] p-0" align="start">
        <Command>
          <CommandInput placeholder="搜尋專案..." />
          <CommandList>
            <CommandEmpty>
              {loading ? '載入中...' : '沒有找到專案'}
            </CommandEmpty>
            <CommandGroup>
              {/* Option to clear selection */}
              {value && (
                <CommandItem
                  value="__clear__"
                  onSelect={() => handleSelect('')}
                  className="text-muted-foreground"
                >
                  <span className="ml-6">清除選擇</span>
                </CommandItem>
              )}
              {projects.map((project) => (
                <CommandItem
                  key={project.project_name}
                  value={project.project_name}
                  onSelect={() => handleSelect(project.project_name)}
                >
                  <Check
                    className={cn(
                      'mr-2 h-4 w-4',
                      value === project.project_name ? 'opacity-100' : 'opacity-0'
                    )}
                  />
                  <div className="flex flex-col">
                    <span>{project.display_name || project.project_name}</span>
                    <span className="text-xs text-muted-foreground">
                      {project.work_item_count} 項 · {project.total_hours.toFixed(1)}h
                    </span>
                  </div>
                </CommandItem>
              ))}
            </CommandGroup>
            <CommandSeparator />
            <CommandGroup>
              {showNewInput ? (
                <div className="p-2 space-y-2">
                  <Input
                    placeholder="輸入新專案名稱..."
                    value={newProjectName}
                    onChange={(e) => setNewProjectName(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') {
                        e.preventDefault()
                        handleAddNew()
                      }
                      if (e.key === 'Escape') {
                        setShowNewInput(false)
                        setNewProjectName('')
                      }
                    }}
                    autoFocus
                  />
                  <div className="flex gap-2">
                    <Button
                      size="sm"
                      variant="ghost"
                      className="flex-1"
                      onClick={() => {
                        setShowNewInput(false)
                        setNewProjectName('')
                      }}
                    >
                      取消
                    </Button>
                    <Button
                      size="sm"
                      className="flex-1"
                      onClick={handleAddNew}
                      disabled={!newProjectName.trim()}
                    >
                      新增
                    </Button>
                  </div>
                </div>
              ) : (
                <CommandItem
                  onSelect={() => setShowNewInput(true)}
                  className="text-primary"
                >
                  <Plus className="mr-2 h-4 w-4" />
                  新增專案
                </CommandItem>
              )}
            </CommandGroup>
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  )
}
