export type BackupFileName =
  | 'config.toml'
  | 'auth.json'
  | 'providers.json'
  | 'provider-preferences.json'
  | 'metadata.json'

export interface BackupMetadata {
  schemaVersion: number
  transactionId: string
  createdAt: string
  operation: string
  providerId: string | null
  configExisted: boolean
  authExisted: boolean
  providersExisted: boolean
  preferencesExisted: boolean
  appVersion: string
}

export type BackupCompatibility = 'current' | 'legacyWithoutPreferences'

export interface BackupSummary {
  directoryName: string
  metadata: BackupMetadata
  files: readonly BackupFileName[]
  compatibility: BackupCompatibility
}

export interface UnavailableBackup {
  directoryName: string
  code: string
  message: string
  canOpenMetadata: boolean
}

export interface BackupInventory {
  backups: readonly BackupSummary[]
  unavailableBackups: readonly UnavailableBackup[]
}
