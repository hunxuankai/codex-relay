export interface FileFingerprint {
  exists: boolean
  len: number
  modifiedUnixMillis: number | null
  sha256: string | null
}

export interface FileSetFingerprint {
  config: FileFingerprint
  auth: FileFingerprint
  providers: FileFingerprint
}

export type WireApi = 'responses'

export interface ProviderProfile {
  id: string
  name: string
  baseUrl: string
  wireApi: WireApi
  model: string | null
  apiKeyConfigured: boolean
  isActive: boolean
  isValid: boolean
  validationMessage: string | null
}

export type ApiKeyChange =
  | { action: 'unchanged' }
  | { action: 'set'; value: string }
  | { action: 'clear' }

export interface CreateProviderInput {
  id: string
  name: string
  baseUrl: string
  wireApi: string
  model: string | null
  apiKey: string
  activateAfterSave: boolean
  expectedFiles: FileSetFingerprint
}

export interface UpdateProviderInput {
  id: string
  name: string
  baseUrl: string
  wireApi: string
  model: string | null
  apiKeyChange: ApiKeyChange
  syncIfActive: boolean
  expectedFiles: FileSetFingerprint
}

export interface ProviderListState {
  providers: ProviderProfile[]
  activeProviderId: string | null
  currentAuthImportAvailable: boolean
  fingerprints: FileSetFingerprint
}

export interface ProviderMutationOutcome {
  providers: ProviderProfile[]
  message: string
}

export interface SwitchOutcome {
  providers: ProviderProfile[]
  activeProviderId: string
  message: string
}
