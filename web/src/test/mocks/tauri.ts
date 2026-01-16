import { vi } from 'vitest'

// Mock responses for different Tauri commands
type CommandHandler = (args?: Record<string, unknown>) => unknown

const commandHandlers: Record<string, CommandHandler> = {}

/**
 * Mock implementation of Tauri's invoke function
 */
export const mockInvoke = vi.fn(async (command: string, args?: Record<string, unknown>) => {
  const handler = commandHandlers[command]
  if (handler) {
    return handler(args)
  }
  throw new Error(`No mock handler for command: ${command}`)
})

/**
 * Register a mock handler for a specific command
 */
export function mockCommand(command: string, handler: CommandHandler) {
  commandHandlers[command] = handler
}

/**
 * Register a mock handler that returns a value
 */
export function mockCommandValue<T>(command: string, value: T) {
  commandHandlers[command] = () => value
}

/**
 * Register a mock handler that throws an error
 */
export function mockCommandError(command: string, error: string) {
  commandHandlers[command] = () => {
    throw new Error(error)
  }
}

/**
 * Clear all command handlers
 */
export function clearCommandHandlers() {
  Object.keys(commandHandlers).forEach((key) => {
    delete commandHandlers[key]
  })
}

/**
 * Reset the mock invoke function and clear handlers
 */
export function resetTauriMock() {
  mockInvoke.mockClear()
  clearCommandHandlers()
}

// Mock the @tauri-apps/api/core module
vi.mock('@tauri-apps/api/core', () => ({
  invoke: mockInvoke,
}))

export { commandHandlers }
