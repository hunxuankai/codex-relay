import type { ProviderApiKeySummary } from './provider'

export type ProfileProblem =
  | 'invalidConfiguration'
  | 'missingProvider'
  | 'missingEnvironmentVariable'
  | 'unsupportedAuthentication'

export type VariablePresence = 'set' | 'notDetected' | 'unavailable'

export type KeyRestriction =
  | 'sharedVariable'
  | 'systemVariable'
  | 'notDedicated'
  | 'unsupportedValue'
  | 'environmentUnavailable'

export interface ProfileKeyStatus {
  user: VariablePresence
  relayProcess: VariablePresence
  matchedUserKeyName: string | null
  restriction: KeyRestriction | null
  sharedWith: readonly string[]
}

export interface CliProfile {
  name: string
  command: string | null
  providerId: string | null
  host: string | null
  variable: string | null
  inheritsDefault: boolean
  problem: ProfileProblem | null
  keyStatus: ProfileKeyStatus | null
}

export interface CliProfileList {
  codexHome: string
  profiles: readonly CliProfile[]
  pendingWrite: boolean
}

export interface ProfileKeyEdit {
  profileName: string
  providerId: string
  host: string | null
  variable: string
  token: string
  replacesValue: boolean
  relayProcessHasValue: boolean
  providerKeys: readonly ProviderApiKeySummary[]
}

export interface SaveProfileKeyInput {
  profileName: string
  token: string
  newKey: string
}

export interface ApplyProfileProviderKeyInput {
  profileName: string
  token: string
  apiKeyId: string
}

export interface ProfileKeySaved {
  windowsNotified: boolean
}
