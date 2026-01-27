export type SettingsSection = 'profile' | 'projects' | 'sync' | 'ai' | 'about'

export interface SettingsMessage {
  type: 'success' | 'error'
  text: string
}
