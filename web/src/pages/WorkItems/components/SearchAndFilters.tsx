import { Search, Filter } from 'lucide-react'
import { Card, CardContent } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import type { WorkItemFilters } from '@/types'

interface SearchAndFiltersProps {
  searchTerm: string
  setSearchTerm: (value: string) => void
  showFilters: boolean
  setShowFilters: (value: boolean) => void
  filters: WorkItemFilters
  setFilters: (filters: WorkItemFilters) => void
  setPage: (page: number) => void
  onSearch: (e: React.FormEvent) => void
  onClearFilters: () => void
}

export function SearchAndFilters({
  searchTerm,
  setSearchTerm,
  showFilters,
  setShowFilters,
  filters,
  setFilters,
  setPage,
  onSearch,
  onClearFilters,
}: SearchAndFiltersProps) {
  return (
    <>
      {/* Search Bar */}
      <section className="flex items-center gap-4 animate-fade-up opacity-0 delay-3">
        <form onSubmit={onSearch} className="flex-1 flex gap-2">
          <div className="relative flex-1">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" strokeWidth={1.5} />
            <Input
              placeholder="搜尋工作項目..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="pl-10"
            />
          </div>
          <Button type="submit" variant="outline">
            搜尋
          </Button>
        </form>
        <Button
          variant="ghost"
          onClick={() => setShowFilters(!showFilters)}
          className={showFilters ? 'bg-accent/10' : ''}
        >
          <Filter className="w-4 h-4 mr-2" strokeWidth={1.5} />
          篩選
        </Button>
      </section>

      {/* Filter Panel */}
      {showFilters && (
        <section className="animate-fade-up">
          <Card>
            <CardContent className="p-6">
              <div className="grid grid-cols-4 gap-4">
                <div className="space-y-2">
                  <Label className="text-xs">來源</Label>
                  <Select
                    value={filters.source || 'all'}
                    onValueChange={(value) => {
                      setPage(1)
                      setFilters({ ...filters, source: value === 'all' ? undefined : value })
                    }}
                  >
                    <SelectTrigger>
                      <SelectValue placeholder="全部" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="all">全部</SelectItem>
                      <SelectItem value="claude_code">Claude Code</SelectItem>
                      <SelectItem value="gitlab">GitLab</SelectItem>
                      <SelectItem value="manual">手動</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                <div className="space-y-2">
                  <Label className="text-xs">Jira 對應</Label>
                  <Select
                    value={filters.jira_mapped === undefined ? 'all' : filters.jira_mapped ? 'true' : 'false'}
                    onValueChange={(value) => {
                      setPage(1)
                      setFilters({
                        ...filters,
                        jira_mapped: value === 'all' ? undefined : value === 'true',
                      })
                    }}
                  >
                    <SelectTrigger>
                      <SelectValue placeholder="全部" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="all">全部</SelectItem>
                      <SelectItem value="true">已對應</SelectItem>
                      <SelectItem value="false">未對應</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                <div className="space-y-2">
                  <Label className="text-xs">Tempo 同步</Label>
                  <Select
                    value={filters.synced_to_tempo === undefined ? 'all' : filters.synced_to_tempo ? 'true' : 'false'}
                    onValueChange={(value) => {
                      setPage(1)
                      setFilters({
                        ...filters,
                        synced_to_tempo: value === 'all' ? undefined : value === 'true',
                      })
                    }}
                  >
                    <SelectTrigger>
                      <SelectValue placeholder="全部" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="all">全部</SelectItem>
                      <SelectItem value="true">已同步</SelectItem>
                      <SelectItem value="false">未同步</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                <div className="flex items-end">
                  <Button variant="ghost" size="sm" onClick={onClearFilters}>
                    清除篩選
                  </Button>
                </div>
              </div>
            </CardContent>
          </Card>
        </section>
      )}
    </>
  )
}
