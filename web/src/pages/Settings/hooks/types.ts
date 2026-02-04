export type SettingsSection = 'profile' | 'projects' | 'sync' | 'export' | 'ai' | 'about' | 'danger'

export interface SettingsMessage {
  type: 'success' | 'error'
  text: string
}
