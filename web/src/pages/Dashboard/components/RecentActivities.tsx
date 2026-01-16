import { Link } from 'react-router-dom'
import { ArrowRight } from 'lucide-react'
import { formatDate, cn } from '@/lib/utils'

interface Activity {
  title: string
  source: string
  date: string
  hours: number
  jiraKey: string | null | undefined
}

interface RecentActivitiesProps {
  activities: Activity[]
}

export function RecentActivities({ activities }: RecentActivitiesProps) {
  return (
    <div className="col-span-3">
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
          最近活動
        </h2>
        <Link to="/work-items" className="text-xs text-muted-foreground hover:text-foreground transition-colors flex items-center gap-1">
          查看全部
          <ArrowRight className="w-3 h-3" strokeWidth={1.5} />
        </Link>
      </div>

      {activities.length > 0 ? (
        <div className="space-y-0">
          {activities.map((activity, index) => (
            <div
              key={`${activity.source}-${activity.date}-${index}`}
              className={cn(
                "py-4 border-b border-border last:border-b-0",
                "animate-fade-up opacity-0",
                index === 0 && "delay-4",
                index === 1 && "delay-5",
                index === 2 && "delay-6"
              )}
            >
              <div className="flex items-start justify-between gap-4">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-3 mb-1">
                    <span className="text-sm text-foreground line-clamp-1">{activity.title}</span>
                    <span className="text-xs text-muted-foreground tabular-nums">{activity.hours.toFixed(1)}h</span>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-xs text-muted-foreground">{activity.source}</span>
                    {activity.jiraKey && (
                      <span className="text-xs text-blue-600">{activity.jiraKey}</span>
                    )}
                  </div>
                </div>
                <span className="text-xs text-muted-foreground tabular-nums flex-shrink-0">
                  {formatDate(activity.date)}
                </span>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="h-44 flex items-center justify-center text-muted-foreground text-sm">
          暫無活動記錄
        </div>
      )}
    </div>
  )
}
