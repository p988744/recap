/**
 * HistorySkeleton Component
 *
 * Skeleton placeholder for the usage history section while loading.
 */

import { Card, CardContent, CardHeader } from '@/components/ui/card'
import { Skeleton } from '@/components/ui/skeleton'
import { History } from 'lucide-react'

export function HistorySkeleton() {
  return (
    <section className="animate-fade-up opacity-0 delay-3">
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between flex-wrap gap-4">
            <div className="flex items-center gap-2">
              <History
                className="w-4 h-4 text-muted-foreground"
                strokeWidth={1.5}
              />
              <span className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground font-normal">
                使用歷史
              </span>
            </div>
            <Skeleton className="h-8 w-28" />
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* QuotaStats skeleton */}
          <div className="space-y-3">
            {/* Header row */}
            <div className="flex items-center justify-between py-2 border-b">
              <Skeleton className="h-4 w-16" />
              <div className="flex gap-8">
                <Skeleton className="h-4 w-12" />
                <Skeleton className="h-4 w-12" />
                <Skeleton className="h-4 w-12" />
              </div>
            </div>
            {/* Data rows */}
            {[1, 2, 3].map((i) => (
              <div key={i} className="flex items-center justify-between py-2">
                <div className="flex items-center gap-2">
                  <Skeleton className="h-4 w-4 rounded-full" />
                  <Skeleton className="h-4 w-20" />
                </div>
                <div className="flex gap-8">
                  <Skeleton className="h-5 w-14" />
                  <Skeleton className="h-4 w-12" />
                  <Skeleton className="h-4 w-12" />
                </div>
              </div>
            ))}
          </div>

          {/* Chart skeleton */}
          <div className="h-[300px] pt-4">
            <div className="h-full flex flex-col">
              {/* Y-axis labels */}
              <div className="flex-1 flex">
                <div className="w-12 flex flex-col justify-between text-right pr-2">
                  <Skeleton className="h-3 w-8 ml-auto" />
                  <Skeleton className="h-3 w-8 ml-auto" />
                  <Skeleton className="h-3 w-8 ml-auto" />
                  <Skeleton className="h-3 w-8 ml-auto" />
                </div>
                {/* Chart area */}
                <div className="flex-1 border-l border-b relative">
                  {/* Horizontal grid lines */}
                  <div className="absolute inset-0 flex flex-col justify-between">
                    {[1, 2, 3, 4].map((i) => (
                      <div key={i} className="border-t border-dashed border-muted" />
                    ))}
                  </div>
                  {/* Animated line placeholder */}
                  <div className="absolute bottom-1/3 left-0 right-0 h-0.5">
                    <Skeleton className="h-full w-full" />
                  </div>
                </div>
              </div>
              {/* X-axis labels */}
              <div className="h-6 flex ml-12">
                <div className="flex-1 flex justify-between pt-2">
                  {[1, 2, 3, 4, 5].map((i) => (
                    <Skeleton key={i} className="h-3 w-8" />
                  ))}
                </div>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>
    </section>
  )
}

export default HistorySkeleton
