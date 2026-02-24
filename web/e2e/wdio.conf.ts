import { spawn, type ChildProcess } from 'node:child_process'
import { platform } from 'node:os'
import { resolve } from 'node:path'

let tauriDriver: ChildProcess | undefined

// WebdriverIO config for Tauri E2E testing via tauri-driver
// Type assertion needed because tauri:options is not in standard WDIO types
export const config = {
  specs: ['./e2e/specs/**/*.e2e.ts'],
  exclude: [],

  maxInstances: 1,

  capabilities: [
    {
      browserName: 'wry',
      'tauri:options': {
        application: getBinaryPath(),
      },
    },
  ],

  logLevel: 'warn' as const,
  bail: 0,

  waitforTimeout: 10000,
  connectionRetryTimeout: 120000,
  connectionRetryCount: 3,

  port: 4444,

  framework: 'mocha' as const,
  reporters: ['spec' as const],

  mochaOpts: {
    ui: 'bdd' as const,
    timeout: 30000,
  },

  onPrepare: function () {
    tauriDriver = spawn('tauri-driver', [], {
      stdio: ['ignore', 'pipe', 'pipe'],
    })

    // Wait for tauri-driver to start
    return new Promise<void>((resolve, reject) => {
      const timeout = setTimeout(() => {
        resolve() // Proceed anyway after timeout
      }, 5000)

      tauriDriver!.stderr?.on('data', (data: Buffer) => {
        const output = data.toString()
        if (output.includes('listening')) {
          clearTimeout(timeout)
          resolve()
        }
      })

      tauriDriver!.on('error', (err: Error) => {
        clearTimeout(timeout)
        reject(new Error(`Failed to start tauri-driver: ${err.message}`))
      })
    })
  },

  onComplete: function () {
    if (tauriDriver) {
      tauriDriver.kill()
      tauriDriver = undefined
    }
  },
}

function getBinaryPath(): string {
  const os = platform()

  if (os === 'linux') {
    return resolve(
      __dirname,
      'src-tauri/target/debug/recap'
    )
  }

  if (os === 'win32') {
    return resolve(
      __dirname,
      'src-tauri/target/debug/recap.exe'
    )
  }

  // macOS — tauri-driver doesn't support macOS, but keep path for reference
  return resolve(
    __dirname,
    'src-tauri/target/debug/bundle/macos/Recap.app/Contents/MacOS/Recap'
  )
}
