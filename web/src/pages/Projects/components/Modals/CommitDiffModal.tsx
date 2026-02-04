import { useEffect } from 'react'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { AlertCircle, FileText, FilePlus, FileMinus, FileEdit, ArrowRight, Plus, Minus, Clock, User } from 'lucide-react'
import { cn } from '@/lib/utils'
import { useCommitDiff } from '../../hooks/useCommitDiff'
import type { CommitFileChange } from '@/types'

interface CommitDiffModalProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  projectPath: string | null
  commitHash: string
  commitMessage?: string
}

const FILE_STATUS_CONFIG: Record<string, { icon: React.ReactNode; className: string; label: string }> = {
  added: {
    icon: <FilePlus className="w-4 h-4" />,
    className: 'text-green-600 dark:text-green-400',
    label: 'Added',
  },
  modified: {
    icon: <FileEdit className="w-4 h-4" />,
    className: 'text-amber-600 dark:text-amber-400',
    label: 'Modified',
  },
  deleted: {
    icon: <FileMinus className="w-4 h-4" />,
    className: 'text-red-600 dark:text-red-400',
    label: 'Deleted',
  },
  renamed: {
    icon: <ArrowRight className="w-4 h-4" />,
    className: 'text-blue-600 dark:text-blue-400',
    label: 'Renamed',
  },
  copied: {
    icon: <FileText className="w-4 h-4" />,
    className: 'text-purple-600 dark:text-purple-400',
    label: 'Copied',
  },
}

function formatDate(isoString: string): string {
  try {
    const date = new Date(isoString)
    return date.toLocaleString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    })
  } catch {
    return isoString
  }
}

function FileChangeRow({ file }: { file: CommitFileChange }) {
  const config = FILE_STATUS_CONFIG[file.status] || FILE_STATUS_CONFIG.modified

  return (
    <div className="flex items-center gap-2 py-1.5 px-2 hover:bg-muted/50 rounded">
      <span className={cn('flex-shrink-0', config.className)} title={config.label}>
        {config.icon}
      </span>

      <div className="flex-1 min-w-0">
        {file.old_path ? (
          <span className="text-sm font-mono truncate">
            <span className="text-muted-foreground">{file.old_path}</span>
            <ArrowRight className="w-3 h-3 inline mx-1 text-muted-foreground" />
            <span>{file.path}</span>
          </span>
        ) : (
          <span className="text-sm font-mono truncate">{file.path}</span>
        )}
      </div>

      {/* Stats */}
      <div className="flex items-center gap-2 text-xs flex-shrink-0">
        {file.insertions > 0 && (
          <span className="flex items-center gap-0.5 text-green-600 dark:text-green-400">
            <Plus className="w-3 h-3" />
            {file.insertions}
          </span>
        )}
        {file.deletions > 0 && (
          <span className="flex items-center gap-0.5 text-red-600 dark:text-red-400">
            <Minus className="w-3 h-3" />
            {file.deletions}
          </span>
        )}
      </div>
    </div>
  )
}

function DiffViewer({ diffText }: { diffText: string }) {
  // Parse diff lines and add coloring
  const lines = diffText.split('\n')

  return (
    <div className="bg-zinc-900 dark:bg-zinc-950 rounded-lg overflow-hidden">
      <pre className="text-xs font-mono overflow-x-auto p-4 max-h-[400px] overflow-y-auto">
        {lines.map((line, index) => {
          let className = 'text-zinc-400'
          if (line.startsWith('+') && !line.startsWith('+++')) {
            className = 'text-green-400 bg-green-950/30'
          } else if (line.startsWith('-') && !line.startsWith('---')) {
            className = 'text-red-400 bg-red-950/30'
          } else if (line.startsWith('@@')) {
            className = 'text-blue-400'
          } else if (line.startsWith('diff --git') || line.startsWith('index ') || line.startsWith('---') || line.startsWith('+++')) {
            className = 'text-zinc-500'
          }

          return (
            <div key={index} className={cn('whitespace-pre', className)}>
              {line || ' '}
            </div>
          )
        })}
      </pre>
    </div>
  )
}

