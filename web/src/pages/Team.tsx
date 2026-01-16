import { useEffect, useState } from 'react'
import {
  Users,
  UserPlus,
  Download,
  RefreshCw,
  Clock,
  Mail,
  MoreHorizontal,
} from 'lucide-react'
import { Card } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { teams as teamsService } from '@/services'
import type { Team } from '@/types'
import { cn } from '@/lib/utils'

export function TeamPage() {
  const [teams, setTeams] = useState<Team[]>([])
  const [loading, setLoading] = useState(true)
  const [selectedTeam, setSelectedTeam] = useState<string | null>(null)

  useEffect(() => {
    async function fetchTeams() {
      try {
        const response = await teamsService.getTeams()
        setTeams(response.teams)
        if (response.teams.length > 0) {
          setSelectedTeam(response.teams[0].name)
        }
      } catch (err) {
        console.error('Failed to fetch teams:', err)
      } finally {
        setLoading(false)
      }
    }
    fetchTeams()
  }, [])

  const currentTeam = teams.find((t) => t.name === selectedTeam)

  if (loading) {
    return (
      <div className="flex items-center justify-center h-96">
        <div className="w-6 h-6 border border-border border-t-charcoal/60 rounded-full animate-spin" />
      </div>
    )
  }

  return (
    <div className="space-y-12">
      {/* Header */}
      <header className="flex items-start justify-between animate-fade-up opacity-0 delay-1">
        <div>
          <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground mb-2">
            管理
          </p>
          <h1 className="font-display text-4xl text-foreground tracking-tight">團隊管理</h1>
        </div>
        <Button variant="outline">
          <UserPlus className="w-4 h-4" strokeWidth={1.5} />
          新增團隊
        </Button>
      </header>

      {/* Team Selector */}
      {teams.length > 0 && (
        <section className="animate-fade-up opacity-0 delay-2">
          <div className="flex gap-2 flex-wrap">
            {teams.map((team) => (
              <button
                key={team.name}
                onClick={() => setSelectedTeam(team.name)}
                className={cn(
                  "px-4 py-2 text-sm transition-all",
                  selectedTeam === team.name
                    ? "bg-foreground text-cream"
                    : "bg-white/50 text-muted-foreground hover:text-foreground border border-border"
                )}
              >
                {team.name}
                <span className="ml-2 text-xs opacity-70">({team.member_count})</span>
              </button>
            ))}
          </div>
        </section>
      )}

      {/* Team Info */}
      {currentTeam && (
        <>
          <section className="animate-fade-up opacity-0 delay-2">
            <Card accent className="p-8">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-4">
                  <div className="w-12 h-12 bg-foreground/5 flex items-center justify-center">
                    <Users className="w-5 h-5 text-muted-foreground" strokeWidth={1.5} />
                  </div>
                  <div>
                    <h2 className="font-display text-2xl text-foreground">{currentTeam.name}</h2>
                    <p className="text-sm text-muted-foreground">
                      {currentTeam.tempo_team_id
                        ? `Tempo Team #${currentTeam.tempo_team_id}`
                        : currentTeam.jira_group || '自訂團隊'}
                    </p>
                  </div>
                </div>
                <div className="flex gap-2">
                  <Button variant="ghost" size="sm">
                    <RefreshCw className="w-4 h-4" strokeWidth={1.5} />
                    同步成員
                  </Button>
                  <Button variant="outline" size="sm">
                    <Download className="w-4 h-4" strokeWidth={1.5} />
                    匯出報表
                  </Button>
                </div>
              </div>
              {currentTeam.last_synced && (
                <p className="mt-4 text-xs text-muted-foreground flex items-center gap-1">
                  <Clock className="w-3 h-3" strokeWidth={1.5} />
                  上次同步：{currentTeam.last_synced}
                </p>
              )}
            </Card>
          </section>

          {/* Members Table */}
          <section className="animate-fade-up opacity-0 delay-3">
            <div className="flex items-center gap-2 mb-6">
              <Users className="w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
              <p className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
                團隊成員
              </p>
              <span className="text-xs text-muted-foreground ml-2">
                {currentTeam.member_count} 位成員
              </span>
            </div>

            {currentTeam.members && currentTeam.members.length > 0 ? (
              <Card className="overflow-hidden">
                <table className="w-full">
                  <thead>
                    <tr className="border-b border-border">
                      <th className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground text-left py-4 px-6 font-medium">
                        成員
                      </th>
                      <th className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground text-left py-4 px-6 font-medium">
                        Email
                      </th>
                      <th className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground text-right py-4 px-6 font-medium">
                        本週工時
                      </th>
                      <th className="text-[10px] uppercase tracking-[0.2em] text-muted-foreground text-right py-4 px-6 font-medium">
                        操作
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    {currentTeam.members.map((member, index) => (
                      <tr
                        key={member.account_id}
                        className={cn(
                          "border-b border-border last:border-b-0 hover:bg-foreground/[0.02] transition-colors",
                          "animate-fade-up opacity-0",
                          index === 0 && "delay-4",
                          index === 1 && "delay-5",
                          index === 2 && "delay-6"
                        )}
                      >
                        <td className="py-4 px-6">
                          <div className="flex items-center gap-3">
                            <div className="w-8 h-8 bg-foreground text-cream flex items-center justify-center text-sm font-medium">
                              {member.display_name.charAt(0)}
                            </div>
                            <span className="text-sm text-foreground">{member.display_name}</span>
                          </div>
                        </td>
                        <td className="py-4 px-6">
                          <span className="text-sm text-muted-foreground flex items-center gap-1">
                            <Mail className="w-3.5 h-3.5" strokeWidth={1.5} />
                            {member.email || '-'}
                          </span>
                        </td>
                        <td className="py-4 px-6 text-right">
                          <span className="text-sm text-foreground tabular-nums">-</span>
                          <span className="text-xs text-muted-foreground ml-1">hrs</span>
                        </td>
                        <td className="py-4 px-6 text-right">
                          <button className="text-muted-foreground hover:text-foreground transition-colors p-1">
                            <MoreHorizontal className="w-4 h-4" strokeWidth={1.5} />
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </Card>
            ) : (
              <Card className="p-12">
                <div className="text-center text-muted-foreground">
                  <Users className="w-8 h-8 mx-auto mb-3 opacity-50" strokeWidth={1} />
                  <p className="text-sm mb-4">尚未同步成員資料</p>
                  <Button variant="outline" size="sm">
                    <RefreshCw className="w-4 h-4" strokeWidth={1.5} />
                    從 Tempo 同步
                  </Button>
                </div>
              </Card>
            )}
          </section>
        </>
      )}

      {/* Empty State */}
      {teams.length === 0 && (
        <section className="animate-fade-up opacity-0 delay-2">
          <Card className="p-16">
            <div className="text-center">
              <Users className="w-12 h-12 mx-auto mb-4 text-charcoal/20" strokeWidth={1} />
              <h3 className="font-display text-xl text-foreground mb-2">尚未設定團隊</h3>
              <p className="text-sm text-muted-foreground mb-6">
                新增團隊以追蹤成員的工時
              </p>
              <Button variant="outline">
                <UserPlus className="w-4 h-4" strokeWidth={1.5} />
                新增團隊
              </Button>
            </div>
          </Card>
        </section>
      )}
    </div>
  )
}
