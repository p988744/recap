import { Clock, GitCommit, FileCode } from 'lucide-react'
import type { HourlyBreakdownItem } from '@/types/worklog'
import { MarkdownSummary } from '@/components/MarkdownSummary'

interface HourlyBreakdownProps {
  items: HourlyBreakdownItem[]
  loading: boolean
}

export function HourlyBreakdown({ items, loading }: HourlyBreakdownProps) {
  if (loading) {
    return (
      <div className="flex items-center justify-center py-6">
        <div className="w-4 h-4 border border-border border-t-foreground/60 rounded-full animate-spin" />
      </div>
    )
  }

  if (items.length === 0) {
    return (
      <div className="py-4 px-4 text-center">
        <p className="text-xs text-muted-foreground">尚無逐時資料</p>
      </div>
    )
  }

  return (
    <div className="divide-y divide-border">
      {items.map((item, i) => (
        <HourlyCard key={i} item={item} />
      ))}
    </div>
  )
}

function uniqueFileNames(files: string[]): string[] {
  const seen = new Set<string>()
  const result: string[] = []
  for (const f of files) {
    const name = f.split('/').pop() || f
    if (!seen.has(name)) {
      seen.add(name)
      result.push(name)
    }
  }
  return result
}

function HourlyCard({ item }: { item: HourlyBreakdownItem }) {
  const fileNames = uniqueFileNames(item.files_modified)

  return (
    <div className="px-4 py-3 pl-11">
      {/* Time label */}
      <span className="flex items-center gap-1 text-xs text-muted-foreground mb-1.5">
        <Clock className="w-3 h-3" strokeWidth={1.5} />
        {item.hour_start}–{item.hour_end}
      </span>

      {/* Summary */}
      <MarkdownSummary content={item.summary} />

      {/* Files modified */}
      {fileNames.length > 0 && (
        <div className="mt-2">
          <div className="flex items-center gap-1 mb-1">
            <FileCode className="w-3 h-3 text-muted-foreground" strokeWidth={1.5} />
            <span className="text-[10px] uppercase tracking-wider text-muted-foreground">
              修改檔案
            </span>
          </div>
          <div className="flex flex-wrap gap-1">
            {fileNames.slice(0, 8).map((name, j) => (
              <span
                key={j}
                className="text-xs text-muted-foreground bg-muted/50 px-1.5 py-0.5 rounded font-mono"
              >
                {name}
              </span>
            ))}
            {fileNames.length > 8 && (
              <span className="text-xs text-muted-foreground">
                +{fileNames.length - 8}
              </span>
            )}
          </div>
        </div>
      )}

      {/* Git commits */}
      {item.git_commits.length > 0 && (
        <div className="mt-2">
          <div className="flex items-center gap-1 mb-1">
            <GitCommit className="w-3 h-3 text-muted-foreground" strokeWidth={1.5} />
            <span className="text-[10px] uppercase tracking-wider text-muted-foreground">
              Commits
            </span>
          </div>
          <div className="space-y-0.5">
            {item.git_commits.map((commit, j) => (
              <div key={j} className="flex items-baseline gap-2">
                <span className="text-xs font-mono text-muted-foreground shrink-0">
                  {commit.hash.slice(0, 7)}
                </span>
                <span className="text-xs text-foreground truncate">{commit.message}</span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}
