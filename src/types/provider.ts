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
  preferences?: FileFingerprint
}

export type WireApi = 'responses'

export interface ProviderProfile {
  id: string
  name: string
  baseUrl: string
  wireApi: WireApi
  models?: readonly string[]
  /** @deprecated legacy DTO compatibility; not persisted by Relay. */
  model?: string | null
  selectedModel?: string | null
  reasoningEfforts?: Record<string, string>
  preferenceConfigured?: boolean
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
  models?: string[]
  /** @deprecated ignored; use models. */
  model?: string | null
  apiKey: string
  activateAfterSave: boolean
  expectedFiles: FileSetFingerprint
}

export interface UpdateProviderInput {
  id: string
  name: string
  baseUrl: string
  wireApi: string
  models?: string[]
  /** @deprecated ignored; use models. */
  model?: string | null
  apiKeyChange: ApiKeyChange
  syncIfActive: boolean
  expectedFiles: FileSetFingerprint
}

export interface ProviderListState {
  providers: ProviderProfile[]
  activeProviderId: string | null
  currentAuthImportAvailable: boolean
  fingerprints: FileSetFingerprint
  modelCatalog?: ModelCatalogItem[]
}

export interface ModelCatalogItem {
  id: string
  reasoningEfforts: readonly string[]
  defaultReasoningEffort: string
}

export interface UpdateProviderPreferenceInput {
  providerId: string
  model: string
  reasoningEffort: string
  expectedFiles: FileSetFingerprint
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
