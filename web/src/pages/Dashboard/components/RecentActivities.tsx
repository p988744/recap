import { Link } from 'react-router-dom'
import { ArrowRight, GitCommit, FileCode } from 'lucide-react'
import { formatDate, cn } from '@/lib/utils'
import { MarkdownSummary } from '@/components/MarkdownSummary'

interface WorklogActivity {
  projectName: string
  dailySummary?: string
  date: string
  totalHours: number
  totalCommits: number
  totalFiles: number
}

interface RecentActivitiesProps {
  activities: WorklogActivity[]
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
              key={`${activity.projectName}-${activity.date}-${index}`}
              className={cn(
                "py-4 border-b border-border last:border-b-0",
                "animate-fade-up opacity-0",
                index === 0 && "delay-4",
                index === 1 && "delay-5",
                index === 2 && "delay-6"
              )}
            >
              {/* Header: project name + hours + date */}
              <div className="flex items-start justify-between gap-4 mb-1">
                <div className="flex items-center gap-3 min-w-0">
                  <span className="text-sm font-medium text-foreground truncate">{activity.projectName}</span>
                  <span className="text-xs text-muted-foreground tabular-nums flex-shrink-0">{activity.totalHours.toFixed(1)}h</span>
                </div>
                <span className="text-xs text-muted-foreground tabular-nums flex-shrink-0">
                  {formatDate(activity.date)}
                </span>
              </div>

              {/* Daily summary rendered as markdown */}
              {activity.dailySummary && (
                <MarkdownSummary content={activity.dailySummary} className="mb-1.5" />
              )}

              {/* Stats: commits & files */}
              <div className="flex items-center gap-3">
                {activity.totalCommits > 0 && (
                  <span className="flex items-center gap-1 text-xs text-muted-foreground">
                    <GitCommit className="w-3 h-3" strokeWidth={1.5} />
                    {activity.totalCommits} commits
                  </span>
                )}
                {activity.totalFiles > 0 && (
                  <span className="flex items-center gap-1 text-xs text-muted-foreground">
                    <FileCode className="w-3 h-3" strokeWidth={1.5} />
                    {activity.totalFiles} 檔案
                  </span>
                )}
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
