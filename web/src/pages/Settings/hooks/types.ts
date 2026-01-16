export type SettingsSection = 'profile' | 'account' | 'integrations' | 'preferences' | 'about'

export interface SettingsMessage {
  type: 'success' | 'error'
  text: string
}
