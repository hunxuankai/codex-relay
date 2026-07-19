import { computed, getCurrentScope, onScopeDispose, readonly, ref, shallowRef } from 'vue'
import * as relay from '../services/tauri'
import type { RelayUiError } from '../types/command'
import type {
  CreateProviderInput,
  FileSetFingerprint,
  ProviderListState,
  ProviderMutationOutcome,
  ProviderProfile,
  SwitchOutcome,
  UpdateProviderInput,
} from '../types/provider'

export interface ProviderClient {
  listProviders(): Promise<ProviderListState>
  createProvider(input: CreateProviderInput): Promise<ProviderMutationOutcome>
  updateProvider(input: UpdateProviderInput): Promise<ProviderMutationOutcome>
  deleteProvider(
    providerId: string,
    expectedFiles: FileSetFingerprint,
  ): Promise<ProviderMutationOutcome>
  switchProvider(providerId: string): Promise<SwitchOutcome>
  importCurrentAuthKey(providerId: string): Promise<ProviderMutationOutcome>
  onProvidersChanged(handler: (state: ProviderListState) => void): Promise<() => void>
}

export interface UseProvidersOptions {
  client?: ProviderClient
  subscribe?: boolean
}

const defaultClient: ProviderClient = {
  listProviders: relay.listProviders,
  createProvider: relay.createProvider,
  updateProvider: relay.updateProvider,
  deleteProvider: relay.deleteProvider,
  switchProvider: relay.switchProvider,
  importCurrentAuthKey: relay.importCurrentAuthKey,
  onProvidersChanged: relay.onProvidersChanged,
}

export function useProviders(options: UseProvidersOptions = {}) {
  const client = options.client ?? defaultClient
  const shouldSubscribe = options.subscribe ?? true
  const providerList = ref<ProviderProfile[]>([])
  const fingerprints = shallowRef<FileSetFingerprint | null>(null)
  const currentAuthImportAvailable = shallowRef(false)
  const selectedProviderId = shallowRef<string | null>(null)
  const loading = shallowRef(true)
  const busy = shallowRef(false)
  const error = shallowRef<RelayUiError | null>(null)
  const successMessage = shallowRef<string | null>(null)
  let stateSequence = 0

  const activeProvider = computed(
    () => providerList.value.find((provider) => provider.isActive) ?? null,
  )
  const selectedProvider = computed(
    () =>
      providerList.value.find((provider) => provider.id === selectedProviderId.value) ?? null,
  )

  function applyState(state: ProviderListState) {
    providerList.value = state.providers
    fingerprints.value = state.fingerprints
    currentAuthImportAvailable.value = state.currentAuthImportAvailable
    const selectionStillExists = state.providers.some(
      (provider) => provider.id === selectedProviderId.value,
    )
    if (!selectionStillExists) {
      selectedProviderId.value =
        state.activeProviderId ?? state.providers.find((provider) => provider.isActive)?.id ?? null
    }
  }

  function setError(caught: unknown) {
    if (caught instanceof relay.RelayCommandError) {
      error.value = { code: caught.code, message: caught.message }
    } else {
      error.value = { code: 'UNEXPECTED_ERROR', message: '操作失败，请重试。' }
    }
  }

  async function refresh(): Promise<boolean> {
    const requestSequence = ++stateSequence
    loading.value = true
    error.value = null
    try {
      const next = await client.listProviders()
      if (requestSequence === stateSequence) {
        applyState(next)
      }
      return true
    } catch (caught) {
      if (requestSequence === stateSequence) {
        setError(caught)
        return false
      }
      return true
    } finally {
      if (requestSequence === stateSequence) {
        loading.value = false
      }
    }
  }

  async function mutate<T extends { message: string }>(action: () => Promise<T>) {
    if (busy.value) return undefined
    busy.value = true
    error.value = null
    successMessage.value = null
    try {
      const outcome = await action()
      const refreshed = await refresh()
      if (refreshed) {
        successMessage.value = outcome.message
      }
      return outcome
    } catch (caught) {
      setError(caught)
      return undefined
    } finally {
      busy.value = false
    }
  }

  function create(input: CreateProviderInput) {
    return mutate(() => client.createProvider(input))
  }

  function update(input: UpdateProviderInput) {
    return mutate(() => client.updateProvider(input))
  }

  async function remove(providerId: string) {
    if (!fingerprints.value) {
      await refresh()
    }
    const expectedFiles = fingerprints.value
    if (!expectedFiles) return undefined
    return mutate(() => client.deleteProvider(providerId, expectedFiles))
  }

  function switchTo(providerId: string) {
    return mutate(() => client.switchProvider(providerId))
  }

  function importCurrentKey(providerId: string) {
    return mutate(() => client.importCurrentAuthKey(providerId))
  }

  function selectProvider(providerId: string | null) {
    selectedProviderId.value = providerId
  }

  let disposed = false
  let stopSubscription: (() => void) | undefined
  if (shouldSubscribe) {
    void client
      .onProvidersChanged((next) => {
        stateSequence += 1
        applyState(next)
        loading.value = false
      })
      .then((stop) => {
        if (disposed) stop()
        else stopSubscription = stop
      })
      .catch((caught: unknown) => {
        if (!disposed) setError(caught)
      })
  }
  if (getCurrentScope()) {
    onScopeDispose(() => {
      disposed = true
      stopSubscription?.()
    })
  }

  void refresh()

  return {
    providers: readonly(providerList),
    fingerprints: readonly(fingerprints),
    currentAuthImportAvailable: readonly(currentAuthImportAvailable),
    selectedProviderId: readonly(selectedProviderId),
    selectedProvider,
    activeProvider,
    loading: readonly(loading),
    busy: readonly(busy),
    error: readonly(error),
    successMessage: readonly(successMessage),
    refresh,
    create,
    update,
    remove,
    switchTo,
    importCurrentKey,
    selectProvider,
  }
}
