import { useState, useEffect, useRef, useCallback } from 'react'
import { Loader2 } from 'lucide-react'
import {
  Popover,
  PopoverContent,
  PopoverAnchor,
} from '@/components/ui/popover'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'
import { tempo } from '@/services'
import type { JiraIssueItem } from '@/types'

interface IssueKeyComboboxProps {
  value: string
  onChange: (value: string) => void
  onBlur?: () => void
  placeholder?: string
  className?: string
  compact?: boolean
}

export function IssueKeyCombobox({
  value,
  onChange,
  onBlur,
  placeholder = 'e.g. PROJ-123',
  className,
  compact,
}: IssueKeyComboboxProps) {
  const [open, setOpen] = useState(false)
  const [issues, setIssues] = useState<JiraIssueItem[]>([])
  const [loading, setLoading] = useState(false)
  const [activeIndex, setActiveIndex] = useState(-1)
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const inputRef = useRef<HTMLInputElement>(null)
  const listRef = useRef<HTMLUListElement>(null)
  const suppressBlurRef = useRef(false)
  // Keep a ref to the latest onBlur so setTimeout callbacks always call the current version
  const onBlurRef = useRef(onBlur)
  onBlurRef.current = onBlur

  const searchIssues = useCallback(async (query: string) => {
    setLoading(true)
    try {
      const result = await tempo.searchIssues({ query, max_results: 10 })
      setIssues(result.issues)
      setActiveIndex(-1)
      if (result.issues.length > 0) {
        setOpen(true)
      }
    } catch {
      setIssues([])
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current)

    if (value.trim().length === 0) {
      setIssues([])
      setOpen(false)
      return
    }

    debounceRef.current = setTimeout(() => {
      searchIssues(value.trim())
    }, 300)

    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current)
    }
  }, [value, searchIssues])

  // Scroll active item into view
  useEffect(() => {
    if (activeIndex < 0 || !listRef.current) return
    const items = listRef.current.querySelectorAll('li')
    items[activeIndex]?.scrollIntoView({ block: 'nearest' })
  }, [activeIndex])

  const handleSelect = (key: string) => {
    onChange(key)
    setOpen(false)
    setActiveIndex(-1)
    // Use ref so the callback runs after React re-renders with the selected value
    setTimeout(() => onBlurRef.current?.(), 0)
  }

  const handleBlur = () => {
    // Delay close so mousedown on items can fire first
    if (suppressBlurRef.current) {
      suppressBlurRef.current = false
      return
    }
    setTimeout(() => {
      setOpen(false)
      setActiveIndex(-1)
      onBlurRef.current?.()
    }, 150)
  }

  const handleFocus = () => {
    if (value.trim() && issues.length > 0) {
      setOpen(true)
    }
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      setOpen(false)
      setActiveIndex(-1)
      return
    }

    if (!open || issues.length === 0) return

    if (e.key === 'ArrowDown') {
      e.preventDefault()
      setActiveIndex((prev) => (prev < issues.length - 1 ? prev + 1 : 0))
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      setActiveIndex((prev) => (prev > 0 ? prev - 1 : issues.length - 1))
    } else if (e.key === 'Enter') {
      e.preventDefault()
      if (activeIndex >= 0 && activeIndex < issues.length) {
        handleSelect(issues[activeIndex].key)
      }
    }
  }

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverAnchor asChild>
        <div className="relative flex-1">
          <Input
            ref={inputRef}
            value={value}
            onChange={(e) => onChange(e.target.value)}
            onFocus={handleFocus}
            onBlur={handleBlur}
            onKeyDown={handleKeyDown}
            placeholder={placeholder}
            className={cn(compact && 'h-8 text-xs', className)}
            role="combobox"
            aria-expanded={open}
            aria-activedescendant={activeIndex >= 0 ? `issue-option-${activeIndex}` : undefined}
            aria-autocomplete="list"
          />
          {loading && (
            <Loader2 className="absolute right-2 top-1/2 -translate-y-1/2 w-3 h-3 animate-spin text-muted-foreground" />
          )}
        </div>
      </PopoverAnchor>
      <PopoverContent
        className="p-0 w-[--radix-popover-trigger-width]"
        align="start"
        sideOffset={4}
        onOpenAutoFocus={(e) => e.preventDefault()}
        onCloseAutoFocus={(e) => e.preventDefault()}
      >
        {issues.length === 0 ? (
          <div className="py-3 text-center text-xs text-muted-foreground">
            No issues found
          </div>
        ) : (
          <ul ref={listRef} className="max-h-[200px] overflow-y-auto py-1" role="listbox">
            {issues.map((issue, i) => (
              <li
                key={issue.key}
                id={`issue-option-${i}`}
                role="option"
                aria-selected={i === activeIndex}
                className={cn(
                  'flex items-center gap-2 px-3 py-1.5 text-xs cursor-pointer',
                  i === activeIndex
                    ? 'bg-accent text-accent-foreground'
                    : 'hover:bg-accent hover:text-accent-foreground',
                )}
                onMouseDown={() => {
                  suppressBlurRef.current = true
                  handleSelect(issue.key)
                }}
                onMouseEnter={() => setActiveIndex(i)}
              >
                <span className="font-medium shrink-0">{issue.key}</span>
                <span className="text-muted-foreground truncate">
                  {issue.summary}
                </span>
              </li>
            ))}
          </ul>
        )}
      </PopoverContent>
    </Popover>
  )
}
