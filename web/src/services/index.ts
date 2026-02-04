/**
 * Services - unified API layer
 *
 * This module provides a clean, organized API for all backend operations.
 * Import from '@/services' instead of directly from individual service files.
 */

// Re-export client utilities
export {
  getAuthToken,
  setAuthToken,
  removeAuthToken,
  getRequiredToken,
  invokeCommand,
  invokeAuth,
} from './client'

// Re-export domain services
export * as auth from './auth'
export * as config from './config'
export * as workItems from './work-items'
export * as reports from './reports'
export * as projects from './projects'
export * as sync from './sync'
export * as backgroundSync from './background-sync'
export * as worklog from './worklog'
export * as worklogSync from './worklog-sync'
export * as tray from './tray'
export * as notification from './notification'
export * as llmUsage from './llm-usage'
export * as quota from './quota'
export * as dangerZone from './danger-zone'
export * as batchCompaction from './batch-compaction'

// Re-export integrations
export * as gitlab from './integrations/gitlab'
export * as tempo from './integrations/tempo'
export * as claude from './integrations/claude'
export * as sources from './integrations/sources'
export * as teams from './integrations/teams'

// Namespace exports for grouping
export * as integrations from './integrations'
