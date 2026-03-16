export interface StorageSecurityStatus {
  hasMasterPassword: boolean
  isUnlocked: boolean
  mode: 'keyring' | 'password'
  keyringAvailable: boolean
  keyringError: string | null
}
