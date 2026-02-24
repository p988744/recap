import { useEffect, useRef, useCallback } from 'react'
import { AlertCircle, RefreshCw } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useProjectTimeline } from '../../hooks/useProjectTimeline'
import { useTimelineSummaries } from '../../hooks/useTimelineSummaries'
import { TimelineControls, TimelineGroupComponent } from '../Timeline'

interface TimelineTabProps {
  projectName: string
}

export function TimelineTab({ projectName }: TimelineTabProps) {
  const {
    groups,
    isLoading,
    isLoadingMore,
    error,
    hasMore,
    timeUnit,
    sources,
    setTimeUnit,
    setSources,
    loadMore,
    refetch,
  } = useProjectTimeline({ projectName })

  // Fetch cached summaries (read-only from DB)
  const { summaries } = useTimelineSummaries({ projectName, timeUnit, groups })

  // Infinite scroll observer
  const observerRef = useRef<IntersectionObserver | null>(null)
  const loadMoreRef = useRef<HTMLDivElement | null>(null)

  const handleObserver = useCallback(
    (entries: IntersectionObserverEntry[]) => {
      const [entry] = entries
      if (entry.isIntersecting && hasMore && !isLoadingMore) {
        loadMore()
      }
    },
    [hasMore, isLoadingMore, loadMore]
  )

  useEffect(() => {
    if (observerRef.current) {
      observerRef.current.disconnect()
    }

    observerRef.current = new IntersectionObserver(handleObserver, {
      root: null,
      rootMargin: '100px',
      threshold: 0,
    })

    if (loadMoreRef.current) {
      observerRef.current.observe(loadMoreRef.current)
    }

    return () => {
      if (observerRef.current) {
        observerRef.current.disconnect()
      }
    }
  }, [handleObserver])

  // Get available sources from the data
  const availableSources = Array.from(
    new Set(
      groups.flatMap((g) => g.sessions.map((s) => s.source))
    )
  )

  // Loading state
  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-16">
        <div className="w-6 h-6 border border-border border-t-foreground/60 rounded-full animate-spin" />
      </div>
    )
  }

  // Error state
  if (error) {
    return (
      <div className="flex flex-col items-center justify-center py-16 gap-4">
        <AlertCircle className="w-10 h-10 text-destructive" />
        <p className="text-sm text-destructive">{error}</p>
        <Button variant="outline" size="sm" onClick={refetch}>
          <RefreshCw className="w-4 h-4 mr-2" />
          Retry
        </Button>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      {/* Controls */}
      <div className="flex items-center justify-between">
        <TimelineControls
          timeUnit={timeUnit}
          onTimeUnitChange={setTimeUnit}
          sources={sources}
          onSourcesChange={setSources}
          availableSources={availableSources.length > 0 ? availableSources : ['claude_code']}
        />

        <Button
          variant="ghost"
          size="sm"
          onClick={refetch}
          className="text-muted-foreground hover:text-foreground"
        >
          <RefreshCw className="w-4 h-4" />
        </Button>
      </div>

      {/* Timeline groups */}
      {groups.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-16 text-center">
          <p className="text-muted-foreground">No timeline data available</p>
          <p className="text-sm text-muted-foreground/60 mt-1">
            Start working on this project to see your activity timeline
          </p>
        </div>
      ) : (
        <div className="space-y-10">
          {groups.map((group) => (
            <TimelineGroupComponent
              key={group.period_label}
              group={group}
              projectName={projectName}
              summary={summaries[group.period_start] ?? null}
            />
          ))}
        </div>
      )}

      {/* Load more trigger */}
      <div ref={loadMoreRef} className="h-8">
        {isLoadingMore && (
          <div className="flex items-center justify-center py-4">
            <div className="w-5 h-5 border border-border border-t-foreground/60 rounded-full animate-spin" />
          </div>
        )}
      </div>

      {/* End of list indicator */}
      {!hasMore && groups.length > 0 && (
        <div className="text-center py-4">
          <span className="text-sm text-muted-foreground/50">
            End of timeline
          </span>
        </div>
      )}
    </div>
  )
}
