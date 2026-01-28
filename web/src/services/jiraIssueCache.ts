/**
 * Global Jira issue detail cache with 15-minute TTL.
 *
 * Stores issue details (summary, description, assignee, issue_type) keyed by
 * issue key. Populated by:
 *   - prefetch() — batch-loads details for multiple keys via one API call
 *   - set()     — called by JiraBadge / validation modals after individual fetch
 *
 * Read by:
 *   - JiraBadge — reads on mount to show title inline without hover-fetch
 *   - Tooltip   — reads cached details for hover popup
 */

import { tempo } from '@/services'

const TTL_MS = 15 * 60 * 1000 // 15 minutes

export interface CachedIssueDetail {
  summary: string
  description?: string
  assignee?: string
  issueType?: string
}

interface CacheEntry {
  detail: CachedIssueDetail
  expiresAt: number
}

const cache = new Map<string, CacheEntry>()

/** Set of keys currently being fetched (dedup in-flight requests) */
let prefetchingKeys = new Set<string>()

function isValid(entry: CacheEntry): boolean {
  return Date.now() < entry.expiresAt
}

export function get(key: string): CachedIssueDetail | undefined {
  const entry = cache.get(key)
  if (!entry) return undefined
  if (!isValid(entry)) {
    cache.delete(key)
    return undefined
  }
  return entry.detail
}

export function set(key: string, detail: CachedIssueDetail): void {
  cache.set(key, { detail, expiresAt: Date.now() + TTL_MS })
}

export function has(key: string): boolean {
  const entry = cache.get(key)
  if (!entry) return false
  if (!isValid(entry)) {
    cache.delete(key)
    return false
  }
  return true
}

/**
 * Batch-prefetch issue details for the given keys.
 * Skips keys already cached (and not expired) or currently in-flight.
 * Safe to call multiple times — deduplicates automatically.
 */
export async function prefetch(keys: string[]): Promise<void> {
  const missing = keys.filter((k) => k && !has(k) && !prefetchingKeys.has(k))
  if (missing.length === 0) return

  for (const k of missing) prefetchingKeys.add(k)

  try {
    const details = await tempo.batchGetIssues(missing)
    const now = Date.now()
    for (const d of details) {
      cache.set(d.key, {
        detail: {
          summary: d.summary,
          description: d.description,
          assignee: d.assignee,
          issueType: d.issue_type,
        },
        expiresAt: now + TTL_MS,
      })
    }
  } catch {
    // Prefetch is best-effort; individual fetches will retry on hover
  } finally {
    for (const k of missing) prefetchingKeys.delete(k)
  }
}

/** Listeners notified when cache is updated (for re-render) */
type Listener = () => void
const listeners = new Set<Listener>()

export function subscribe(listener: Listener): () => void {
  listeners.add(listener)
  return () => listeners.delete(listener)
}

/** Prefetch and notify listeners when done */
export async function prefetchAndNotify(keys: string[]): Promise<void> {
  const sizeBefore = cache.size
  await prefetch(keys)
  if (cache.size !== sizeBefore) {
    for (const fn of listeners) fn()
  }
}
