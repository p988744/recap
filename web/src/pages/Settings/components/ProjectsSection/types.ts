import type { ReactNode } from 'react'

/** A discovered project from a data source (before sync) */
export interface DiscoveredProject {
  name: string
  path: string
  sessionCount: number
  sessions: DiscoveredSession[]
}

export interface DiscoveredSession {
  id: string
  summary: string
  timestamp?: string
  detail?: string
}

export interface SyncResult {
  sessionsProcessed: number
  sessionsSkipped: number
  workItemsCreated: number
  workItemsUpdated: number
}

/** Abstract data source adapter */
export interface DataSourceAdapter {
  /** Unique key for this source */
  key: string
  /** Display name */
  label: string
  /** Brand icon component */
  icon: ReactNode
  /** Tailwind text color class for this source */
  colorClass: string
  /** Tailwind bg color class for badges */
  badgeBgClass: string
  /** Check if this source is installed/available */
  checkInstalled(): Promise<boolean>
  /** Scan for projects (discovery) */
  scanProjects(): Promise<DiscoveredProject[]>
  /** Sync all discovered projects to work items */
  syncAll(): Promise<SyncResult>
}
