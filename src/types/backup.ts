export type BackupFileName =
  | 'config.toml'
  | 'auth.json'
  | 'providers.json'
  | 'provider-preferences.json'
  | 'metadata.json'

export interface BackupMetadata {
  transactionId: string
  createdAt: string
  operation: string
  providerId: string | null
  configExisted: boolean
  authExisted: boolean
  providersExisted: boolean
  preferencesExisted?: boolean
  appVersion: string
}

export interface BackupSummary {
  directoryName: string
  metadata: BackupMetadata
  files: readonly BackupFileName[]
}
