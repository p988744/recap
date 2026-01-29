import { useCallback, useEffect, useState } from 'react'
import { Check, RotateCcw } from 'lucide-react'
import { Card } from '@/components/ui/card'
import { projects as projectsService } from '@/services'
import { GeminiIcon } from './icons/GeminiIcon'

export function AntigravityPathSetting() {
  const [path, setPath] = useState('')
  const [isDefault, setIsDefault] = useState(true)
  const [editing, setEditing] = useState(false)
  const [editValue, setEditValue] = useState('')
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const fetchPath = useCallback(async () => {
    try {
      const data = await projectsService.getAntigravitySessionPath()
      setPath(data.path)
      setIsDefault(data.is_default)
    } catch (err) {
      console.error('Failed to fetch Antigravity session path:', err)
    }
  }, [])

  useEffect(() => {
    fetchPath()
  }, [fetchPath])

  const handleSave = useCallback(async () => {
    setSaving(true)
    setError(null)
    try {
      await projectsService.updateAntigravitySessionPath(editValue || null)
      await fetchPath()
      setEditing(false)
    } catch (err) {
      setError(String(err))
    } finally {
      setSaving(false)
    }
  }, [editValue, fetchPath])

  const handleReset = useCallback(async () => {
    setSaving(true)
    setError(null)
    try {
      await projectsService.updateAntigravitySessionPath(null)
      await fetchPath()
      setEditing(false)
    } catch (err) {
      setError(String(err))
    } finally {
      setSaving(false)
    }
  }, [fetchPath])

  return (
    <Card className="p-4">
      <div className="flex items-center gap-2.5 mb-2">
        <GeminiIcon className="w-4 h-4 text-blue-500" />
        <span className="text-sm font-medium text-foreground">Antigravity Session 路徑</span>
        {isDefault && (
          <span className="text-[10px] text-muted-foreground/60 bg-foreground/5 px-1.5 py-0.5 rounded">
            預設
          </span>
        )}
      </div>

      {editing ? (
        <div className="ml-[26px] space-y-2">
          <input
            type="text"
            value={editValue}
            onChange={(e) => setEditValue(e.target.value)}
            placeholder="~/.gemini/antigravity"
            className="w-full text-xs px-2.5 py-1.5 rounded border border-border bg-background text-foreground placeholder:text-muted-foreground/40 focus:outline-none focus:ring-1 focus:ring-foreground/20"
            autoFocus
            onKeyDown={(e) => {
              if (e.key === 'Enter') handleSave()
              if (e.key === 'Escape') setEditing(false)
            }}
          />
          {error && (
            <p className="text-xs text-red-500">{error}</p>
          )}
          <div className="flex items-center gap-2">
            <button
              onClick={handleSave}
              disabled={saving}
              className="flex items-center gap-1 text-xs text-foreground hover:text-foreground/80 transition-colors disabled:opacity-50"
            >
              <Check className="w-3 h-3" />
              儲存
            </button>
            <button
              onClick={() => setEditing(false)}
              className="text-xs text-muted-foreground hover:text-foreground transition-colors"
            >
              取消
            </button>
            {!isDefault && (
              <button
                onClick={handleReset}
                disabled={saving}
                className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors ml-auto disabled:opacity-50"
              >
                <RotateCcw className="w-3 h-3" />
                重設為預設
              </button>
            )}
          </div>
        </div>
      ) : (
        <div className="ml-[26px]">
          <button
            onClick={() => {
              setEditValue(path)
              setEditing(true)
              setError(null)
            }}
            className="text-xs text-muted-foreground hover:text-foreground break-all leading-relaxed transition-colors text-left"
          >
            {path}
          </button>
          <p className="text-[10px] text-muted-foreground/50 mt-1">
            點擊修改路徑。此路徑用於掃描 Antigravity 專案 session。請使用絕對路徑。
          </p>
        </div>
      )}
    </Card>
  )
}
