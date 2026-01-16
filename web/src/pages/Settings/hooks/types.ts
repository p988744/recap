export type SettingsSection = 'profile' | 'account' | 'integrations' | 'ai' | 'preferences' | 'about'

export interface SettingsMessage {
  type: 'success' | 'error'
  text: string
}
