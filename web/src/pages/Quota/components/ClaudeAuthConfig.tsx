/**
 * Claude Auth Configuration Component
 *
 * Provides UI for configuring Claude OAuth token manually
 * when automatic credential discovery fails.
 */

import { useState, useEffect, useCallback } from 'react'
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Badge } from '@/components/ui/badge'
import { Key, RefreshCw, Check, X, Info, Loader2, ChevronDown, ChevronUp, Terminal, Globe } from 'lucide-react'
import { quota } from '@/services'
import type { ClaudeAuthStatus } from '@/types/quota'

const LOG_PREFIX = '[ClaudeAuthConfig]'

interface ClaudeAuthConfigProps {
  onAuthStatusChange?: () => void
}

export function ClaudeAuthConfig({ onAuthStatusChange }: ClaudeAuthConfigProps) {
  // Auth status state
  const [authStatus, setAuthStatus] = useState<ClaudeAuthStatus | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // Token input state
  const [tokenInput, setTokenInput] = useState('')
  const [saving, setSaving] = useState(false)
  const [saveSuccess, setSaveSuccess] = useState(false)
  const [showInstructions, setShowInstructions] = useState(false)

  // Fetch auth status
  const fetchAuthStatus = useCallback(async () => {
    console.log(`${LOG_PREFIX} Fetching auth status...`)
    setLoading(true)
    setError(null)

    try {
      const status = await quota.checkClaudeAuthStatus()
      console.log(`${LOG_PREFIX} Auth status:`, status)
      setAuthStatus(status)

      // If manual token is configured, fetch it to show (masked)
      if (status.manual_configured) {
        const token = await quota.getClaudeOAuthToken()
        if (token) {
          // Show masked token (first 8 chars + ...)
          setTokenInput(token.substring(0, 8) + '...')
        }
      }
    } catch (err) {
      console.error(`${LOG_PREFIX} Error fetching auth status:`, err)
      setError(err instanceof Error ? err.message : 'Failed to fetch auth status')
    } finally {
      setLoading(false)
    }
  }, [])

  // Save token
  const handleSaveToken = useCallback(async () => {
    if (!tokenInput.trim() || tokenInput.includes('...')) {
      return
    }

    console.log(`${LOG_PREFIX} Saving OAuth token...`)
    setSaving(true)
    setError(null)
    setSaveSuccess(false)

    try {
      await quota.setClaudeOAuthToken(tokenInput)
      console.log(`${LOG_PREFIX} Token saved successfully`)
      setSaveSuccess(true)

      // Refresh auth status
      await fetchAuthStatus()

      // Notify parent
      onAuthStatusChange?.()

      // Clear success message after 3 seconds
      setTimeout(() => setSaveSuccess(false), 3000)
    } catch (err) {
      console.error(`${LOG_PREFIX} Error saving token:`, err)
      setError(err instanceof Error ? err.message : 'Failed to save token')
    } finally {
      setSaving(false)
    }
  }, [tokenInput, fetchAuthStatus, onAuthStatusChange])

  // Clear token
  const handleClearToken = useCallback(async () => {
    console.log(`${LOG_PREFIX} Clearing OAuth token...`)
    setSaving(true)
    setError(null)

    try {
      await quota.setClaudeOAuthToken(null)
      console.log(`${LOG_PREFIX} Token cleared`)
      setTokenInput('')

      // Refresh auth status
      await fetchAuthStatus()

      // Notify parent
      onAuthStatusChange?.()
    } catch (err) {
      console.error(`${LOG_PREFIX} Error clearing token:`, err)
      setError(err instanceof Error ? err.message : 'Failed to clear token')
    } finally {
      setSaving(false)
    }
  }, [fetchAuthStatus, onAuthStatusChange])

  // Initial load
  useEffect(() => {
    fetchAuthStatus()
  }, [fetchAuthStatus])

  // Render auth status badge
  const renderStatusBadge = () => {
    if (!authStatus) return null

    const { active_source, manual_configured, manual_valid } = authStatus

    if (active_source === 'auto') {
      return (
        <Badge variant="default" className="gap-1">
          <Check className="w-3 h-3" />
          自動認證
        </Badge>
      )
    }

    if (active_source === 'manual' && manual_valid) {
      return (
        <Badge variant="default" className="gap-1">
          <Check className="w-3 h-3" />
          手動 Token
        </Badge>
      )
    }

    if (manual_configured && !manual_valid) {
      return (
        <Badge variant="destructive" className="gap-1">
          <X className="w-3 h-3" />
          Token 無效
        </Badge>
      )
    }

    return (
      <Badge variant="secondary" className="gap-1">
        <X className="w-3 h-3" />
        未認證
      </Badge>
    )
  }

  if (loading) {
    return (
      <Card>
        <CardContent className="flex items-center justify-center h-48">
          <Loader2 className="w-5 h-5 animate-spin text-muted-foreground" />
        </CardContent>
      </Card>
    )
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Key className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
            <CardTitle className="text-sm">Claude OAuth 認證</CardTitle>
          </div>
          {renderStatusBadge()}
        </div>
        <CardDescription>
          配額追蹤需要 Claude OAuth 認證。優先使用自動認證，若失敗可手動輸入 Token。
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Auto auth status */}
        <div className="flex items-center justify-between p-3 rounded-lg bg-muted/50">
          <div className="flex items-center gap-2">
            <Info className="w-4 h-4 text-muted-foreground" />
            <div className="text-sm">
              <span>自動認證</span>
              <span className="text-muted-foreground text-xs ml-1">
                （系統憑證儲存區）
              </span>
            </div>
          </div>
          {authStatus?.auto_available ? (
            <Badge variant="outline" className="text-green-600 border-green-200 bg-green-50 dark:bg-green-950 dark:border-green-800 dark:text-green-400">
              可用
            </Badge>
          ) : (
            <Badge variant="outline" className="text-muted-foreground">
              不可用
            </Badge>
          )}
        </div>

        {/* Manual token input */}
        <div className="space-y-3">
          <Label htmlFor="oauth-token" className="text-sm">
            手動 OAuth Token
          </Label>
          <div className="flex gap-2">
            <Input
              id="oauth-token"
              type="password"
              placeholder="sk-ant-oat01-..."
              value={tokenInput}
              onChange={(e) => setTokenInput(e.target.value)}
              className="font-mono text-xs"
            />
            <Button
              size="sm"
              onClick={handleSaveToken}
              disabled={saving || !tokenInput.trim() || tokenInput.includes('...')}
            >
              {saving ? <Loader2 className="w-4 h-4 animate-spin" /> : '儲存'}
            </Button>
            {authStatus?.manual_configured && (
              <Button
                size="sm"
                variant="outline"
                onClick={handleClearToken}
                disabled={saving}
              >
                清除
              </Button>
            )}
          </div>

          {/* Expandable instructions */}
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setShowInstructions(!showInstructions)}
            className="gap-1 text-xs text-muted-foreground hover:text-foreground p-0 h-auto"
          >
            {showInstructions ? (
              <ChevronUp className="w-3 h-3" />
            ) : (
              <ChevronDown className="w-3 h-3" />
            )}
            如何取得 OAuth Token？
          </Button>

          {showInstructions && (
            <div className="space-y-4 p-4 rounded-lg bg-muted/30 border text-sm">
              {/* Method 1: Claude Code CLI */}
              <div className="space-y-2">
                <div className="flex items-center gap-2 font-medium">
                  <Terminal className="w-4 h-4" />
                  方法一：Claude Code CLI（推薦）
                </div>
                <p className="text-muted-foreground text-xs">
                  如果你已安裝 Claude Code CLI，執行以下指令登入：
                </p>
                <code className="block p-2 rounded bg-muted font-mono text-xs">
                  claude /login
                </code>
                <p className="text-muted-foreground text-xs">
                  登入後 Token 會自動存入系統 Keychain，Recap 會自動讀取。
                </p>
              </div>

              {/* Method 2: Claude Web */}
              <div className="space-y-2">
                <div className="flex items-center gap-2 font-medium">
                  <Globe className="w-4 h-4" />
                  方法二：從 Claude 網頁版取得
                </div>
                <ol className="list-decimal list-inside space-y-1.5 text-xs text-muted-foreground">
                  <li>用瀏覽器開啟 <a href="https://claude.ai" target="_blank" rel="noopener noreferrer" className="text-primary hover:underline">claude.ai</a> 並登入</li>
                  <li>
                    開啟開發者工具 Network 分頁：
                    <ul className="list-disc list-inside ml-4 mt-1">
                      <li>macOS: <kbd className="bg-muted px-1 rounded">Cmd + Option + I</kbd> → Network</li>
                      <li>Windows/Linux: <kbd className="bg-muted px-1 rounded">F12</kbd> → Network</li>
                    </ul>
                  </li>
                  <li>在 claude.ai 發送任意訊息</li>
                  <li>在 Network 分頁篩選 <code className="bg-muted px-1 rounded">api</code></li>
                  <li>點擊任一請求 → Headers → 找到 <code className="bg-muted px-1 rounded">Authorization</code></li>
                  <li>複製 <code className="bg-muted px-1 rounded">Bearer</code> 後面的 token 值</li>
                </ol>
                <p className="text-xs text-muted-foreground mt-2">
                  Token 格式類似：<code className="bg-muted px-1 rounded text-[10px]">sk-ant-sid01-xxx...</code>
                </p>
              </div>

              {/* Note */}
              <div className="flex items-start gap-2 p-2 rounded bg-amber-500/10 border border-amber-500/20 text-xs">
                <Info className="w-4 h-4 text-amber-600 flex-shrink-0 mt-0.5" />
                <div className="text-amber-800 dark:text-amber-200">
                  <strong>注意：</strong>此功能僅適用於 Claude Pro/Max 訂閱用戶。
                  Token 會過期，若配額查詢失敗請重新取得。
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Error message */}
        {error && (
          <div className="flex items-center gap-2 p-3 rounded-lg border border-destructive/50 bg-destructive/10 text-destructive text-sm">
            <X className="w-4 h-4 flex-shrink-0" />
            <span>{error}</span>
          </div>
        )}

        {/* Success message */}
        {saveSuccess && (
          <div className="flex items-center gap-2 p-3 rounded-lg border border-green-200 bg-green-50 text-green-800 text-sm">
            <Check className="w-4 h-4 flex-shrink-0" />
            <span>Token 已儲存並驗證成功</span>
          </div>
        )}

        {/* Refresh button */}
        <div className="flex justify-end">
          <Button
            variant="ghost"
            size="sm"
            onClick={fetchAuthStatus}
            disabled={loading}
            className="gap-1 text-xs"
          >
            <RefreshCw className={`w-3 h-3 ${loading ? 'animate-spin' : ''}`} />
            重新檢查
          </Button>
        </div>
      </CardContent>
    </Card>
  )
}

export default ClaudeAuthConfig
