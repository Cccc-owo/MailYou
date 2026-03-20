export interface StorageSecurityStatus {
  hasMasterPassword: boolean
  isUnlocked: boolean
  mode: 'keyring' | 'password'
  keyringAvailable: boolean
  keyringError: string | null
  hasRecoveryKeyBackup: boolean
  masterPasswordRecommended: boolean
}

export interface RecoveryExportStatus {
  exportDir: string
  latestExportedAt: string | null
  snapshotCount: number
}
