/**
 * CostSkeleton Component
 *
 * Skeleton placeholder for the cost summary section while loading.
 */

import { Card, CardContent, CardHeader } from '@/components/ui/card'
import { Skeleton } from '@/components/ui/skeleton'
import { DollarSign } from 'lucide-react'

export function CostSkeleton() {
  return (
    <section className="animate-fade-up opacity-0 delay-2">
      <div className="flex items-center gap-2 mb-4">
        <DollarSign
          className="w-4 h-4 text-muted-foreground"
          strokeWidth={1.5}
        />
        <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
          費用統計
        </h2>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        {/* Cost Card Skeleton */}
        <Card>
          <CardContent className="pt-6 space-y-4">
            {/* Today/30 days stats */}
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Skeleton className="h-3 w-16" />
                <Skeleton className="h-8 w-24" />
                <Skeleton className="h-3 w-20" />
              </div>
              <div className="space-y-2">
                <Skeleton className="h-3 w-16" />
                <Skeleton className="h-8 w-24" />
                <Skeleton className="h-3 w-20" />
              </div>
            </div>

            {/* Model breakdown */}
            <div className="space-y-3 pt-4 border-t">
              <Skeleton className="h-3 w-24" />
              <div className="space-y-2">
                <div className="flex justify-between">
                  <Skeleton className="h-4 w-32" />
                  <Skeleton className="h-4 w-16" />
                </div>
                <div className="flex justify-between">
                  <Skeleton className="h-4 w-28" />
                  <Skeleton className="h-4 w-16" />
                </div>
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Cost Chart Skeleton */}
        <Card>
          <CardHeader className="pb-2">
            <Skeleton className="h-3 w-32" />
          </CardHeader>
          <CardContent>
            <div className="h-[200px] flex items-end gap-1">
              {/* Bar chart skeleton */}
              {Array.from({ length: 15 }).map((_, i) => (
                <Skeleton
                  key={i}
                  className="flex-1"
                  style={{ height: `${20 + Math.random() * 60}%` }}
                />
              ))}
            </div>
          </CardContent>
        </Card>
      </div>
    </section>
  )
}

export default CostSkeleton