export function CommitDiffModal({
  open,
  onOpenChange,
  projectPath,
  commitHash,
  commitMessage,
}: CommitDiffModalProps) {
  const { diff, isLoading, error, fetchDiff, clearDiff } = useCommitDiff()

  useEffect(() => {
    if (open && projectPath && commitHash) {
      fetchDiff(projectPath, commitHash)
    }
    if (!open) {
      clearDiff()
    }
  }, [open, projectPath, commitHash, fetchDiff, clearDiff])

  const shortHash = commitHash.slice(0, 7)

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-4xl max-h-[90vh] overflow-hidden flex flex-col">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <span className="font-mono text-sm bg-muted px-2 py-0.5 rounded">
              {shortHash}
            </span>
            <span className="truncate">
              {commitMessage || diff?.message || 'Commit Details'}
            </span>
          </DialogTitle>
        </DialogHeader>

        <div className="flex-1 overflow-y-auto space-y-4">
          {/* Loading state */}
          {isLoading && (
            <div className="flex items-center justify-center py-16">
              <div className="w-6 h-6 border border-border border-t-foreground/60 rounded-full animate-spin" />
            </div>
          )}

          {/* Error state */}
          {error && !isLoading && (
            <div className="flex flex-col items-center justify-center py-8 gap-3">
              <AlertCircle className="w-10 h-10 text-destructive" />
              <p className="text-sm text-destructive text-center">{error}</p>
              {!projectPath && (
                <p className="text-xs text-muted-foreground">
                  This project does not have a local git repository path configured.
                </p>
              )}
            </div>
          )}

          {/* Diff content */}
          {diff && !isLoading && (
            <>
              {/* Commit info */}
              <div className="flex flex-wrap items-center gap-4 text-sm text-muted-foreground">
                <div className="flex items-center gap-1">
                  <User className="w-4 h-4" />
                  <span>{diff.author}</span>
                </div>
                <div className="flex items-center gap-1">
                  <Clock className="w-4 h-4" />
                  <span>{formatDate(diff.date)}</span>
                </div>
              </div>

              {/* Stats summary */}
              <div className="flex items-center gap-4 text-sm">
                <span className="text-muted-foreground">
                  {diff.stats.files_changed} file{diff.stats.files_changed !== 1 ? 's' : ''} changed
                </span>
                {diff.stats.insertions > 0 && (
                  <span className="flex items-center gap-1 text-green-600 dark:text-green-400">
                    <Plus className="w-4 h-4" />
                    {diff.stats.insertions} insertion{diff.stats.insertions !== 1 ? 's' : ''}
                  </span>
                )}
                {diff.stats.deletions > 0 && (
                  <span className="flex items-center gap-1 text-red-600 dark:text-red-400">
                    <Minus className="w-4 h-4" />
                    {diff.stats.deletions} deletion{diff.stats.deletions !== 1 ? 's' : ''}
                  </span>
                )}
              </div>

              {/* File list */}
              {diff.files.length > 0 && (
                <div className="border rounded-lg">
                  <div className="px-3 py-2 border-b bg-muted/30">
                    <span className="text-sm font-medium">Changed Files</span>
                  </div>
                  <div className="divide-y">
                    {diff.files.map((file, index) => (
                      <FileChangeRow key={`${file.path}-${index}`} file={file} />
                    ))}
                  </div>
                </div>
              )}

              {/* Diff viewer */}
              {diff.diff_text ? (
                <div>
                  <div className="mb-2">
                    <span className="text-sm font-medium">Diff</span>
                  </div>
                  <DiffViewer diffText={diff.diff_text} />
                </div>
              ) : (
                <div className="text-center py-8 text-muted-foreground text-sm">
                  <p>Diff text not available.</p>
                  <p className="text-xs mt-1">
                    The repository may not be accessible locally.
                  </p>
                </div>
              )}
            </>
          )}

          {/* No project path warning */}
          {!projectPath && !isLoading && !error && (
            <div className="flex flex-col items-center justify-center py-8 gap-3 text-center">
              <AlertCircle className="w-10 h-10 text-muted-foreground" />
              <p className="text-sm text-muted-foreground">
                Cannot load diff: This project does not have a local path configured.
              </p>
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  )
}
