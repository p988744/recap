export type SettingsSection = 'profile' | 'account' | 'projects' | 'sync' | 'ai' | 'preferences' | 'about'

export interface SettingsMessage {
  type: 'success' | 'error'
  text: string
}
