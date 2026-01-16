export type SettingsSection = 'profile' | 'account' | 'integrations' | 'sync' | 'ai' | 'preferences' | 'about'

export interface SettingsMessage {
  type: 'success' | 'error'
  text: string
}
