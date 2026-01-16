import { FolderGit2, Terminal, Link2, GitBranch } from 'lucide-react'
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs'
import { GitRepoCardV2 } from './GitRepoCardV2'
import { ClaudeCodeCardV2 } from './ClaudeCodeCardV2'
import { JiraTempoCardV2 } from './JiraTempoCardV2'
import { GitLabCardV2 } from './GitLabCardV2'

const integrationTabs = [
  { id: 'git', label: 'Git', icon: FolderGit2 },
  { id: 'claude', label: 'Claude', icon: Terminal },
  { id: 'jira', label: 'Jira', icon: Link2 },
  { id: 'gitlab', label: 'GitLab', icon: GitBranch },
]

export function IntegrationsSectionV2() {
  return (
    <section className="space-y-6 animate-fade-up opacity-0 delay-1">
      <h2 className="font-display text-2xl text-foreground">整合服務</h2>

      <Tabs defaultValue="git" className="w-full">
        <TabsList className="w-full justify-start rounded-lg">
          {integrationTabs.map((tab) => (
            <TabsTrigger
              key={tab.id}
              value={tab.id}
              className="flex items-center gap-2 rounded-md"
            >
              <tab.icon className="w-4 h-4" strokeWidth={1.5} />
              {tab.label}
            </TabsTrigger>
          ))}
        </TabsList>

        <TabsContent value="git">
          <GitRepoCardV2 />
        </TabsContent>

        <TabsContent value="claude">
          <ClaudeCodeCardV2 />
        </TabsContent>

        <TabsContent value="jira">
          <JiraTempoCardV2 />
        </TabsContent>

        <TabsContent value="gitlab">
          <GitLabCardV2 />
        </TabsContent>
      </Tabs>
    </section>
  )
}
