import { useMemo } from 'react'
import {
  CheckCircle2,
  XCircle,
  Loader2,
  Eye,
  Globe,
} from 'lucide-react'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { ScrollArea } from '@/components/ui/scroll-area'
import type { HttpExportConfig, HttpExportResponse, WorkItem } from '@/types'

interface HttpExportModalProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  configs: HttpExportConfig[]
  selectedConfigId: string
  onConfigChange: (id: string) => void
  items: WorkItem[]
  result: HttpExportResponse | null
  exporting: boolean
  exportedIds?: Set<string>
  onExport: (dryRun: boolean, includeExported?: boolean) => void
  onClose: () => void
}

export function HttpExportModal({
  open,
  onOpenChange,
  configs,
  selectedConfigId,
  onConfigChange,
  items,
  result,
  exporting,
  exportedIds,
  onExport,
  onClose,
}: HttpExportModalProps) {
  const selectedConfig = configs.find((c) => c.id === selectedConfigId)

  const newCount = useMemo(() => {
    if (!exportedIds || exportedIds.size === 0) return items.length
    return items.filter((i) => !exportedIds.has(i.id)).length
  }, [items, exportedIds])

  const exportedCount = items.length - newCount

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Globe className="w-4 h-4" strokeWidth={1.5} />
            HTTP Export
          </DialogTitle>
        </DialogHeader>

        <div className="space-y-4">
          {/* Config selector */}
          <div className="space-y-1.5">
            <label className="text-xs text-muted-foreground">Endpoint</label>
            <Select value={selectedConfigId} onValueChange={onConfigChange}>
              <SelectTrigger>
                <SelectValue placeholder="Select endpoint" />
              </SelectTrigger>
              <SelectContent>
                {configs.map((c) => (
                  <SelectItem key={c.id} value={c.id}>
                    {c.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            {selectedConfig && (
              <p className="text-[10px] text-muted-foreground truncate">
                {selectedConfig.method} {selectedConfig.url}
              </p>
            )}
          </div>

          {/* Items list or results */}
          {!result ? (
            <div className="space-y-2">
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <span>{items.length} 個項目</span>
                {exportedCount > 0 && (
                  <span className="text-[10px] px-1.5 py-0.5 rounded bg-muted">
                    {exportedCount} 已匯出・{newCount} 新項目
                  </span>
                )}
              </div>
              <ScrollArea className="h-48 border rounded-md">
                <div className="p-2 space-y-1">
                  {items.map((item) => {
                    const isExported = exportedIds?.has(item.id)
                    return (
                      <div
                        key={item.id}
                        className={`flex items-center justify-between py-1.5 px-2 text-xs rounded hover:bg-muted/50 ${isExported ? 'opacity-50' : ''}`}
                      >
                        <div className="flex items-center gap-1.5 truncate flex-1 mr-2">
                          {isExported && (
                            <CheckCircle2 className="w-3 h-3 text-muted-foreground shrink-0" />
                          )}
                          <span className="truncate">{item.title}</span>
                        </div>
                        <div className="flex items-center gap-2 text-muted-foreground shrink-0">
                          <span>{item.hours}h</span>
                          <span>{item.date}</span>
                        </div>
                      </div>
                    )
                  })}
                </div>
              </ScrollArea>
            </div>
          ) : (
            <div className="space-y-2">
              <div className="flex items-center gap-3 text-sm">
                <span className="text-muted-foreground">
                  {result.dry_run ? 'Preview' : 'Result'}:
                </span>
                <span className="text-green-600">{result.successful} ok</span>
                {result.failed > 0 && (
                  <span className="text-red-600">{result.failed} failed</span>
                )}
              </div>
              <ScrollArea className="h-48 border rounded-md">
                <div className="p-2 space-y-1">
                  {result.results.map((r, i) => (
                    <div
                      key={i}
                      className="flex items-center gap-2 py-1.5 px-2 text-xs rounded hover:bg-muted/50"
                    >
                      {r.status === 'success' || r.status === 'dry_run' ? (
                        <CheckCircle2 className="w-3.5 h-3.5 text-green-500 shrink-0" />
                      ) : (
                        <XCircle className="w-3.5 h-3.5 text-red-500 shrink-0" />
                      )}
                      <span className="truncate flex-1">{r.work_item_title}</span>
                      {r.http_status && (
                        <span className="text-muted-foreground shrink-0">
                          HTTP {r.http_status}
                        </span>
                      )}
                      {r.error_message && (
                        <span
                          className="text-red-500 truncate max-w-[200px]"
                          title={r.error_message}
                        >
                          {r.error_message}
                        </span>
                      )}
                    </div>
                  ))}
                </div>
              </ScrollArea>
            </div>
          )}
        </div>

        <DialogFooter>
          {!result ? (
            <>
              <Button variant="outline" size="sm" onClick={() => onExport(true)} disabled={exporting}>
                {exporting ? (
                  <Loader2 className="w-3.5 h-3.5 mr-1 animate-spin" />
                ) : (
                  <Eye className="w-3.5 h-3.5 mr-1" strokeWidth={1.5} />
                )}
                Preview
              </Button>
              <Button variant="outline" size="sm" onClick={onClose}>
                Cancel
              </Button>
              {exportedCount > 0 && newCount === 0 ? (
                <Button
                  size="sm"
                  onClick={() => onExport(false, true)}
                  disabled={exporting || !selectedConfigId}
                >
                  {exporting && <Loader2 className="w-3.5 h-3.5 mr-1 animate-spin" />}
                  全部重新匯出
                </Button>
              ) : (
                <Button
                  size="sm"
                  onClick={() => onExport(false)}
                  disabled={exporting || !selectedConfigId || newCount === 0}
                >
                  {exporting && <Loader2 className="w-3.5 h-3.5 mr-1 animate-spin" />}
                  匯出{exportedCount > 0 ? ` ${newCount} 個新項目` : ''}
                </Button>
              )}
            </>
          ) : (
            <>
              {result.dry_run && (
                <Button
                  size="sm"
                  onClick={() => onExport(false)}
                  disabled={exporting}
                >
                  {exporting && <Loader2 className="w-3.5 h-3.5 mr-1 animate-spin" />}
                  確認匯出
                </Button>
              )}
              <Button variant="outline" size="sm" onClick={onClose}>
                Close
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
