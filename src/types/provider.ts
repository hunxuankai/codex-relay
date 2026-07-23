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
  preferences: FileFingerprint
}

export type WireApi = 'responses'

export interface ProviderBaseUrlSummary {
  id: string
  name: string
  url: string
}

export interface ProviderApiKeySummary {
  id: string
  name: string
}

export type ProviderBaseUrlStatus = 'managed' | 'external'
export type ProviderApiKeyStatus = 'managed' | 'external' | 'missing'

export interface ProviderProfile {
  id: string
  name: string
  baseUrl: string
  baseUrls: readonly ProviderBaseUrlSummary[]
  selectedBaseUrlId: string | null
  baseUrlStatus: ProviderBaseUrlStatus
  apiKeys: readonly ProviderApiKeySummary[]
  selectedApiKeyId: string | null
  apiKeyStatus: ProviderApiKeyStatus
  wireApi: WireApi
  models: readonly string[]
  selectedModel: string | null
  reasoningEfforts: Readonly<Record<string, string>>
  preferenceConfigured: boolean
  apiKeyConfigured: boolean
  configurationComplete: boolean
  disabledReason: string | null
  isActive: boolean
  isValid: boolean
  validationMessage: string | null
}

export interface CreateProviderInput {
  id: string
  name: string
  baseUrlName: string
  baseUrl: string
  wireApi: string
  models: string[]
  apiKeyName: string
  apiKey: string
  activateAfterSave: boolean
  expectedFiles: FileSetFingerprint
}

export interface UpdateProviderInput {
  id: string
  name: string
  wireApi: string
  models: string[]
  syncIfActive: boolean
  expectedFiles: FileSetFingerprint
}

export interface ProviderBaseUrlDraft {
  id: string | null
  name: string
  url: string
}

export interface SaveProviderBaseUrlsInput {
  providerId: string
  entries: ProviderBaseUrlDraft[]
  expectedFiles: FileSetFingerprint
}

export interface SelectProviderBaseUrlInput {
  providerId: string
  baseUrlId: string
  expectedFiles: FileSetFingerprint
}

export interface ProviderApiKeyManagementEntry {
  id: string
  name: string
  apiKey: string
}

export interface ProviderApiKeyManagementState {
  providerId: string
  entries: ProviderApiKeyManagementEntry[]
  selectedApiKeyId: string | null
  apiKeyStatus: ProviderApiKeyStatus
  fingerprints: FileSetFingerprint
}

export interface ProviderApiKeyDraft {
  id: string | null
  name: string
  apiKey: string
}

export interface SaveProviderApiKeysInput {
  providerId: string
  entries: ProviderApiKeyDraft[]
  expectedFiles: FileSetFingerprint
}

export interface SelectProviderApiKeyInput {
  providerId: string
  apiKeyId: string
  expectedFiles: FileSetFingerprint
}

export interface ImportCurrentApiKeyInput {
  providerId: string
  name: string
  expectedFiles: FileSetFingerprint
}

export interface ProviderListState {
  providers: ProviderProfile[]
  activeProviderId: string | null
  currentAuthImportAvailable: boolean
  fingerprints: FileSetFingerprint
  modelCatalog: ModelCatalogItem[]
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
