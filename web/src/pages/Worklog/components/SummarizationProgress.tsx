import { Loader2, Sparkles } from 'lucide-react'

interface SummarizationProgressProps {
  log: string[]
  active: boolean
}

export function SummarizationProgress({ log, active }: SummarizationProgressProps) {
  if (log.length === 0) return null

  return (
    <div className="rounded-md p-3 text-xs bg-amber-50 text-amber-800 border border-amber-200 space-y-1 max-h-32 overflow-y-auto">
      <div className="flex items-center gap-1.5 font-medium">
        <Sparkles className="w-3.5 h-3.5" />
        LLM Processing
      </div>
      {log.map((msg, i) => (
        <div key={i} className="flex items-center gap-1.5">
          {i === log.length - 1 && active && (
            <Loader2 className="w-3 h-3 animate-spin shrink-0" />
          )}
          <span>{msg}</span>
        </div>
      ))}
    </div>
  )
}
