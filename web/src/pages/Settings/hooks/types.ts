export type SettingsSection = 'profile' | 'projects' | 'sync' | 'export' | 'ai' | 'about' | 'advanced'

export interface SettingsMessage {
  type: 'success' | 'error'
  text: string
}
