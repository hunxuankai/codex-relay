import { getCurrentScope, onScopeDispose, readonly, shallowRef } from 'vue'
import * as relay from '../services/tauri'
import type { RelayUiError } from '../types/command'
import type {
  ProviderApiKeyDraft,
  ProviderApiKeyManagementState,
  ProviderApiKeyStatus,
  ProviderMutationOutcome,
  SaveProviderApiKeysInput,
} from '../types/provider'

export interface ProviderApiKeyManagerClient {
  getProviderApiKeysForManagement(providerId: string): Promise<ProviderApiKeyManagementState>
  saveProviderApiKeys(input: SaveProviderApiKeysInput): Promise<ProviderMutationOutcome>
}

export interface UseProviderApiKeyManagerOptions {
  client?: ProviderApiKeyManagerClient
  onSaved?: (outcome: ProviderMutationOutcome) => void | Promise<void>
}

const defaultClient: ProviderApiKeyManagerClient = {
  getProviderApiKeysForManagement: relay.getProviderApiKeysForManagement,
  saveProviderApiKeys: relay.saveProviderApiKeys,
}

export function useProviderApiKeyManager(options: UseProviderApiKeyManagerOptions = {}) {
  const client = options.client ?? defaultClient
  const providerId = shallowRef<string | null>(null)
  const entries = shallowRef<ProviderApiKeyDraft[]>([])
  const selectedApiKeyId = shallowRef<string | null>(null)
  const apiKeyStatus = shallowRef<ProviderApiKeyStatus | null>(null)
  const fingerprints = shallowRef<ProviderApiKeyManagementState['fingerprints'] | null>(null)
  const loading = shallowRef(false)
  const busy = shallowRef(false)
  const error = shallowRef<RelayUiError | null>(null)
  const successMessage = shallowRef<string | null>(null)
  let requestSequence = 0

  function setError(caught: unknown) {
    error.value = caught instanceof relay.RelayCommandError
      ? { code: caught.code, message: caught.message }
      : { code: 'UNEXPECTED_ERROR', message: 'API Key 操作失败，请重试。' }
  }

  function applyState(state: ProviderApiKeyManagementState) {
    providerId.value = state.providerId
    entries.value = state.entries.map((entry) => ({ ...entry }))
    selectedApiKeyId.value = state.selectedApiKeyId
    apiKeyStatus.value = state.apiKeyStatus
    fingerprints.value = state.fingerprints
  }

  function clear() {
    requestSequence += 1
    providerId.value = null
    entries.value = []
    selectedApiKeyId.value = null
    apiKeyStatus.value = null
    fingerprints.value = null
    loading.value = false
    busy.value = false
    error.value = null
    successMessage.value = null
  }

  async function load(targetProviderId: string): Promise<boolean> {
    if (busy.value) return false
    const sequence = ++requestSequence
    providerId.value = targetProviderId
    entries.value = []
    selectedApiKeyId.value = null
    apiKeyStatus.value = null
    fingerprints.value = null
    loading.value = true
    error.value = null
    successMessage.value = null
    try {
      const state = await client.getProviderApiKeysForManagement(targetProviderId)
      if (sequence !== requestSequence) return false
      applyState(state)
      return true
    } catch (caught) {
      if (sequence === requestSequence) setError(caught)
      return false
    } finally {
      if (sequence === requestSequence) loading.value = false
    }
  }

  function replaceEntries(nextEntries: readonly ProviderApiKeyDraft[]) {
    if (loading.value || busy.value) return
    entries.value = nextEntries.map((entry) => ({ ...entry }))
  }

  async function save(): Promise<ProviderMutationOutcome | undefined> {
    if (loading.value || busy.value || !providerId.value || !fingerprints.value) return undefined
    const sequence = ++requestSequence
    const targetProviderId = providerId.value
    const input: SaveProviderApiKeysInput = {
      providerId: targetProviderId,
      entries: entries.value.map((entry) => ({ ...entry })),
      expectedFiles: fingerprints.value,
    }
    busy.value = true
    error.value = null
    successMessage.value = null
    try {
      const outcome = await client.saveProviderApiKeys(input)
      if (sequence !== requestSequence) return undefined
      const refreshed = await client.getProviderApiKeysForManagement(targetProviderId)
      if (sequence !== requestSequence) return undefined
      applyState(refreshed)
      await options.onSaved?.(outcome)
      if (sequence !== requestSequence) return undefined
      successMessage.value = outcome.message
      return outcome
    } catch (caught) {
      if (sequence === requestSequence) setError(caught)
      return undefined
    } finally {
      if (sequence === requestSequence) busy.value = false
    }
  }

  if (getCurrentScope()) {
    onScopeDispose(clear)
  }

  return {
    providerId: readonly(providerId),
    entries: readonly(entries),
    selectedApiKeyId: readonly(selectedApiKeyId),
    apiKeyStatus: readonly(apiKeyStatus),
    loading: readonly(loading),
    busy: readonly(busy),
    error: readonly(error),
    successMessage: readonly(successMessage),
    load,
    replaceEntries,
    save,
    clear,
  }
}

export type ProviderApiKeyManager = ReturnType<typeof useProviderApiKeyManager>
