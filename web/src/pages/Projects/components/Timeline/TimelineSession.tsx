import { useState } from 'react'
import { ChevronDown, ChevronRight, Clock, GitCommit, Plus, Minus } from 'lucide-react'
import { cn } from '@/lib/utils'
import type { TimelineSessionDetail, TimelineCommitDetail } from '@/types'
import { MarkdownSummary } from '@/components/MarkdownSummary'
import { ClaudeIcon, GeminiIcon } from '@/components/icons'
import { CommitDiffModal } from '../Modals/CommitDiffModal'

interface TimelineSessionProps {
  session: TimelineSessionDetail
  projectPath: string | null
}

const SOURCE_CONFIG: Record<string, { icon: React.ReactNode; label: string; className: string }> = {
  claude_code: {
    icon: <ClaudeIcon className="w-3.5 h-3.5" />,
    label: 'Claude Code',
    className: 'bg-amber-50 dark:bg-amber-900/20 border-amber-200/50 dark:border-amber-800/30',
  },
  antigravity: {
    icon: <GeminiIcon className="w-3.5 h-3.5" />,
    label: 'Antigravity',
    className: 'bg-blue-50 dark:bg-blue-900/20 border-blue-200/50 dark:border-blue-800/30',
  },
}

function formatTime(isoString: string): string {
  try {
    const date = new Date(isoString)
    return date.toLocaleTimeString('en-US', {
      hour: '2-digit',
      minute: '2-digit',
      hour12: false,
    })
  } catch {
    return isoString.slice(11, 16) || '--:--'
  }
}

function formatDuration(hours: number): string {
  if (hours < 1) {
    return `${Math.round(hours * 60)}m`
  }
  const h = Math.floor(hours)
  const m = Math.round((hours - h) * 60)
  return m > 0 ? `${h}h ${m}m` : `${h}h`
}

export function TimelineSession({ session, projectPath }: TimelineSessionProps) {
  const [isExpanded, setIsExpanded] = useState(false)
  const [selectedCommit, setSelectedCommit] = useState<TimelineCommitDetail | null>(null)
  const config = SOURCE_CONFIG[session.source] || SOURCE_CONFIG.claude_code

  const hasCommits = session.commits && session.commits.length > 0

  // Extract title without project prefix
  const title = session.title.includes(']')
    ? session.title.split(']').slice(1).join(']').trim()
    : session.title

  const handleCommitClick = (commit: TimelineCommitDetail, e: React.MouseEvent) => {
    e.stopPropagation()
    setSelectedCommit(commit)
  }

  return (
    <>
      <CommitDiffModal
        open={selectedCommit !== null}
        onOpenChange={(open) => !open && setSelectedCommit(null)}
        projectPath={projectPath}
        commitHash={selectedCommit?.hash || ''}
        commitMessage={selectedCommit?.message}
      />
    <div
      className={cn(
        'rounded-lg border transition-colors',
        config.className
      )}
    >
      {/* Header */}
      <div
        className={cn(
          'flex items-start gap-3 p-3 cursor-pointer',
          hasCommits && 'hover:bg-black/5 dark:hover:bg-white/5'
        )}
        onClick={() => hasCommits && setIsExpanded(!isExpanded)}
      >
        {/* Expand icon or placeholder */}
        <div className="w-4 h-4 mt-0.5 flex-shrink-0">
          {hasCommits ? (
            isExpanded ? (
              <ChevronDown className="w-4 h-4 text-muted-foreground" />
            ) : (
              <ChevronRight className="w-4 h-4 text-muted-foreground" />
            )
          ) : null}
        </div>

        {/* Content */}
        <div className="flex-1 min-w-0">
          {/* Source and time */}
          <div className="flex items-center gap-2 mb-1">
            {config.icon}
            <span className="text-xs text-muted-foreground">
              {formatTime(session.start_time)} - {formatTime(session.end_time)}
            </span>
            <span className="text-xs text-muted-foreground/60">
              ({formatDuration(session.hours)})
            </span>
          </div>

          {/* Title */}
          <h4 className="text-sm font-medium leading-snug">{title}</h4>

          {/* Summary */}
          {session.summary && (
            <div className="mt-2 text-sm text-muted-foreground">
              <MarkdownSummary content={session.summary} />
            </div>
          )}

          {/* Commits count badge */}
          {hasCommits && (
            <div className="flex items-center gap-1 mt-2">
              <GitCommit className="w-3 h-3 text-muted-foreground" />
              <span className="text-xs text-muted-foreground">
                {session.commits.length} commit{session.commits.length > 1 ? 's' : ''}
              </span>
            </div>
          )}
        </div>

        {/* Hours */}
        <div className="flex items-center gap-1 text-sm text-muted-foreground flex-shrink-0">
          <Clock className="w-3.5 h-3.5" />
          {session.hours.toFixed(1)}h
        </div>
      </div>

      {/* Expanded commits */}
      {isExpanded && hasCommits && (
        <div className="border-t px-3 py-2 bg-black/5 dark:bg-white/5">
          <div className="space-y-2 pl-7">
            {session.commits.map((commit, index) => (
              <CommitRow
                key={`${commit.hash}-${index}`}
                commit={commit}
                onClick={(e) => handleCommitClick(commit, e)}
              />
            ))}
          </div>
        </div>
      )}
    </div>
    </>
  )
}

interface CommitRowProps {
  commit: TimelineCommitDetail
  onClick: (e: React.MouseEvent) => void
}

function CommitRow({ commit, onClick }: CommitRowProps) {
  return (
    <div
      className="flex items-start gap-2 py-1 px-1 -mx-1 rounded cursor-pointer hover:bg-muted/50 transition-colors"
      onClick={onClick}
      title="Click to view diff"
    >
      {/* Hash */}
      <span className="font-mono text-xs text-muted-foreground flex-shrink-0 bg-muted/50 px-1.5 py-0.5 rounded">
        {commit.short_hash}
      </span>

      {/* Message */}
      <span className="text-sm flex-1 truncate">{commit.message}</span>

      {/* Stats */}
      {(commit.insertions > 0 || commit.deletions > 0) && (
        <div className="flex items-center gap-1 text-xs text-muted-foreground flex-shrink-0">
          {commit.insertions > 0 && (
            <span className="flex items-center gap-0.5 text-green-600 dark:text-green-400">
              <Plus className="w-3 h-3" />
              {commit.insertions}
            </span>
          )}
          {commit.deletions > 0 && (
            <span className="flex items-center gap-0.5 text-red-600 dark:text-red-400">
              <Minus className="w-3 h-3" />
              {commit.deletions}
            </span>
          )}
        </div>
      )}
    </div>
  )
}
