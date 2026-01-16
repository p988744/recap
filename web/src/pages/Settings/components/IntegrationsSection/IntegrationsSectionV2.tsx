import { GitRepoCardV2 } from './GitRepoCardV2'
import { ClaudeCodeCardV2 } from './ClaudeCodeCardV2'
import { JiraTempoCardV2 } from './JiraTempoCardV2'
import { GitLabCardV2 } from './GitLabCardV2'

export function IntegrationsSectionV2() {
  return (
    <section className="space-y-8 animate-fade-up opacity-0 delay-1">
      <h2 className="font-display text-2xl text-foreground">整合服務</h2>

      <GitRepoCardV2 />
      <ClaudeCodeCardV2 />
      <JiraTempoCardV2 />
      <GitLabCardV2 />
    </section>
  )
}
