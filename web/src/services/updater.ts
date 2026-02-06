/**
 * App updater service — thin wrapper around @tauri-apps/plugin-updater.
 */
import { check, type Update } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'

export type { Update }

export interface DownloadProgress {
  contentLength: number | undefined
  downloaded: number
}

/**
 * Check for a new version from the configured update endpoint.
 * Returns the Update object if available, or null if already up-to-date.
 */
export async function checkForUpdate(): Promise<Update | null> {
  const update = await check()
  return update ?? null
}

/**
 * Download and install an available update, reporting progress via callback.
 */
export async function downloadAndInstall(
  update: Update,
  onProgress?: (progress: DownloadProgress) => void,
): Promise<void> {
  let downloaded = 0
  await update.downloadAndInstall((event) => {
    if (event.event === 'Started') {
      onProgress?.({ contentLength: event.data.contentLength ?? undefined, downloaded: 0 })
    } else if (event.event === 'Progress') {
      downloaded += event.data.chunkLength
      onProgress?.({ contentLength: undefined, downloaded })
    } else if (event.event === 'Finished') {
      onProgress?.({ contentLength: undefined, downloaded })
    }
  })
}

/**
 * Relaunch the app after an update has been installed.
 */
export async function relaunchApp(): Promise<void> {
  await relaunch()
}
