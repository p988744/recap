export type SettingsSection = 'profile' | 'projects' | 'sync' | 'export' | 'ai' | 'about'

export interface SettingsMessage {
  type: 'success' | 'error'
  text: string
}
