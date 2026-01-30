import { useCallback, useState } from 'react'
import { X } from 'lucide-react'
import { projects as projectsService } from '@/services'

interface AddProjectDialogProps {
  open: boolean
  onClose: () => void
  onAdded: () => void
}

export function AddProjectDialog({ open, onClose, onAdded }: AddProjectDialogProps) {
  const [projectName, setProjectName] = useState('')
  const [gitRepoPath, setGitRepoPath] = useState('')
  const [displayName, setDisplayName] = useState('')
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const handleSubmit = useCallback(async () => {
    if (!projectName.trim() || !gitRepoPath.trim()) return

    setSaving(true)
    setError(null)
    try {
      await projectsService.addManualProject({
        project_name: projectName.trim(),
        git_repo_path: gitRepoPath.trim(),
        display_name: displayName.trim() || null,
      })
      setProjectName('')
      setGitRepoPath('')
      setDisplayName('')
      onAdded()
    } catch (err) {
      setError(String(err))
    } finally {
      setSaving(false)
    }
  }, [projectName, gitRepoPath, displayName, onAdded])

  const handleClose = useCallback(() => {
    setProjectName('')
    setGitRepoPath('')
    setDisplayName('')
    setError(null)
    onClose()
  }, [onClose])

  if (!open) return null

  return (
    <>
      {/* Backdrop */}
      <div
        className="fixed inset-0 bg-black/20 z-40 transition-opacity"
        onClick={handleClose}
      />

      {/* Dialog */}
      <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
        <div
          className="w-full max-w-md bg-background border border-border rounded-lg shadow-xl"
          onClick={(e) => e.stopPropagation()}
        >
          {/* Header */}
          <div className="flex items-center justify-between px-5 py-4 border-b border-border">
            <h3 className="font-display text-lg text-foreground">新增專案</h3>
            <button
              onClick={handleClose}
              className="p-1.5 rounded hover:bg-foreground/10 transition-colors"
            >
              <X className="w-4 h-4 text-muted-foreground" />
            </button>
          </div>

          {/* Body */}
          <div className="px-5 py-5 space-y-4">
            <p className="text-xs text-muted-foreground">
              手動新增沒有使用 Claude Code 的專案。至少需要一個 Git 儲存庫路徑。
            </p>

            {/* Project name */}
            <div>
              <label className="block text-xs font-medium text-foreground mb-1.5">
                專案名稱 <span className="text-red-400">*</span>
              </label>
              <input
                type="text"
                value={projectName}
                onChange={(e) => setProjectName(e.target.value)}
                placeholder="my-project"
                className="w-full text-sm px-3 py-2 rounded border border-border bg-background text-foreground placeholder:text-muted-foreground/40 focus:outline-none focus:ring-1 focus:ring-foreground/20"
                autoFocus
              />
            </div>

            {/* Git repo path */}
            <div>
              <label className="block text-xs font-medium text-foreground mb-1.5">
                Git 儲存庫路徑 <span className="text-red-400">*</span>
              </label>
              <input
                type="text"
                value={gitRepoPath}
                onChange={(e) => setGitRepoPath(e.target.value)}
                placeholder="/Users/username/projects/my-project"
                className="w-full text-sm px-3 py-2 rounded border border-border bg-background text-foreground placeholder:text-muted-foreground/40 focus:outline-none focus:ring-1 focus:ring-foreground/20"
              />
              <p className="text-[10px] text-muted-foreground/50 mt-1">
                專案的根目錄路徑，需包含 .git 資料夾。
              </p>
            </div>

            {/* Display name (optional) */}
            <div>
              <label className="block text-xs font-medium text-foreground mb-1.5">
                顯示名稱 <span className="text-muted-foreground/50">(選填)</span>
              </label>
              <input
                type="text"
                value={displayName}
                onChange={(e) => setDisplayName(e.target.value)}
                placeholder="My Project"
                className="w-full text-sm px-3 py-2 rounded border border-border bg-background text-foreground placeholder:text-muted-foreground/40 focus:outline-none focus:ring-1 focus:ring-foreground/20"
              />
            </div>

            {error && (
              <p className="text-xs text-red-500 bg-red-50 dark:bg-red-950/20 px-3 py-2 rounded">
                {error}
              </p>
            )}
          </div>

          {/* Footer */}
          <div className="flex items-center justify-end gap-2 px-5 py-4 border-t border-border">
            <button
              onClick={handleClose}
              className="px-3 py-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors rounded"
            >
              取消
            </button>
            <button
              onClick={handleSubmit}
              disabled={saving || !projectName.trim() || !gitRepoPath.trim()}
              className="px-3 py-1.5 text-xs text-background bg-foreground rounded hover:bg-foreground/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {saving ? '新增中...' : '新增'}
            </button>
          </div>
        </div>
      </div>
    </>
  )
}
